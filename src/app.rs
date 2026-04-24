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
use std::sync::{Arc, Mutex};

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
    pub state: Arc<Mutex<State>>,
    pub input: String,
    pub config: Config,
    clipboard: Option<arboard::Clipboard>,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::Idle)),
            input: String::new(),
            config,
            clipboard: arboard::Clipboard::new().ok(),
        }
    }

    /// Send the current input as a completion request.
    pub fn send(&mut self, ctx: egui::Context) {
        if self.input.trim().is_empty() {
            return;
        }

        {
            let s = self.state.lock().unwrap();
            if s.is_loading() {
                return;
            }
        }

        {
            let mut s = self.state.lock().unwrap();
            *s = State::Loading;
        }

        let prompt = self.input.trim().to_string();
        let endpoint = self.config.endpoint.clone();
        let model = self.config.model.clone();
        let timeout = self.config.timeout;
        let state = self.state.clone();

        // Spawn async request
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(api::complete(&endpoint, model.as_deref(), &prompt, timeout));

            let mut s = state.lock().unwrap();
            *s = match result {
                Ok(text) => State::Done(text),
                Err(e) => State::Error(e.to_string()),
            };

            ctx.request_repaint();
        });
    }

    /// Copy response to clipboard.
    pub fn copy_response(&mut self, text: &str) {
        if let Some(clipboard) = &mut self.clipboard {
            let _ = clipboard.set_text(text.to_string());
        }
    }

    /// Clear and reset to idle.
    pub fn clear(&mut self) {
        self.input.clear();
        let mut s = self.state.lock().unwrap();
        *s = State::Idle;
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
                ui.add(
                    egui::TextEdit::multiline(&mut self.input)
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .hint_text("Ask..."),
                );

                // Buttons
                ui.horizontal(|ui| {
                    let is_loading = {
                        let s = self.state.lock().unwrap();
                        s.is_loading()
                    };
                    let can_send = !self.input.trim().is_empty() && !is_loading;

                    if ui.add_enabled(can_send, egui::Button::new("Send")).clicked() {
                        self.send(ctx.clone());
                    }

                    if ui.button("Clear").clicked() {
                        self.clear();
                    }
                });

                ui.add_space(12.0);

                // Clone state for display
                let state_clone = {
                    let s = self.state.lock().unwrap();
                    s.clone()
                };

                match state_clone {
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
                        let text_for_copy = text.clone();
                        ui.group(|ui| {
                            ui.label("Response:");
                            ui.add_space(4.0);
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    ui.label(&text);
                                });
                            ui.add_space(4.0);
                            if ui.button("Copy").clicked() {
                                self.copy_response(&text_for_copy);
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