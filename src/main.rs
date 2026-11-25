mod app;
mod archive;
mod build;
mod github;
mod model;
mod store;
mod theme;
mod ui;
mod worker;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Ultima")
            .with_inner_size([1560.0, 940.0])
            .with_min_inner_size([1180.0, 720.0])
            .with_app_id("ultima"),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Ultima",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
