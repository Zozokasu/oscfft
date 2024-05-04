use std::sync::atomic::spin_loop_hint;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
use portaudio as pa;
use portaudio::{DeviceIndex, Duplex, DuplexStreamCallbackArgs, NonBlocking, PortAudio, Stream, StreamCallbackResult};
use serde::__private::de::AdjacentlyTaggedEnumVariantVisitor;
use wasm_bindgen_futures::wasm_bindgen::closure::Closure;
use std::thread;
use std::time::Duration;
use eframe::CreationContext;
use portaudio::stream::{CallbackResult, DuplexCallbackArgs, DuplexSettings};
use spectrum_analyzer::{Frequency, FrequencyLimit, FrequencySpectrum, FrequencyValue, samples_fft_to_spectrum};
use spectrum_analyzer::scaling::divide_by_N_sqrt;
use spectrum_analyzer::windows::hann_window;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,
    num_devices: u32,
    #[serde(skip)]
    port_audio: PortAudio,
    #[serde(skip)]
    state: AppState,
}

struct AppState {
    duration: f32,
    processing: bool,
    output_device: DeviceIndex,
    input_device: DeviceIndex,
    sample_rate: f64,
    frames: u32,
    channels: i32,
    interleaved: bool,
    port_audio: PortAudio,
    pa_settings: DuplexSettings<f32, f32>,
    pa_stream: Stream<NonBlocking, Duplex<f32, f32>>,
    stream_manager: StreamManager
}

struct StreamManager {
    sender: Sender<Vec<f32>>,
    receiver: Receiver<Vec<f32>>,
    //callback: Closure<args: DuplexCallbackArgs<f32,f32> -> CallbackResult>,
}
impl StreamManager{
    fn new() -> Self{
        let (sender, receiver) = ::std::sync::mpsc::channel();
        Self{
            sender,
            receiver,
        }
    }

    fn callback(&self, args: DuplexCallbackArgs<f32,f32>) -> CallbackResult {
        self.sender.send(args.in_buffer.to_vec()).ok();
        return pa::Continue
    }
    fn get_callback(&self) -> Closure<Fn(DuplexCallbackArgs<f32,f32>) -> CallbackResult>{
        let callback = self.callback;
        return Closure::wrap(Box::new(callback) as Box<Fn(DuplexCallbackArgs<f32,f32>) -> CallbackResult>);
    }

    fn calc_spectrum(&self) -> FrequencySpectrum{
        let sample = self.receiver.try_recv().unwrap();
        let hann_window = hann_window(&sample[0..2048]);
        let spectrum_hann_window = samples_fft_to_spectrum(
            &hann_window,
            44100,
                FrequencyLimit::All,
            Some(&divide_by_N_sqrt),
        ).unwrap();
        return spectrum_hann_window;
    }

}

impl AppState {
    fn new(output: DeviceIndex, input: DeviceIndex, ) -> Self {
        let stream_manager = StreamManager::new();
        let mut pa = PortAudio::new().unwrap();
        let mut setting = pa_default_setting(&mut pa, 2, 44_100.0, 2048, true);
        let mut stream = pa.open_non_blocking_stream(setting, stream_manager.get_callback).unwrap();
        Self {
            duration: 10.0,
            processing: false,
            input_device: input,
            output_device: output,
            sample_rate: 44_100.0,
            frames: 2048,
            channels: 2,
            interleaved: true,
            port_audio: pa,
            pa_settings: setting,
            pa_stream: stream,
            stream_manager
        }
    }

    fn fft_sample(&self) -> Vec<(Frequency, FrequencyValue)> {
        let samples = self.stream_manager.receiver.try_recv().unwrap();
        let hann_window = hann_window(&samples[0..2048]);
        let spectrum_hann_window = samples_fft_to_spectrum(
            &hann_window,
            44100,
            FrequencyLimit::All,
            Some(&divide_by_N_sqrt),
        ).unwrap();
        return spectrum_hann_window.data().to_vec();
    }

