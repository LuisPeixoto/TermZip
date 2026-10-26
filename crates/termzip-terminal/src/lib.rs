use alacritty_terminal::event::VoidListener;
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{self, Color, NamedColor, Rgb as AnsiRgb};
use termzip_core::{Rgb, Theme};

#[derive(Debug, Clone, Copy)]
struct TerminalSize {
    rows: usize,
    columns: usize,
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalStyle {
    pub foreground: Rgb,
    pub background: Rgb,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCell {
    pub character: char,
    pub style: TerminalStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub rows: usize,
    pub columns: usize,
    pub cells: Vec<TerminalCell>,
    pub cursor: Option<(usize, usize)>,
    pub scrollback_offset: usize,
    pub alternate_screen: bool,
    pub mouse_reporting: bool,
}

impl TerminalSnapshot {
    pub fn cell(&self, row: usize, column: usize) -> Option<&TerminalCell> {
        if row >= self.rows || column >= self.columns {
            return None;
        }

        self.cells.get(row * self.columns + column)
    }
}

pub struct TerminalGrid {
    term: Term<VoidListener>,
    parser: ansi::Processor,
}

impl TerminalGrid {
    pub fn new(rows: usize, columns: usize, scrollback_lines: usize) -> Self {
        let size = TerminalSize {
            rows: rows.max(1),
            columns: columns.max(1),
        };
        let config = Config {
            scrolling_history: scrollback_lines,
            ..Config::default()
        };

        Self {
            term: Term::new(config, &size, VoidListener),
            parser: ansi::Processor::new(),
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    pub fn resize(&mut self, rows: usize, columns: usize) {
        let size = TerminalSize {
            rows: rows.max(1),
            columns: columns.max(1),
        };
        self.term.resize(size);
    }

    pub fn scroll(&mut self, lines: i32) {
        self.term.scroll_display(Scroll::Delta(lines));
    }

    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    pub fn snapshot(&self, theme: Theme) -> TerminalSnapshot {
        let rows = self.term.screen_lines();
        let columns = self.term.columns();
        let content = self.term.renderable_content();
        let scrollback_offset = content.display_offset;
        let mut cells = vec![blank_cell(theme); rows.saturating_mul(columns)];

        for indexed in content.display_iter {
            let row = indexed.point.line.0 + scrollback_offset as i32;
            let column = indexed.point.column.0;
            if row < 0 || column >= columns || row as usize >= rows {
                continue;
            }

            let cell = indexed.cell;
            let mut foreground = resolve_color(cell.fg, theme, content.colors);
            let mut background = resolve_color(cell.bg, theme, content.colors);
            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut foreground, &mut background);
            }

            let dim = cell.flags.contains(Flags::DIM);
            if dim {
                foreground = Rgb::new(
                    (foreground.r as u16 / 2) as u8,
                    (foreground.g as u16 / 2) as u8,
                    (foreground.b as u16 / 2) as u8,
                );
            }

            cells[row as usize * columns + column] = TerminalCell {
                character: if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    ' '
                } else {
                    cell.c
                },
                style: TerminalStyle {
                    foreground,
                    background,
                    bold: cell.flags.contains(Flags::BOLD),
                    italic: cell.flags.contains(Flags::ITALIC),
                    underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
                    strikethrough: cell.flags.contains(Flags::STRIKEOUT),
                    hidden: cell.flags.contains(Flags::HIDDEN),
                },
            };
        }

        let cursor = if scrollback_offset == 0 {
            let point = content.cursor.point;
            let row = point.line.0;
            let column = point.column.0;
            (row >= 0 && (row as usize) < rows && column < columns)
                .then_some((row as usize, column))
        } else {
            None
        };

        TerminalSnapshot {
            rows,
            columns,
            cells,
            cursor,
            scrollback_offset,
            alternate_screen: content
                .mode
                .contains(alacritty_terminal::term::TermMode::ALT_SCREEN),
            mouse_reporting: content
                .mode
                .intersects(alacritty_terminal::term::TermMode::MOUSE_MODE),
        }
    }

    pub fn plain_text(&self) -> Vec<String> {
        let grid = self.term.grid();
        let history = grid.history_size();
        let rows = grid.total_lines();
        let columns = grid.columns();
        let first_line = -(history as i32);
        let mut output = Vec::with_capacity(rows);

        for row in 0..rows {
            let line = Line(first_line + row as i32);
            let mut text = String::with_capacity(columns);
            for column in 0..columns {
                let cell: &Cell = &grid[line][Column(column)];
                if !cell.flags.contains(Flags::WIDE_CHAR_SPACER)
                    && !cell.flags.contains(Flags::HIDDEN)
                {
                    text.push(cell.c);
                    if let Some(combining) = cell.zerowidth() {
                        text.extend(combining);
                    }
                }
            }
            output.push(text.trim_end().to_owned());
        }

        output
    }
}

fn blank_cell(theme: Theme) -> TerminalCell {
    TerminalCell {
        character: ' ',
        style: TerminalStyle {
            foreground: theme.foreground,
            background: theme.surface,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            hidden: false,
        },
    }
}

fn resolve_color(
    color: Color,
    theme: Theme,
    colors: &alacritty_terminal::term::color::Colors,
) -> Rgb {
    match color {
        Color::Spec(rgb) => from_ansi_rgb(rgb),
        Color::Indexed(index) => colors[index as usize]
            .map(from_ansi_rgb)
            .unwrap_or_else(|| indexed_color(index, theme)),
        Color::Named(named) => colors[named as usize]
            .map(from_ansi_rgb)
            .unwrap_or_else(|| named_color(named, theme)),
    }
}

fn from_ansi_rgb(rgb: AnsiRgb) -> Rgb {
    Rgb::new(rgb.r, rgb.g, rgb.b)
}

fn named_color(named: NamedColor, theme: Theme) -> Rgb {
    match named {
        NamedColor::Foreground | NamedColor::BrightForeground | NamedColor::DimForeground => {
            theme.foreground
        }
        NamedColor::Background => theme.surface,
        NamedColor::Cursor => theme.accent,
        NamedColor::Black => theme.ansi[0],
        NamedColor::Red => theme.ansi[1],
        NamedColor::Green => theme.ansi[2],
        NamedColor::Yellow => theme.ansi[3],
        NamedColor::Blue => theme.ansi[4],
        NamedColor::Magenta => theme.ansi[5],
        NamedColor::Cyan => theme.ansi[6],
        NamedColor::White => theme.ansi[7],
        NamedColor::BrightBlack => theme.ansi[8],
        NamedColor::BrightRed => theme.ansi[9],
        NamedColor::BrightGreen => theme.ansi[10],
        NamedColor::BrightYellow => theme.ansi[11],
        NamedColor::BrightBlue => theme.ansi[12],
        NamedColor::BrightMagenta => theme.ansi[13],
        NamedColor::BrightCyan => theme.ansi[14],
        NamedColor::BrightWhite => theme.ansi[15],
        NamedColor::DimBlack => dim(theme.ansi[0]),
        NamedColor::DimRed => dim(theme.ansi[1]),
        NamedColor::DimGreen => dim(theme.ansi[2]),
        NamedColor::DimYellow => dim(theme.ansi[3]),
        NamedColor::DimBlue => dim(theme.ansi[4]),
        NamedColor::DimMagenta => dim(theme.ansi[5]),
        NamedColor::DimCyan => dim(theme.ansi[6]),
        NamedColor::DimWhite => dim(theme.ansi[7]),
    }
}

fn dim(color: Rgb) -> Rgb {
    Rgb::new(color.r / 2, color.g / 2, color.b / 2)
}

fn indexed_color(index: u8, theme: Theme) -> Rgb {
    match index {
        0..=15 => theme.ansi[index as usize],
        16..=231 => {
            let color = index - 16;
            let component = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
            Rgb::new(
                component(color / 36),
                component((color % 36) / 6),
                component(color % 6),
            )
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            Rgb::new(gray, gray, gray)
        }
    }
}

#[cfg(test)]
mod tests {
    use termzip_core::ThemeKind;

