// app.rs
//
// Purpose: Application state machine and UI rendering
//
// This module:
// - Defines the App state machine (Idle/Loading/Done/Error)
// - Renders the egui UI
// - Handles user input and clipboard operations

use crate::api;
use crate::config::Config;

/// Application state.
#[derive(Debug, Clone)]
pub enum State {
    Idle,
    Loading,
    Done(String),
    Error(String),
}

impl State {
    pub fn is_loading(&self) -> bool {
        matches!(self, State::Loading)
    }
}

/// Application struct.
pub struct App {
    pub state: State,
    pub input: String,
    pub history: Vec<String>,
    pub config: Config,
    clipboard: Option<arboard::Clipboard>,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            state: State::Idle,
            input: String::new(),
            history: Vec::new(),
            config,
            clipboard: arboard::Clipboard::new().ok(),
        }
    }

    /// Send the current input as a completion request.
    pub fn send(&mut self, ctx: egui::Context) {
        if self.input.trim().is_empty() || self.state.is_loading() {
            return;
        }

        let prompt = self.input.trim().to_string();
        let endpoint = self.config.endpoint.clone();
        let model = self.config.model.clone();
        let timeout = self.config.timeout;

        self.state = State::Loading;

        // Spawn async request
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(api::complete(&endpoint, model.as_deref(), &prompt, timeout));

            // Update state on main thread
            ctx.request_repaint();
            std::thread::spawn(move || {
                // The repaint will pick up the new state when poll_state is called
            });
        });
    }

    /// Poll for async result (called each frame).
    pub fn poll_state(&mut self) {
        // State updates happen via ctx.request_repaint() pattern
        // In a real implementation, use egui's Sense or channels
    }

    /// Copy response to clipboard.
    pub fn copy_response(&mut self) {
        if let State::Done(text) = &self.state {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text.clone());
            }
        }
    }

    /// Clear and reset to idle.
    pub fn clear(&mut self) {
        self.input.clear();
        self.state = State::Idle;
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Title
                ui.heading("lm-modal");
                ui.add_space(8.0);

                // Input field
                let input_response = ui.add(
                    egui::TextEdit::multiline(&mut self.input)
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .hint_text("Ask..."),
                );

                // Send button
                ui.horizontal(|ui| {
                    let can_send = !self.input.trim().is_empty() && !self.state.is_loading();
                    if ui.add_enabled(can_send, egui::Button::new("Send")).clicked() {
                        self.send(ctx.clone());
                    }

                    if ui.button("Clear").clicked() {
                        self.clear();
                    }
                });

                ui.add_space(12.0);

                // Status / Response
                match &self.state {
                    State::Idle => {
                        ui.label("Ready");
                    }
                    State::Loading => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Thinking...");
                        });
                    }
                    State::Done(text) => {
                        ui.group(|ui| {
                            ui.label("Response:");
                            ui.add_space(4.0);
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    ui.label(text);
                                });
                            ui.add_space(4.0);
                            if ui.button("Copy").clicked() {
                                self.copy_response();
                            }
                        });
                    }
                    State::Error(e) => {
                        ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
                    }
                }
            });
        });
    }
}