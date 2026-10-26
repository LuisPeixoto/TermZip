use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Id, Layout, Margin, Pos2, Rect, RichText, Sense,
    Stroke, Vec2,
};
use termzip_core::{Rgb, Settings, Theme, ThemeKind};
use termzip_pty::{PtyOptions, PtySession};
use termzip_terminal::{CellSelection, TerminalGrid, TerminalSnapshot};

use crate::terminal_view::{self, TerminalViewport};

const DEFAULT_ROWS: usize = 24;
const DEFAULT_COLUMNS: usize = 100;
const COMMAND_HISTORY_LIMIT: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Composer,
    Terminal,
}

pub struct TermZipApp {
    settings: Settings,
    terminal: TerminalGrid,
    session: Option<PtySession>,
    output: Option<Receiver<Vec<u8>>>,
    command: String,
    command_history: VecDeque<String>,
    history_selection: Option<usize>,
    selection: Option<CellSelection>,
    input_mode: InputMode,
    settings_open: bool,
    terminal_focus: bool,
    status_message: Option<String>,
    child_exited: bool,
    last_grid_size: (u16, u16),
    last_output_at: f64,
    cursor_phase: bool,
}

impl TermZipApp {
    pub fn new(context: &eframe::CreationContext<'_>) -> Self {
        let settings = Settings::load().unwrap_or_default();
        context.egui_ctx.set_visuals(visuals(settings.theme));

        let mut app = Self {
            terminal: TerminalGrid::new(DEFAULT_ROWS, DEFAULT_COLUMNS, settings.scrollback_lines),
            settings,
            session: None,
            output: None,
            command: String::new(),
            command_history: VecDeque::new(),
            history_selection: None,
            selection: None,
            input_mode: InputMode::Composer,
            settings_open: false,
            terminal_focus: false,
            status_message: None,
            child_exited: false,
            last_grid_size: (0, 0),
            last_output_at: 0.0,
            cursor_phase: true,
        };

        let repaint = context.egui_ctx.clone();
        let options = PtyOptions {
            shell: app.settings.shell.as_deref().map(PathBuf::from),
            working_directory: app.settings.working_directory.as_deref().map(PathBuf::from),
            rows: DEFAULT_ROWS as u16,
            columns: DEFAULT_COLUMNS as u16,
            ..PtyOptions::default()
        };

        match PtySession::spawn(options, move || repaint.request_repaint()) {
            Ok((session, output)) => {
                app.session = Some(session);
                app.output = Some(output);
            }
            Err(error) => {
                app.status_message = Some(format!("Could not start the shell: {error:#}"))
            }
        }

        app
    }

    fn theme(&self) -> Theme {
        self.settings.theme.palette()
    }

    fn process_output(&mut self, context: &egui::Context) {
        if let Some(receiver) = &self.output {
            while let Ok(bytes) = receiver.try_recv() {
                self.terminal.feed(&bytes);
                self.last_output_at = context.input(|input| input.time);
            }
        }

        let responses = self.terminal.drain_pty_responses();
        if let Some(session) = &self.session {
            for response in responses {
                if let Err(error) = session.write(response.as_bytes()) {
                    self.status_message =
                        Some(format!("Could not answer a terminal query: {error:#}"));
                    break;
                }
            }
        }

        if !self.child_exited
            && let Some(session) = self.session.as_mut()
            && let Ok(Some(status)) = session.try_wait()
        {
            self.child_exited = true;
            self.status_message = Some(format!("Shell exited with status {}", status.exit_code()));
        }
    }

    fn submit_command(&mut self) {
        let command = self.command.trim().to_owned();
        if command.is_empty() {
            return;
        }

        if let Some(session) = &self.session {
            let mut input = command.as_bytes().to_vec();
            input.push(b'\r');
            match session.write(&input) {
                Ok(()) => {
                    if self.command_history.back() != Some(&command) {
                        self.command_history.push_back(command);
                    }
                    while self.command_history.len() > COMMAND_HISTORY_LIMIT {
                        self.command_history.pop_front();
                    }
                    self.command.clear();
                    self.history_selection = None;
                    self.status_message = None;
                }
                Err(error) => {
                    self.status_message = Some(format!("Could not send command: {error:#}"))
                }
            }
        } else {
            self.status_message =
                Some("The shell is not available. Restart TermZip to try again.".to_owned());
        }
    }