    use super::TerminalGrid;

    #[test]
    fn ansi_text_and_truecolor_are_rendered() {
        let mut terminal = TerminalGrid::new(4, 12, 100);
        terminal.feed(b"\x1b[38;2;120;130;140mHello\x1b[0m");

        let snapshot = terminal.snapshot(ThemeKind::Dark.palette());

        assert_eq!(snapshot.cell(0, 0).unwrap().character, 'H');
        assert_eq!(
            snapshot.cell(0, 0).unwrap().style.foreground,
            termzip_core::Rgb::new(120, 130, 140)
        );
        assert_eq!(snapshot.cell(0, 4).unwrap().character, 'o');
    }

    #[test]
    fn alternate_screen_and_cursor_visibility_are_tracked() {
        let mut terminal = TerminalGrid::new(4, 12, 100);
        terminal.feed(b"\x1b[?1049h\x1b[2;3Hhello");

        let snapshot = terminal.snapshot(ThemeKind::Dark.palette());

        assert!(snapshot.alternate_screen);
        assert_eq!(snapshot.cell(1, 2).unwrap().character, 'h');
        assert_eq!(snapshot.cursor, Some((1, 7)));
    }

    #[test]
    fn resizing_updates_the_emulated_screen() {
        let mut terminal = TerminalGrid::new(4, 12, 100);
        terminal.resize(7, 20);

        let snapshot = terminal.snapshot(ThemeKind::Dark.palette());

        assert_eq!((snapshot.rows, snapshot.columns), (7, 20));
        assert_eq!(snapshot.cells.len(), 140);
    }

    #[test]
    fn scrollback_keeps_plain_text_lines() {
        let mut terminal = TerminalGrid::new(2, 20, 10);
        terminal.feed(b"first\r\nsecond\r\nthird");

        let lines = terminal.plain_text();

        assert!(lines.iter().any(|line| line == "first"));
        assert!(lines.iter().any(|line| line == "third"));
    }
}