    fn run(&'static mut self) {
        thread::spawn(move || {
            self.audio_process();
        });
    }
    fn audio_process(&self) {
        //消すとクラッシュする
        println!("hi");
        loop {
            println!("HI!")
        }
    }

    fn stream_start(&mut self) {
        self.pa_stream.start().unwrap()
    }

    fn stream_stop(&mut self) {
        self.pa_stream.stop().unwrap()
    }
}

fn pa_default_setting(pa:&mut PortAudio,channels:i32, sample_rate: f64, frames: u32 , interleaved: bool) -> DuplexSettings<f32,f32>{

    let def_input = pa.default_input_device().unwrap();
    let input_info = pa.device_info(def_input).unwrap();

    // Construct the input stream parameters.
    let latency = input_info.default_low_input_latency;
    let input_params = pa::StreamParameters::<f32>::new(def_input, channels, interleaved, latency);

    let def_output = pa.default_output_device().unwrap();
    let output_info = pa.device_info(def_output).unwrap();

    // Construct the output stream parameters.
    let latency = output_info.default_low_output_latency;
    let output_params = pa::StreamParameters::<f32>::new(def_output, channels, interleaved, latency);

    // Check that the stream format is supported.
    pa.is_duplex_format_supported(input_params, output_params, sample_rate).unwrap();

    // Construct the settings with which we'll open our duplex stream.
    let settings = pa::DuplexStreamSettings::new(input_params, output_params, sample_rate, frames);
    return settings;


}
impl Default for TemplateApp {
    fn default() -> Self {
        let mut pa = PortAudio::new().unwrap();
        let device_count = pa.device_count().unwrap();
        for index in 0..device_count {
            let device = pa.device_info(DeviceIndex(index)).unwrap();
            println!("{}: Inputs - {}, Outputs - {}",device.name,device.max_input_channels, device.max_output_channels);
        }
        let default_output = pa.default_output_device().unwrap();
        let default_input = pa.default_input_device().unwrap();
        let mut state = AppState::new(default_output, default_input);
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            num_devices: device_count,
            port_audio: pa,
            state,
        }

    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        // }


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
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("ahoaho").clicked() {
                            println!("ahoaho");
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Nanka eekanji no yatsu");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }
            ui.horizontal(|ui| {
               ui.label("Device Count: ");
                ui.label(self.num_devices.to_string())
            });
            egui::ComboBox::from_label("Select Output Device")
                .selected_text(self.port_audio.device_info(self.state.output_device).unwrap().name)
                .show_ui(ui, |ui|{
                   ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    for index in 0..self.num_devices {
                        let device = self.port_audio.device_info(DeviceIndex(index)).unwrap();
                        if device.max_output_channels != 0 {
                            ui.selectable_value(&mut self.state.output_device, DeviceIndex(index), device.name);
                        }

                    }
                });
            egui::ComboBox::from_label("Select Input Device")
                .selected_text(self.port_audio.device_info(self.state.input_device).unwrap().name)
                .show_ui(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    for index in 0..self.num_devices {
                        let device = self.port_audio.device_info(DeviceIndex(index)).unwrap();
                        if (device.max_input_channels != 0) {
                            ui.selectable_value(&mut self.state.output_device, DeviceIndex(index), device.name);
                        }
                    }
                });
            if ui.button("Toggle").clicked(){
                if (!self.state.processing){
                    self.state.stream_start()
                } else {
                    self.state.stream_stop()
                }
                self.state.processing = !self.state.processing;
            }
            if ui.button("get").clicked(){
                let samples = self.state.stream_manager.receiver.try_recv().unwrap();
                let hann_window = hann_window(&samples[0..2048]);
                let spectrum_hann_window = samples_fft_to_spectrum(
                    &hann_window,
                    44100,
                    FrequencyLimit::All,
                    Some(&divide_by_N_sqrt),
                ).unwrap();
                for (fr,fr_val) in spectrum_hann_window.data().iter() {
                    println!("{}Hz => {}",fr,fr_val);
                }
            }

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/master/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}