    fn write_input(&mut self, bytes: &[u8]) {
        if let Some(session) = &self.session
            && let Err(error) = session.write(bytes)
        {
            self.status_message = Some(format!("Could not write to the terminal: {error:#}"));
        }
    }

    fn resize_terminal(&mut self, viewport: &TerminalViewport) {
        let size = (viewport.rows.max(1), viewport.columns.max(1));
        if size == self.last_grid_size {
            return;
        }

        self.last_grid_size = size;
        self.terminal.resize(size.0.into(), size.1.into());
        if let Some(session) = &self.session {
            let pixel_width = (viewport.cell_width * f32::from(size.1))
                .round()
                .clamp(0.0, 65_535.0) as u16;
            let pixel_height = (viewport.cell_height * f32::from(size.0))
                .round()
                .clamp(0.0, 65_535.0) as u16;
            if let Err(error) = session.resize(size.0, size.1, pixel_width, pixel_height) {
                self.status_message = Some(format!("Could not resize the terminal: {error:#}"));
            }
        }
    }

    fn show_top_bar(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme();
        ui.horizontal(|ui| {
            let mark_rect = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover()).0;
            draw_mark(ui.painter(), mark_rect, theme);

            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.add_space(1.0);
                ui.label(
                    RichText::new("TermZip")
                        .size(15.0)
                        .strong()
                        .color(color(theme.foreground)),
                );
                ui.label(
                    RichText::new("A calmer command line")
                        .size(10.5)
                        .color(color(theme.muted)),
                );
            });

