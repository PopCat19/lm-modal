// app.rs
//
// Purpose: Application state machine and UI rendering
//
// This module:
// - Defines the App state machine (Idle/Loading/Done/Error)
// - Renders the egui UI using PMD design principles
// - Handles user input and clipboard operations
// - Manages conversation history and multiturn mode

use crate::api;
use crate::config::Config;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use serde::{Serialize, Deserialize};

/// Maximum backup sessions to keep
const MAX_BACKUPS: usize = 10;

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

/// Conversation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    SingleTurn,
    MultiTurn,
}

impl Mode {
    pub fn toggle(self) -> Self {
        match self {
            Mode::SingleTurn => Mode::MultiTurn,
            Mode::MultiTurn => Mode::SingleTurn,
        }
    }
    
    pub fn label(&self) -> &'static str {
        match self {
            Mode::SingleTurn => "single",
            Mode::MultiTurn => "multi",
        }
    }
}

/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
}

/// Conversation history for multiturn mode
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Conversation {
    pub messages: Vec<Message>,
}

impl Conversation {
    pub fn as_api_messages(&self) -> Vec<api::ApiMessage> {
        self.messages.iter().map(|m| api::ApiMessage {
            role: match m.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            },
            content: m.content.clone(),
        }).collect()
    }
    
    pub fn push_user(&mut self, content: String) {
        self.messages.push(Message {
            role: Role::User,
            content,
        });
    }
    
    pub fn push_assistant(&mut self, content: String) {
        self.messages.push(Message {
            role: Role::Assistant,
            content,
        });
    }
}

