#![cfg_attr(all(not(debug_assertions), not(test)), windows_subsystem = "windows")]
mod client;
mod data;
mod hotkeys;
mod live;
mod recommendations;
mod rune_icons;
mod sprites;
mod ui;
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([1000.0, 680.0]),
        renderer: eframe::Renderer::Glow,
        vsync: true,
        ..Default::default()
    };
    eframe::run_native(
        "Lightrift - League companion",
        options,
        Box::new(|cc| Ok(Box::new(ui::Lightrift::new(cc)))),
    )
}