            ui.add_space(24.0);
            ui.separator();
            ui.add_space(12.0);
            ui.label(
                RichText::new(current_directory(&self.settings))
                    .size(12.0)
                    .color(color(theme.muted)),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let theme_label = match self.settings.theme {
                    ThemeKind::Dark => "Light theme",
                    ThemeKind::Light => "Dark theme",
                };
                if subtle_button(ui, theme_label, theme).clicked() {
                    self.settings.theme = self.settings.theme.toggle();
                    if let Err(error) = self.settings.save() {
                        self.status_message = Some(format!("Could not save settings: {error}"));
                    }
                }
                if subtle_button(
                    ui,
                    if self.settings_open {
                        "Close settings"
                    } else {
                        "Preferences"
                    },
                    theme,
                )
                .clicked()
                {
                    self.settings_open = !self.settings_open;
                }
            });
        });
    }

    fn show_terminal(
        &mut self,
        ui: &mut egui::Ui,
        context: &egui::Context,
        snapshot: &TerminalSnapshot,
    ) {
        let theme = self.theme();
        let panel = Frame::new()
            .fill(color(theme.surface))
            .stroke(Stroke::new(1.0, color(theme.border)))
            .corner_radius(12.0)
            .inner_margin(Margin::same(12));

        let inner = panel.show(ui, |ui| {
            terminal_view::show_terminal(
                ui,
                snapshot,
                self.selection,
                theme,
                self.settings.font_size,
                self.cursor_phase && !self.child_exited,
            )
        });

        let viewport = inner.inner;
        if viewport.response.clicked() {
            self.terminal_focus = self.input_mode == InputMode::Terminal;
            if !snapshot.mouse_reporting {
                self.selection = None;
            }
            context.memory_mut(|memory| memory.surrender_focus(Id::unique("command-composer")));
        }

        if viewport.response.drag_started() {
            context.memory_mut(|memory| memory.surrender_focus(Id::unique("command-composer")));
        }

        if (viewport.response.drag_started() || viewport.response.dragged())
            && !snapshot.mouse_reporting
            && let Some(point) = context
                .input(|input| input.pointer.interact_pos())
                .and_then(|position| viewport.cell_at(position))
        {
            if viewport.response.drag_started() || self.selection.is_none() {
                self.selection = Some(CellSelection {
                    anchor: point,
                    focus: point,
                });
            } else if let Some(selection) = self.selection.as_mut() {
                selection.focus = point;
            }
        }
        self.resize_terminal(&viewport);

        let scroll_delta = context.input(|input| input.smooth_scroll_delta.y);
        if inner
            .response
            .rect
            .contains(context.input(|input| input.pointer.hover_pos().unwrap_or(Pos2::ZERO)))
            && scroll_delta.abs() > 0.0
            && !snapshot.mouse_reporting
        {
            self.terminal.scroll((scroll_delta / 32.0).round() as i32);
            context.request_repaint();
        }
    }

    fn show_composer(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme();
        let composer_id = Id::unique("command-composer");
        Frame::new()
            .fill(color(theme.surface))
            .stroke(Stroke::new(1.0, if self.input_mode == InputMode::Composer { color(theme.accent).linear_multiply(0.65) } else { color(theme.border) }))
            .corner_radius(13.0)
            .inner_margin(Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let prompt_rect = ui.allocate_exact_size(Vec2::new(30.0, 34.0), Sense::hover()).0;
                    let center = prompt_rect.center();
                    ui.painter().circle_filled(center, 11.0, color(theme.accent_soft));
                    ui.painter().text(center, Align2::CENTER_CENTER, "›", FontId::monospace(20.0), color(theme.accent));

                    ui.vertical(|ui| {
                        ui.add_space(1.0);
                        match self.input_mode {
                            InputMode::Composer => {
                                let input = egui::TextEdit::singleline(&mut self.command)
                                    .id(composer_id)
                                    .font(FontId::monospace(self.settings.font_size))
                                    .hint_text("Type a command…")
                                    .desired_width(f32::INFINITY)
                                    .frame(Frame::NONE);
                                let response = ui.add(input);
                                self.terminal_focus = false;
                                if response.has_focus() {
                                    self.handle_composer_history(ui);
                                    if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                                        self.submit_command();
                                    }
                                }
                            }
                            InputMode::Terminal => {
                                ui.add_space(2.0);
                                ui.label(RichText::new("Terminal focused · keys are sent to the running process").size(12.0).color(color(theme.muted)));
                            }
                        }
                        ui.label(RichText::new(match self.input_mode {
                            InputMode::Composer => "Commands run in your persistent shell",
                            InputMode::Terminal => "Click the terminal area to focus it · use the status control to return",
                        }).size(10.0).color(color(theme.muted)));
                    });

                    if self.input_mode == InputMode::Composer
                        && accent_button(ui, "Run", theme).clicked()
                    {
                        self.submit_command();
                    }
                });
            });
    }

    fn handle_composer_history(&mut self, ui: &egui::Ui) {
        if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
            let next = self
                .history_selection
                .map_or(self.command_history.len().saturating_sub(1), |index| {
                    index.saturating_sub(1)
                });
            if let Some(command) = self.command_history.get(next) {
                self.command.clone_from(command);
                self.history_selection = Some(next);
            }
        }
        if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
            let Some(index) = self.history_selection else {
                return;
            };
            let next = index + 1;
            if let Some(command) = self.command_history.get(next) {
                self.command.clone_from(command);
                self.history_selection = Some(next);
            } else {
                self.command.clear();
                self.history_selection = None;
            }
        }
    }

    fn show_status_bar(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("●")
                    .size(9.0)
                    .color(color(if self.child_exited {
                        theme.warning
                    } else {
                        theme.success
                    })),
            );
            let shell_status = if self.child_exited {
                "Shell exited".to_owned()
            } else {
                shell_name(&self.settings)
            };
            ui.label(
                RichText::new(shell_status)
                    .size(11.0)
                    .color(color(theme.muted)),
            );

            if let Some(message) = &self.status_message {
                ui.separator();
                ui.label(
                    RichText::new(message)
                        .size(11.0)
                        .color(color(theme.warning)),
                );
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if subtle_button(
                    ui,
                    if self.input_mode == InputMode::Composer {
                        "Terminal mode"
                    } else {
                        "Composer mode"
                    },
                    theme,
                )
                .clicked()
                {
                    self.input_mode = match self.input_mode {
                        InputMode::Composer => InputMode::Terminal,
                        InputMode::Terminal => InputMode::Composer,
                    };
                    self.terminal_focus = self.input_mode == InputMode::Terminal;
                }
                ui.label(
                    RichText::new(format!(
                        "{} × {}",
                        self.last_grid_size.1, self.last_grid_size.0
                    ))
                    .size(10.0)
                    .color(color(theme.muted)),
                );
            });
        });
    }

    fn show_settings(&mut self, context: &egui::Context) {
        if !self.settings_open {
            return;
        }

        let theme = self.theme();
        egui::Window::new("Preferences")
            .id(Id::unique("preferences"))
            .collapsible(false)
            .resizable(false)
            .default_width(300.0)
            .anchor(Align2::RIGHT_TOP, Vec2::new(-20.0, 64.0))
            .show(context, |ui| {
                ui.label(RichText::new("Appearance").size(13.0).strong());
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for (kind, label) in [(ThemeKind::Dark, "Dark"), (ThemeKind::Light, "Light")] {
                        let selected = self.settings.theme == kind;
                        if ui.selectable_label(selected, label).clicked() {
                            self.settings.theme = kind;
                            if let Err(error) = self.settings.save() {
                                self.status_message =
                                    Some(format!("Could not save settings: {error}"));
                            }
                        }
                    }
                });

                ui.add_space(12.0);
                ui.label(RichText::new("Terminal font size").size(13.0).strong());
                let changed = ui
                    .add(egui::Slider::new(&mut self.settings.font_size, 12.0..=22.0).suffix(" px"))
                    .changed();
                if changed && let Err(error) = self.settings.save() {
                    self.status_message = Some(format!("Could not save settings: {error}"));
                }

                ui.add_space(12.0);
                ui.label(RichText::new("Shell").size(13.0).strong());
                ui.label(
                    RichText::new(shell_name(&self.settings))
                        .size(12.0)
                        .color(color(theme.muted)),
                );
                ui.label(
                    RichText::new("Set `shell` in config.toml to choose another shell.")
                        .size(11.0)
                        .color(color(theme.muted)),
                );

                ui.add_space(12.0);
                ui.label(RichText::new("Scrollback").size(13.0).strong());
                ui.label(
                    RichText::new(format!("{} lines", self.settings.scrollback_lines))
                        .size(12.0)
                        .color(color(theme.muted)),
                );
            });
    }

    fn handle_terminal_input(&mut self, context: &egui::Context) {
        if self.input_mode != InputMode::Terminal || !self.terminal_focus {
            return;
        }

        let events = context.input(|input| input.events.clone());
        for event in events {
            match event {
                egui::Event::Text(text) => self.write_input(text.as_bytes()),
                egui::Event::Paste(text) => {
                    let snapshot = self.terminal.snapshot(self.theme());
                    if snapshot.bracketed_paste {
                        let mut paste = String::with_capacity(text.len() + 24);
                        paste.push_str("\x1b[200~");
                        paste.push_str(&text);
                        paste.push_str("\x1b[201~");
                        self.write_input(paste.as_bytes());
                    } else {
                        self.write_input(text.as_bytes());
                    }
                }
                egui::Event::Copy => self.copy_selection(context),
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if key == egui::Key::C
                        && clipboard_shortcut(modifiers)
                        && self.selection.is_some()
                    {
                        self.copy_selection(context);
                        continue;
                    }
                    if let Some(bytes) = key_bytes(key, modifiers) {
                        self.write_input(&bytes);
                    }
                }
                _ => {}
            }
        }
    }

    fn copy_selection(&mut self, context: &egui::Context) {
        if let Some(text) = self.selection.and_then(|selection| {
            self.terminal
                .snapshot(self.theme())
                .selected_text(selection)
        }) {
            context.copy_text(text);
            self.status_message = Some("Selection copied".to_owned());
        }
    }

    fn handle_composer_copy(&mut self, context: &egui::Context) {
        if self.input_mode != InputMode::Composer
            || context.memory(|memory| memory.has_focus(Id::unique("command-composer")))
            || self.selection.is_none()
        {
            return;
        }

        let events = context.input(|input| input.events.clone());
        for event in events {
            match event {
                egui::Event::Copy => self.copy_selection(context),
                egui::Event::Key {
                    key: egui::Key::C,
                    pressed: true,
                    modifiers,
                    ..
                } if clipboard_shortcut(modifiers) => self.copy_selection(context),
                _ => {}
            }
        }
    }
}

