// app.rs
//
// Purpose: Application state machine and UI rendering
//
// This module:
// - Defines the App state machine (Idle/Loading/Done/Error)
// - Renders the egui UI
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
    Loading { pending_user_msg: String },
    Done(String),
    Error(String),
}

impl State {
    pub fn is_loading(&self) -> bool {
        matches!(self, State::Loading { .. })
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
    
    /// Multiturn mode toggle
    pub mode: Mode,
    
    /// Conversation history for multiturn
    pub conversation: Conversation,
    
    /// Last response (for copy)
    pub last_response: Option<String>,
    
    /// History tree viewer visible
    pub show_history: bool,
    
    /// Backup sessions (last N temporary sessions)
    pub backups: VecDeque<(i64, Conversation)>,
    
    /// Clipboard handle
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

        let prompt = self.input.trim().to_string();
        
        // Store user message and build API messages based on mode
        let messages = match self.mode {
            Mode::SingleTurn => {
                vec![api::ApiMessage {
                    role: "user",
                    content: prompt.clone(),
                }]
            }
            Mode::MultiTurn => {
                // Add user message to conversation
                self.conversation.push_user(prompt.clone());
                self.conversation.as_api_messages()
            }
        };

        // Set loading state
        {
            let mut s = self.state.lock().unwrap();
            *s = State::Loading { pending_user_msg: prompt };
        }

        let endpoint = self.config.endpoint.clone();
        let model = self.config.model.clone();
        let timeout = self.config.timeout;
        let state = self.state.clone();
        let mode = self.mode;

        // Spawn async request
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
        
        // Clear input after sending
        self.input.clear();
    }

    /// Copy response to clipboard.
    pub fn copy_response(&mut self) {
        if let Some(ref text) = self.last_response {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text.clone());
            }
        }
    }

    /// Clear and reset to idle.
    pub fn clear(&mut self) {
        // Stash current conversation if it has content
        if !self.conversation.messages.is_empty() {
            self.stash_backup();
        }
        
        self.input.clear();
        self.conversation = Conversation::default();
        self.last_response = None;
        let mut s = self.state.lock().unwrap();
        *s = State::Idle;
    }
    
    /// Stash current conversation to backup
    fn stash_backup(&mut self) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        
        self.backups.push_front((timestamp, self.conversation.clone()));
        
        // Rotate: remove oldest if over limit
        while self.backups.len() > MAX_BACKUPS {
            self.backups.pop_back();
        }
        
        // Persist to disk
        self.save_backups();
    }
    
    /// Save backups to disk
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
    
    /// Load backups from disk
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
        // Handle response from loading state (once)
        {
            let mut s = self.state.lock().unwrap();
            if let State::Done(response) = s.clone() {
                // Store response and add to conversation
                self.last_response = Some(response.clone());
                if self.mode == Mode::MultiTurn {
                    self.conversation.push_assistant(response);
                }
                // Reset to Idle after handling
                *s = State::Idle;
            }
        }
        
        // Keyboard shortcuts - handle before widgets
        let mut should_toggle_mode = false;
        ctx.input_mut(|i| {
            // Escape to clear/close
            if i.key_pressed(egui::Key::Escape) {
                self.clear();
            }
            
            // Tab to toggle mode (consume event to prevent focus cycling)
            if i.key_pressed(egui::Key::Tab) {
                should_toggle_mode = true;
                i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
            }
            
            // Shift+C to copy response
            if i.modifiers.shift && i.key_pressed(egui::Key::C) {
                self.copy_response();
            }
            
            // Ctrl+Enter to send
            if i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl && !self.input.trim().is_empty() {
                self.send(ctx.clone());
            }
        });
        
        if should_toggle_mode {
            self.mode = self.mode.toggle();
        }

        // Central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header with mode indicator
                ui.horizontal(|ui| {
                    ui.heading("lm-modal");
                    ui.add_space(8.0);
                    let mode_text = match self.mode {
                        Mode::SingleTurn => "single",
                        Mode::MultiTurn => "multi",
                    };
                    ui.label(
                        egui::RichText::new(mode_text)
                            .small()
                            .color(egui::Color32::from_rgb(128, 128, 128))
                    );
                    ui.add_space(8.0);
                    if ui.small_button("history").clicked() {
                        self.show_history = !self.show_history;
                    }
                });
                ui.add_space(8.0);

                // Show conversation history in multiturn mode
                if self.mode == Mode::MultiTurn && !self.conversation.messages.is_empty() {
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .show(ui, |ui| {
                            for msg in &self.conversation.messages {
                                let (color, prefix) = match msg.role {
                                    Role::User => (egui::Color32::LIGHT_BLUE, "You: "),
                                    Role::Assistant => (egui::Color32::LIGHT_GREEN, "AI: "),
                                };
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(prefix).color(color).strong());
                                    ui.label(&msg.content);
                                });
                            }
                        });
                    ui.add_space(8.0);
                }

                // Input field
                ui.add(
                    egui::TextEdit::multiline(&mut self.input)
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .hint_text("Ask... (Ctrl+Enter=send, Tab=mode)")
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

                // State display
                let state_clone = {
                    let s = self.state.lock().unwrap();
                    s.clone()
                };

                match state_clone {
                    State::Idle => {}
                    State::Loading { .. } => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Thinking...");
                        });
                    }
                    State::Done(text) => {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label("Response:");
                                if ui.small_button("Copy (Shift+C)").clicked() {
                                    self.last_response = Some(text.clone());
                                    self.copy_response();
                                }
                            });
                            ui.add_space(4.0);
                            egui::ScrollArea::vertical()
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    ui.label(&text);
                                });
                        });
                    }
                    State::Error(e) => {
                        ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
                    }
                }
                
                // History viewer (if open)
                if self.show_history {
                    ui.add_space(8.0);
                    egui::CollapsingHeader::new("Backups")
                        .default_open(true)
                        .show(ui, |ui| {
                            if self.backups.is_empty() {
                                ui.label("No backup sessions");
                            } else {
                                for (i, (ts, conv)) in self.backups.iter().enumerate() {
                                    let dt = chrono_timestamp(*ts);
                                    ui.horizontal(|ui| {
                                        ui.label(format!("#{} {} - {} msgs", i + 1, dt, conv.messages.len()));
                                        if ui.small_button("restore").clicked() {
                                            self.conversation = conv.clone();
                                            self.show_history = false;
                                            self.last_response = None;
                                            let mut s = self.state.lock().unwrap();
                                            *s = State::Idle;
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