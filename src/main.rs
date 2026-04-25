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

/// PMD-inspired dark theme colors (OKLCH approximated to sRGB)
/// Hierarchy: 88x > 80x > 72x > 8x > 4x
mod pmd {
    use egui::Color32;
    
    pub const BG_FLOOR: Color32 = Color32::from_rgb(41, 41, 41);     // 4x - background
    pub const SURFACE: Color32 = Color32::from_rgb(51, 51, 51);     // 8x - panels
    pub const TEXT_SUB: Color32 = Color32::from_rgb(176, 176, 176);  // 72x - subtext
    pub const TEXT_BODY: Color32 = Color32::from_rgb(204, 204, 204); // 80x - body
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(232, 232, 232); // 88x - headers
    pub const ACCENT: Color32 = Color32::from_rgb(86, 156, 214);     // Function blue
    pub const BORDER: Color32 = Color32::from_rgb(70, 70, 70);       // 8x border
    
    // Opacity variants
    pub const OPACITY_DISABLED: f32 = 0.24;
    pub const OPACITY_SECONDARY: f32 = 0.48;
    
    // Spacing (rem ~= 16px base)
    pub const SPACING_TIGHT: f32 = 4.0;     // 0.25rem
    pub const SPACING_STANDARD: f32 = 8.0;   // 0.5rem
    pub const SPACING_GENEROUS: f32 = 16.0;  // 1rem
    
    // Radius
    pub const RADIUS: f32 = 16.0; // 1rem
}

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
        Box::new(move |cc| {
            // Apply PMD-inspired styling
            let mut style = (*cc.egui_ctx.style()).clone();
            
            // Colors (PMD dark scheme)
            style.visuals.window_fill = pmd::BG_FLOOR;
            style.visuals.panel_fill = pmd::BG_FLOOR;
            style.visuals.extreme_bg_color = pmd::SURFACE;
            style.visuals.faint_bg_color = pmd::SURFACE;
            style.visuals.code_bg_color = pmd::SURFACE;
            
            // Text colors by hierarchy
            style.visuals.override_text_color = Some(pmd::TEXT_BODY);
            style.visuals.hyperlink_color = pmd::ACCENT;
            style.visuals.error_fg_color = egui::Color32::from_rgb(214, 86, 86);
            style.visuals.warn_fg_color = egui::Color32::from_rgb(214, 156, 86);
            
            // Widget styling
            style.visuals.widgets.noninteractive.bg_fill = pmd::SURFACE;
            style.visuals.widgets.inactive.bg_fill = pmd::SURFACE;
            style.visuals.widgets.hovered.bg_fill = pmd::SURFACE;
            style.visuals.widgets.active.bg_fill = pmd::SURFACE;
            
            // Button text
            style.visuals.widgets.noninteractive.fg_stroke.color = pmd::TEXT_SUB;
            style.visuals.widgets.inactive.fg_stroke.color = pmd::TEXT_BODY;
            style.visuals.widgets.hovered.fg_stroke.color = pmd::TEXT_PRIMARY;
            style.visuals.widgets.active.fg_stroke.color = pmd::TEXT_PRIMARY;
            
            // Borders (0.125rem = 2px)
            style.visuals.widgets.noninteractive.bg_stroke.width = 2.0;
            style.visuals.widgets.noninteractive.bg_stroke.color = pmd::BORDER;
            style.visuals.widgets.inactive.bg_stroke.width = 2.0;
            style.visuals.widgets.inactive.bg_stroke.color = pmd::BORDER;
            style.visuals.widgets.hovered.bg_stroke.width = 2.0;
            style.visuals.widgets.hovered.bg_stroke.color = pmd::TEXT_SUB;
            style.visuals.widgets.active.bg_stroke.width = 2.0;
            style.visuals.widgets.active.bg_stroke.color = pmd::TEXT_PRIMARY;
            
            // Spacing
            style.spacing.item_spacing = egui::vec2(pmd::SPACING_STANDARD, pmd::SPACING_STANDARD);
            style.spacing.button_padding = egui::vec2(pmd::SPACING_STANDARD, pmd::SPACING_TIGHT);
            style.spacing.interact_size = egui::vec2(32.0, 16.0);
            
            // Radius (1rem = 16px)
            style.visuals.window_rounding = egui::Rounding::same(pmd::RADIUS);
            style.visuals.menu_rounding = egui::Rounding::same(pmd::RADIUS);
            
            cc.egui_ctx.set_style(style);
            
            let mut app = App::new(config);
            app.load_backups();
            Box::new(app)
        }),
    )
}