impl eframe::App for TermZipApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        self.process_output(&context);
        context.set_visuals(visuals(self.settings.theme));

        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(color(self.theme().background))
                    .inner_margin(Margin::same(0)),
            )
            .show(ui, |ui| {
                let theme = self.theme();
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    ui.vertical(|ui| {
                        self.show_top_bar(ui);
                        ui.add_space(14.0);

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("TERMINAL")
                                    .size(10.0)
                                    .strong()
                                    .color(color(theme.muted)),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("⌘  Local shell")
                                    .size(11.0)
                                    .color(color(theme.foreground)),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("PREVIEW")
                                    .size(9.0)
                                    .strong()
                                    .color(color(theme.accent)),
                            );
                        });
                        ui.add_space(8.0);

                        let snapshot = self.terminal.snapshot(theme);
                        self.show_terminal(ui, &context, &snapshot);
                        ui.add_space(10.0);
                        self.show_composer(ui);
                        ui.add_space(8.0);
                        self.show_status_bar(ui);
                    });
                    ui.add_space(18.0);
                });
                ui.add_space(14.0);
            });

        self.show_settings(&context);
        self.handle_terminal_input(&context);
        self.handle_composer_copy(&context);

        if context.input(|input| input.time - self.last_output_at > 0.45) {
            self.cursor_phase = !self.cursor_phase;
            self.last_output_at = context.input(|input| input.time);
            context.request_repaint_after(Duration::from_millis(450));
        }
    }
}

