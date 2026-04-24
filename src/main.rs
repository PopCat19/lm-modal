// main.rs
//
// Purpose: Entry point and Wayland window creation
//
// This module:
// - Initializes the Wayland overlay window
// - Sets up the egui + glow renderer
// - Runs the application event loop

mod api;
mod app;
mod config;

use app::App;
use config::Config;

fn main() -> eframe::Result<()> {
    let config = Config::load();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("lm-modal")
            .with_inner_size([600.0, 400.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top(),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "lm-modal",
        options,
        Box::new(move |_cc| {
            let mut app = App::new(config);
            app.load_backups();
            Box::new(app)
        }),
    )
}