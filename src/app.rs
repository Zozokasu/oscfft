use spectrum_analyzer::scaling::divide_by_N_sqrt;
use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
use async_osc::{OscSocket};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    label: String,

    #[serde(skip)]
    value: f32,

    #[serde(skip)]
    cpal_host: cpal::Host,

    #[serde(skip)]
    device_configuration: DeviceConfiguration,

    #[serde(skip)]
    target_address: String,

    #[serde(skip)]
    target_port: u16,

    #[serde(skip)]
    is_running: bool,
}

pub struct DeviceConfiguration {
    pub input_device: Option<String>,
    pub input_devices: Vec<String>,
    pub handle: Option<cpal::Stream>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        use cpal::traits::*;

        let host = cpal::default_host();
        let ins = host.input_devices().unwrap();

        Self {
            label: "Hello World!".to_owned(),
            value: 2.7,
            cpal_host: host,
            device_configuration: DeviceConfiguration {
                input_device: None,
                input_devices: ins
                    .into_iter()
                    .map(|v| v.name().unwrap().to_string())
                    .collect(),
                handle: None,
            },
            target_address: "127.0.0.1".to_string(),
            target_port: 50000,
            is_running: false,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                // ui.menu_button("File", |ui| {
                //     if ui.button("Quit").clicked() {
                //         ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                //     }
                //     if ui.button("ahoaho").clicked() {
                //         println!("ahoaho");
                //     }
                // });

                // ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Audio Input -> FFT -> OSC Sender");

            // ui.horizontal(|ui| {
            //     ui.label("Write something: ");
            //     ui.text_edit_singleline(&mut self.label);
            // });

            // ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            // if ui.button("Increment").clicked() {
            //     self.value += 1.0;
            // }

            let ins = &self.device_configuration.input_devices;

            ui.horizontal(|ui| {
                ui.label("Device Count: ");
                ui.label(ins.len().to_string())
            });

            egui::ComboBox::from_label("Select Input Device")
                .selected_text(match &self.device_configuration.input_device {
                    Some(s) => s.to_string(),
                    None => "-".to_string(),
                })
                .show_ui(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);

                    ins.iter().for_each(|in_name| {
                        ui.selectable_value(
                            &mut self.device_configuration.input_device,
                            Some(in_name.clone()),
                            in_name.clone(),
                        );
                    });
                });

            ui.horizontal(|ui| {
                ui.label("Receiver IP address: ");
                ui.text_edit_singleline(&mut self.target_address);
            });

            ui.horizontal(|ui| {
                ui.label("Receiver Port:");
                ui.add(egui::Slider::new(&mut self.target_port, 0..=65535).text(""));
            });

            if ui.button(if self.is_running {"Stop"} else {"Start"}).clicked() {
                if self.device_configuration.handle.is_some() {
                    self.device_configuration.handle = None;
                    self.is_running = false;
                    //println!("ハンドルを開放しました。");
                } else {
                    let Some(input) = &self.device_configuration.input_device else {
                        return;
                    };
                    self.is_running = true;

                    use cpal::traits::*;

                    let input = self
                        .cpal_host
                        .input_devices()
                        .unwrap()
                        .find(|v| v.name().unwrap().as_str() == input.as_str())
                        .unwrap();

                    let config: cpal::StreamConfig = input.default_input_config().unwrap().into();
                    use tokio::sync::mpsc;
                    let (tx, mut rx) = mpsc::channel::<Vec<f32>>(16);

                    //let socket = OscSocket::bind(format!("{}:{}", self.target_address, self.target_port));
                    let addr_port = format!("{}:{}",self.target_address, self.target_port);
                    let bind = format!("{}:0",self.target_address);

                    tokio::spawn(async move {
                        let mut recv_buffer: Vec<f32> = vec![];
                        let socket = OscSocket::bind(bind).await.unwrap();
                        //println!("{}",addr_port);
                        socket.connect(&addr_port).await.unwrap();

                        while let Some(new) = rx.recv().await {
                            recv_buffer.extend(new);

                            while recv_buffer.len() >= 2048 {
                                let hann_window = hann_window(&recv_buffer[0..2048]);

                                let spectrum_hann_window = samples_fft_to_spectrum(
                                    &hann_window,
                                    config.sample_rate.0,
                                    FrequencyLimit::All,
                                    Some(&divide_by_N_sqrt),
                                )
                                .unwrap();
                                let x:Vec<f32> = spectrum_hann_window.data().iter().map(|x| x.1.val()).collect();
                                let x0 = &x[0..256];
                                let x1 = &x[256..512];
                                let x2 = &x[512..768];
                                let x3 = &x[768..1024];
                                let message0 = async_osc::rosc::OscMessage{addr:"/fft/0".to_string(),args:x0.iter().map(|x| async_osc::rosc::OscType::Float(*x)).collect::<Vec<async_osc::rosc::OscType>>()};
                                let message1 = async_osc::rosc::OscMessage{addr:"/fft/1".to_string(),args:x1.iter().map(|x| async_osc::rosc::OscType::Float(*x)).collect::<Vec<async_osc::rosc::OscType>>()};
                                let message2 = async_osc::rosc::OscMessage{addr:"/fft/2".to_string(),args:x2.iter().map(|x| async_osc::rosc::OscType::Float(*x)).collect::<Vec<async_osc::rosc::OscType>>()};
                                let message3 = async_osc::rosc::OscMessage{addr:"/fft/3".to_string(),args:x3.iter().map(|x| async_osc::rosc::OscType::Float(*x)).collect::<Vec<async_osc::rosc::OscType>>()};
                                socket.send(message0).await.unwrap();
                                socket.send(message1).await.unwrap();
                                socket.send(message2).await.unwrap();
                                socket.send(message3).await.unwrap();

                                recv_buffer = Vec::from(&recv_buffer[2048..]);
                            }
                        }
                        //println!("受信スレッドが停止しました。");
                    });

                    let input_data_fn = move |samples: &[f32], _: &cpal::InputCallbackInfo| {
                        tx.blocking_send(samples.to_owned()).unwrap();
                    };

                    let input_stream = input
                        .build_input_stream(
                            &config,
                            input_data_fn,
                            |err: cpal::StreamError| eprintln!("{:?}", err),
                            None,
                        )
                        .unwrap();

                    input_stream.play().unwrap();

                    self.device_configuration.handle = Some(input_stream);
                }
            }

            ui.separator();

            ui.hyperlink_to("GitHub", "https://github.com/Zozokasu/oscfft");

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                made_by_zozokasu(ui)
            });
        });
    }
}

// fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
//     ui.horizontal(|ui| {
//         ui.spacing_mut().item_spacing.x = 0.0;
//         ui.label("Powered by ");
//         ui.hyperlink_to("egui", "https://github.com/emilk/egui");
//         ui.label(" and ");
//         ui.hyperlink_to(
//             "eframe",
//             "https://github.com/emilk/egui/tree/master/crates/eframe",
//         );
//         ui.label(".");
//     });
// }

fn made_by_zozokasu(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Made by ");
        ui.hyperlink_to("zozokasu", "https://zozoka.su/");
    });
}
