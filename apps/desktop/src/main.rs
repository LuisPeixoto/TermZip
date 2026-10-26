mod app;
mod terminal_view;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([860.0, 560.0])
            .with_title("TermZip")
            .with_app_id("com.luispeixoto.termzip"),
        ..Default::default()
    };

    eframe::run_native(
        "TermZip",
        options,
        Box::new(|context| Ok(Box::new(app::TermZipApp::new(context)))),
    )
}
