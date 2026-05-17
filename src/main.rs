mod app;
mod document;
mod config;

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "PDF Viewer",
        options,
        Box::new(|cc| Ok(Box::new(app::PdfApp::new(cc)))),
    )
}