fn key_bytes(key: egui::Key, modifiers: egui::Modifiers) -> Option<Vec<u8>> {
    if modifiers.ctrl {
        let control = match key {
            egui::Key::A => 1,
            egui::Key::B => 2,
            egui::Key::C => 3,
            egui::Key::D => 4,
            egui::Key::E => 5,
            egui::Key::F => 6,
            egui::Key::G => 7,
            egui::Key::H => 8,
            egui::Key::I => 9,
            egui::Key::J => 10,
            egui::Key::K => 11,
            egui::Key::L => 12,
            egui::Key::M => 13,
            egui::Key::N => 14,
            egui::Key::O => 15,
            egui::Key::P => 16,
            egui::Key::Q => 17,
            egui::Key::R => 18,
            egui::Key::S => 19,
            egui::Key::T => 20,
            egui::Key::U => 21,
            egui::Key::V => 22,
            egui::Key::W => 23,
            egui::Key::X => 24,
            egui::Key::Y => 25,
            egui::Key::Z => 26,
            _ => return None,
        };
        return Some(vec![control]);
    }

    let sequence: &[u8] = match key {
        egui::Key::Enter => b"\r",
        egui::Key::Tab => b"\t",
        egui::Key::Backspace => b"\x7f",
        egui::Key::Escape => b"\x1b",
        egui::Key::ArrowUp => b"\x1b[A",
        egui::Key::ArrowDown => b"\x1b[B",
        egui::Key::ArrowRight => b"\x1b[C",
        egui::Key::ArrowLeft => b"\x1b[D",
        egui::Key::Home => b"\x1b[H",
        egui::Key::End => b"\x1b[F",
        egui::Key::Delete => b"\x1b[3~",
        egui::Key::PageUp => b"\x1b[5~",
        egui::Key::PageDown => b"\x1b[6~",
        _ => return None,
    };
    Some(sequence.to_vec())
}

