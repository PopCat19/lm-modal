# Context

- `Cargo.toml` — Rust dependencies: egui + glow for Wayland overlay
- `flake.nix` — Nix flake with dev shell and Home Manager module output
- `src/main.rs` — Entry point, Wayland window creation, event loop
- `src/app.rs` — Application state machine (Idle/Loading/Done/Error)
- `src/api.rs` — OpenAI-compatible HTTP client (single request, no streaming)
- `src/config.rs` — CLI args and TOML config file parsing
- `modules/home-manager.nix` — Home Manager module for NixOS integration