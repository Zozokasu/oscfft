#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![windows_subsystem = "windows"]

fn main() -> eframe::Result<()> {
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let rt = tokio::runtime::Runtime::new().unwrap();

    let _enter = rt.enter();

    std::thread::spawn(move || {
        rt.block_on(async {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        })
    });


    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon.png")[..])
                    .unwrap(),
            ),
        ..Default::default()
    };

    eframe::run_native(
        "Sound FFT to OSC",
        native_options,
        Box::new(|cc| Box::new(fft2osc::TemplateApp::new(cc))),
    )
}