/// Application struct.
pub struct App {
    pub state: Arc<Mutex<State>>,
    pub input: String,
    pub config: Config,
    pub mode: Mode,
    pub conversation: Conversation,
    pub last_response: Option<String>,
    pub show_history: bool,
    pub backups: VecDeque<(i64, Conversation)>,
    clipboard: Option<arboard::Clipboard>,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::Idle)),
            input: String::new(),
            config,
            mode: Mode::default(),
            conversation: Conversation::default(),
            last_response: None,
            show_history: false,
            backups: VecDeque::new(),
            clipboard: arboard::Clipboard::new().ok(),
        }
    }

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

        let prompt = self.input.trim().to_string();
        
        let messages = match self.mode {
            Mode::SingleTurn => {
                vec![api::ApiMessage {
                    role: "user",
                    content: prompt.clone(),
                }]
            }
            Mode::MultiTurn => {
                self.conversation.push_user(prompt.clone());
                self.conversation.as_api_messages()
            }
        };

        {
            let mut s = self.state.lock().unwrap();
            *s = State::Loading;
        }

        let endpoint = self.config.endpoint.clone();
        let model = self.config.model.clone();
        let timeout = self.config.timeout;
        let state = self.state.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(api::complete_with_history(&endpoint, model.as_deref(), messages, timeout));

            let mut s = state.lock().unwrap();
            *s = match result {
                Ok(text) => State::Done(text),
                Err(e) => State::Error(e.to_string()),
            };

            ctx.request_repaint();
        });
        
        self.input.clear();
    }

    pub fn copy_response(&mut self) {
        if let Some(ref text) = self.last_response {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text.clone());
            }
        }
    }

    pub fn clear(&mut self) {
        if !self.conversation.messages.is_empty() {
            self.stash_backup();
        }
        
        self.input.clear();
        self.conversation = Conversation::default();
        self.last_response = None;
        let mut s = self.state.lock().unwrap();
        *s = State::Idle;
    }
    
    fn stash_backup(&mut self) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        
        self.backups.push_front((timestamp, self.conversation.clone()));
        
        while self.backups.len() > MAX_BACKUPS {
            self.backups.pop_back();
        }
        
        self.save_backups();
    }
    
    fn save_backups(&self) {
        if let Err(e) = self._save_backups_impl() {
            eprintln!("Failed to save backups: {}", e);
        }
    }
    
    fn _save_backups_impl(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config.backups_dir)?;
        let path = self.config.backups_dir.join("backups.json");
        let json = serde_json::to_string_pretty(&self.backups)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    pub fn load_backups(&mut self) {
        if let Err(e) = self._load_backups_impl() {
            eprintln!("Failed to load backups: {}", e);
        }
    }
    
    fn _load_backups_impl(&mut self) -> std::io::Result<()> {
        let path = self.config.backups_dir.join("backups.json");
        if !path.exists() {
            return Ok(());
        }
        let json = std::fs::read_to_string(path)?;
        self.backups = serde_json::from_str(&json).unwrap_or_default();
        Ok(())
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle response from loading state
        {
            let mut s = self.state.lock().unwrap();
            if let State::Done(response) = s.clone() {
                self.last_response = Some(response.clone());
                if self.mode == Mode::MultiTurn {
                    self.conversation.push_assistant(response);
                }
                *s = State::Idle;
            }
        }
        
        // Keyboard shortcuts
        ctx.input_mut(|i| {
            if i.key_pressed(egui::Key::Tab) {
                self.mode = self.mode.toggle();
                i.events.retain(|e| !matches!(e, egui::Event::Key { key: egui::Key::Tab, .. }));
            }
            if i.key_pressed(egui::Key::Escape) {
                self.clear();
            }
            if i.modifiers.shift && i.key_pressed(egui::Key::C) {
                self.copy_response();
            }
            if i.key_pressed(egui::Key::M) && !i.modifiers.any() {
                self.mode = self.mode.toggle();
            }
            if i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl && !self.input.trim().is_empty() {
                self.send(ctx.clone());
            }
        });

        // PMD color constants
        use egui::{Color32, RichText, Vec2};
        const BG_FLOOR: Color32 = Color32::from_rgb(41, 41, 41);
        const SURFACE: Color32 = Color32::from_rgb(51, 51, 51);
        const TEXT_SUB: Color32 = Color32::from_rgb(176, 176, 176);
        const TEXT_BODY: Color32 = Color32::from_rgb(204, 204, 204);
        const TEXT_PRIMARY: Color32 = Color32::from_rgb(232, 232, 232);
        const ACCENT: Color32 = Color32::from_rgb(86, 156, 214);
        
        const SPACING_TIGHT: f32 = 4.0;
        const SPACING_STANDARD: f32 = 8.0;
        const SPACING_GENEROUS: f32 = 16.0;
        const RADIUS: f32 = 16.0;

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            let available = ui.available_size();
            
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                // === HEADER (88x) ===
                ui.horizontal(|ui| {
                    ui.label(RichText::new("lm-modal").strong().color(TEXT_PRIMARY));
                    ui.add_space(SPACING_GENEROUS);
                    
                    // Mode indicator (72x)
                    ui.label(RichText::new(self.mode.label()).small().color(TEXT_SUB));
                    ui.add_space(SPACING_STANDARD);
                    
                    // Action buttons (72x text)
                    if ui.small_button("history").clicked() {
                        self.show_history = !self.show_history;
                    }
                });
                
                ui.add_space(SPACING_GENEROUS);

                // === CONVERSATION HISTORY (80x) ===
                if self.mode == Mode::MultiTurn && !self.conversation.messages.is_empty() {
                    let hist_height = (available.y * 0.3).min(150.0);
                    egui::ScrollArea::vertical()
                        .max_height(hist_height)
                        .show(ui, |ui| {
                            for msg in &self.conversation.messages {
                                let (color, prefix) = match msg.role {
                                    Role::User => (ACCENT, "You: "),
                                    Role::Assistant => (TEXT_BODY, "AI: "),
                                };
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(prefix).color(color).strong());
                                    ui.label(RichText::new(&msg.content).color(TEXT_BODY));
                                });
                            }
                        });
                    ui.add_space(SPACING_STANDARD);
                }

                // === INPUT AREA (80x) ===
                let input_height = (available.y * 0.15).max(60.0);
                ui.add_sized([available.x, input_height],
                    egui::TextEdit::multiline(&mut self.input)
                        .desired_width(f32::INFINITY)
                        .hint_text(RichText::new("Ask... (Ctrl+Enter=send, Tab/M=mode)").color(TEXT_SUB))
                );

                // === BUTTONS (72x) ===
                ui.horizontal(|ui| {
                    let is_loading = self.state.lock().unwrap().is_loading();
                    let can_send = !self.input.trim().is_empty() && !is_loading;

                    if ui.add_enabled(can_send, egui::Button::new("Send")).clicked() {
                        self.send(ctx.clone());
                    }
                    if ui.button("Clear").clicked() {
                        self.clear();
                    }
                    if ui.small_button(match self.mode {
                        Mode::SingleTurn => "→ multi",
                        Mode::MultiTurn => "→ single",
                    }).clicked() {
                        self.mode = self.mode.toggle();
                    }
                });

                ui.add_space(SPACING_GENEROUS);

                // === RESPONSE AREA (80x) ===
                if let Some(ref response) = self.last_response {
                    let response_clone = response.clone();
                    let response_text = response.clone();
                    let remaining = ui.available_height() - SPACING_STANDARD;
                    
                    egui::ScrollArea::vertical()
                        .max_height(remaining)
                        .show(ui, |ui| {
                            // Response container with surface background
                            egui::Frame::group(ui.style())
                                .fill(SURFACE)
                                .rounding(RADIUS)
                                .inner_margin(Vec2::splat(SPACING_STANDARD))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("Response:").strong().color(TEXT_PRIMARY));
                                        if ui.small_button("Copy").clicked() {
                                            if let Some(clipboard) = &mut self.clipboard {
                                                let _ = clipboard.set_text(response_clone);
                                            }
                                        }
                                    });
                                    ui.add_space(SPACING_TIGHT);
                                    ui.label(RichText::new(&response_text).color(TEXT_BODY));
                                });
                        });
                }

                // === LOADING INDICATOR ===
                if self.state.lock().unwrap().is_loading() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new("Thinking...").color(TEXT_SUB));
                    });
                }

                // === ERROR DISPLAY (72x) ===
                if let State::Error(e) = self.state.lock().unwrap().clone() {
                    ui.label(RichText::new(format!("Error: {}", e)).color(Color32::from_rgb(214, 86, 86)));
                }

                // === HISTORY VIEWER ===
                if self.show_history {
                    ui.add_space(SPACING_STANDARD);
                    egui::CollapsingHeader::new("Backups")
                        .default_open(true)
                        .show(ui, |ui| {
                            if self.backups.is_empty() {
                                ui.label(RichText::new("No backup sessions").color(TEXT_SUB));
                            } else {
                                for (i, (ts, conv)) in self.backups.iter().enumerate() {
                                    let dt = chrono_timestamp(*ts);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(format!("#{} {} - {} msgs", i + 1, dt, conv.messages.len())).color(TEXT_BODY));
                                        if ui.small_button("restore").clicked() {
                                            self.conversation = conv.clone();
                                            self.show_history = false;
                                            self.last_response = None;
                                            *self.state.lock().unwrap() = State::Idle;
                                        }
                                    });
                                }
                            }
                        });
                }
            });
        });
    }
}

fn chrono_timestamp(ts: i64) -> String {
    use std::time::{UNIX_EPOCH, SystemTime};
    let duration = UNIX_EPOCH + std::time::Duration::from_secs(ts as u64);
    let datetime: chrono::DateTime<chrono::Local> = duration.into();
    datetime.format("%m/%d %H:%M").to_string()
}