fn clipboard_shortcut(modifiers: egui::Modifiers) -> bool {
    modifiers.mac_cmd || (modifiers.ctrl && modifiers.shift)
}

fn current_directory(settings: &Settings) -> String {
    settings
        .working_directory
        .as_deref()
        .map(str::to_owned)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|path| path.display().to_string())
        })
        .unwrap_or_else(|| "Local shell".to_owned())
}

fn shell_name(settings: &Settings) -> String {
    let shell = settings
        .shell
        .clone()
        .or_else(|| std::env::var("SHELL").ok());
    shell
        .as_deref()
        .and_then(|path| {
            PathBuf::from(path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| {
            if cfg!(windows) {
                "cmd".to_owned()
            } else {
                "sh".to_owned()
            }
        })
}

fn visuals(theme: ThemeKind) -> egui::Visuals {
    let palette = theme.palette();
    let mut visuals = match theme {
        ThemeKind::Dark => egui::Visuals::dark(),
        ThemeKind::Light => egui::Visuals::light(),
    };
    visuals.panel_fill = color(palette.background);
    visuals.window_fill = color(palette.surface);
    visuals.extreme_bg_color = color(palette.elevated);
    visuals.faint_bg_color = color(palette.surface);
    visuals.override_text_color = Some(color(palette.foreground));
    visuals.selection.bg_fill = color(palette.accent_soft);
    visuals.selection.stroke = Stroke::new(1.0, color(palette.accent));
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, color(palette.border));
    visuals.widgets.inactive.bg_fill = color(palette.elevated);
    visuals.widgets.inactive.weak_bg_fill = color(palette.surface);
    visuals.widgets.hovered.bg_fill = color(palette.accent_soft);
    visuals.widgets.active.bg_fill = color(palette.accent);
    visuals
}

fn draw_mark(painter: &egui::Painter, rect: Rect, theme: Theme) {
    let rounded = rect.shrink(1.0);
    painter.rect_filled(rounded, 10.0, color(theme.elevated));
    painter.rect_stroke(
        rounded,
        10.0,
        Stroke::new(1.0, color(theme.border)),
        egui::StrokeKind::Inside,
    );
    let center = rect.center();
    painter.line_segment(
        [
            Pos2::new(center.x - 6.0, center.y - 5.0),
            Pos2::new(center.x - 1.0, center.y),
        ],
        Stroke::new(1.8, color(theme.accent)),
    );
    painter.line_segment(
        [
            Pos2::new(center.x - 1.0, center.y),
            Pos2::new(center.x - 6.0, center.y + 5.0),
        ],
        Stroke::new(1.8, color(theme.accent)),
    );
    painter.line_segment(
        [
            Pos2::new(center.x + 1.0, center.y + 5.0),
            Pos2::new(center.x + 7.0, center.y + 5.0),
        ],
        Stroke::new(1.8, color(theme.success)),
    );
}

fn subtle_button(ui: &mut egui::Ui, label: &str, theme: Theme) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(11.0).color(color(theme.muted)))
            .fill(color(theme.background))
            .stroke(Stroke::new(1.0, color(theme.border)))
            .corner_radius(7.0),
    )
}

fn accent_button(ui: &mut egui::Ui, label: &str, theme: Theme) -> egui::Response {
    ui.add(
        egui::Button::new(
            RichText::new(label)
                .size(12.0)
                .strong()
                .color(Color32::WHITE),
        )
        .fill(color(theme.accent))
        .stroke(Stroke::NONE)
        .corner_radius(8.0),
    )
}

fn color(rgb: Rgb) -> Color32 {
    Color32::from_rgb(rgb.r, rgb.g, rgb.b)
}
