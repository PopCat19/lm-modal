// config.rs
//
// Purpose: Parse CLI arguments and TOML configuration file
//
// This module:
// - Defines the Config struct with all configuration options
// - Parses command-line arguments
// - Falls back to TOML config file at XDG config home
// - Provides defaults for all options

use serde::Deserialize;
use std::path::PathBuf;

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// OpenAI-compatible API endpoint
    pub endpoint: String,
    /// Model name (None uses endpoint default)
    pub model: Option<String>,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Backup sessions directory
    pub backups_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("lm-modal");

        Self {
            endpoint: String::from("http://localhost:8088"),
            model: None,
            timeout: 120,
            backups_dir: data_dir.join("backups"),
        }
    }
}

/// TOML config file structure.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ConfigFile {
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub timeout: Option<u64>,
}

impl Config {
    /// Load configuration from CLI args and config file.
    pub fn load() -> Self {
        let args = parse_args();
        let file_config = load_config_file();

        let mut config = Config::default();

        // CLI args take precedence over config file
        if let Some(endpoint) = args.endpoint.or(file_config.endpoint) {
            config.endpoint = endpoint;
        }
        config.model = args.model.or(file_config.model);
        if let Some(timeout) = args.timeout.or(file_config.timeout) {
            config.timeout = timeout;
        }

        // Ensure directories exist
        std::fs::create_dir_all(&config.backups_dir).ok();

        config
    }
}

/// Parsed CLI arguments.
struct Args {
    endpoint: Option<String>,
    model: Option<String>,
    timeout: Option<u64>,
}

fn parse_args() -> Args {
    let mut args = Args {
        endpoint: None,
        model: None,
        timeout: None,
    };

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--endpoint" | "-e" => {
                args.endpoint = iter.next();
            }
            "--model" | "-m" => {
                args.model = iter.next();
            }
            "--timeout" | "-t" => {
                args.timeout = iter.next().and_then(|s| s.parse().ok());
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
    }

    args
}

fn load_config_file() -> ConfigFile {
    let config_path = dirs::config_dir()
        .map(|p| p.join("lm-modal/config.toml"))
        .unwrap_or_else(|| PathBuf::from("config.toml"));

    if !config_path.exists() {
        return ConfigFile::default();
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return ConfigFile::default(),
    };

    toml::from_str(&content).unwrap_or_default()
}

fn print_help() {
    println!("lm-modal - Wayland-native LLM overlay");
    println!();
    println!("Usage: lm-modal [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -e, --endpoint <URL>  API endpoint (default: http://localhost:8088)");
    println!("  -m, --model <NAME>    Model name (default: endpoint default)");
    println!("  -t, --timeout <SEC>   Request timeout (default: 120)");
    println!("  -h, --help            Show this help message");
    println!();
    println!("Keyboard Shortcuts:");
    println!("  Ctrl+Enter  Send message");
    println!("  Shift+C     Copy response");
    println!("  Tab         Toggle single/multi-turn mode");
    println!("  Esc         Clear session");
}