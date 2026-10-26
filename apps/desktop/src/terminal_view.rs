use std::time::Duration;

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};
use termzip_core::{Rgb, Theme};
use termzip_terminal::TerminalSnapshot;

pub struct TerminalViewport {
    pub response: egui::Response,
    pub rows: u16,
    pub columns: u16,
    pub cell_width: f32,
    pub cell_height: f32,
}

pub fn show_terminal(
    ui: &mut egui::Ui,
    snapshot: &TerminalSnapshot,
    theme: Theme,
    font_size: f32,
    cursor_visible: bool,
) -> TerminalViewport {
    let available = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(available, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 10.0, color(theme.surface));

    let font = FontId::monospace(font_size);
    let cell_width = ui.fonts_mut(|fonts| {
        fonts
            .layout_no_wrap("M".to_owned(), font.clone(), color(theme.foreground))
            .size()
            .x
    });
    let cell_width = cell_width.max(font_size * 0.52);
    let cell_height = font_size * 1.52;
    let columns = (rect.width() / cell_width).floor().max(2.0) as u16;
    let rows = (rect.height() / cell_height).floor().max(1.0) as u16;
    let top = rect.top() + 12.0;
    let left = rect.left() + 16.0;
    let visible_rows = usize::from(rows).min(snapshot.rows);
    let visible_columns = usize::from(columns).min(snapshot.columns);

    for row in 0..visible_rows {
        let mut column = 0;
        while column < visible_columns {
            let Some(cell) = snapshot.cell(row, column) else {
                break;
            };
            let run_start = column;
            let run_style = cell.style;
            let mut text = String::new();
            while column < visible_columns {
                let Some(next) = snapshot.cell(row, column) else {
                    break;
                };
                if next.style != run_style {
                    break;
                }
                text.push(if next.style.hidden {
                    ' '
                } else {
                    next.character
                });
                column += 1;
            }

            let position = Pos2::new(
                left + run_start as f32 * cell_width,
                top + row as f32 * cell_height,
            );
            let foreground = if run_style.bold {
                brighten(run_style.foreground)
            } else {
                color(run_style.foreground)
            };
            if run_style.background != theme.surface {
                painter.rect_filled(
                    Rect::from_min_size(
                        position,
                        Vec2::new((column - run_start) as f32 * cell_width, cell_height),
                    ),
                    0.0,
                    color(run_style.background),
                );
            }
            painter.text(position, Align2::LEFT_TOP, text, font.clone(), foreground);

            if run_style.underline || run_style.strikethrough {
                let y = if run_style.underline {
                    position.y + cell_height - 2.0
                } else {
                    position.y + cell_height * 0.55
                };
                painter.line_segment(
                    [
                        Pos2::new(position.x, y),
                        Pos2::new(left + column as f32 * cell_width, y),
                    ],
                    Stroke::new(1.0, foreground),
                );
            }
        }
    }

    if cursor_visible
        && let Some((row, column)) = snapshot.cursor
        && row < visible_rows
        && column < visible_columns
    {
        let min = Pos2::new(
            left + column as f32 * cell_width,
            top + row as f32 * cell_height,
        );
        let cursor_rect = Rect::from_min_size(min, Vec2::new(cell_width, cell_height));
        painter.rect_filled(cursor_rect, 2.0, color(theme.accent).linear_multiply(0.82));
        if let Some(cell) = snapshot.cell(row, column) {
            painter.text(
                min,
                Align2::LEFT_TOP,
                cell.character,
                font,
                color(theme.background),
            );
        }
    }

    ui.ctx().request_repaint_after(Duration::from_millis(420));

    TerminalViewport {
        response,
        rows,
        columns,
        cell_width,
        cell_height,
    }
}

fn color(rgb: Rgb) -> Color32 {
    Color32::from_rgb(rgb.r, rgb.g, rgb.b)
}

fn brighten(rgb: Rgb) -> Color32 {
    Color32::from_rgb(
        rgb.r.saturating_add(12),
        rgb.g.saturating_add(12),
        rgb.b.saturating_add(12),
    )
}
