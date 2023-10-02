mod measurements;

use crate::measurements::MeasurementWindow;
use eframe::Theme;
use eframe::egui;
use egui::Align2;
use egui_plot::{Legend, Line, Plot};
use eframe::egui::{Style, Visuals};

use std::num::ParseFloatError;
use std::sync::*;
use std::thread;
use std::time::Duration;
use std::io;
use std::collections::VecDeque;
use struct_iterable::Iterable;

struct Parser{}

impl Parser {
    fn parse_float(buffer: &[u8]) -> Result<f32, ParseFloatError>
    {
        // f32::from_str(buffer)
        std::str::from_utf8(buffer).unwrap().trim().parse::<f32>()
    }

    fn parse_int(buffer: &[u8]) -> i32 {
        let string = std::str::from_utf8(buffer).unwrap();
        string.trim().parse().unwrap()
    }

    fn parse_string(buffer: &[u8]) -> String {
        String::from_utf8_lossy(buffer).to_string()
    }
}

fn serial_listener(blackboard: &Blackboard)
{
    let delay_between_rereads = 10;
    println!("Serial port:");
    let ports = serialport::available_ports().expect("No ports found!");
    for p in &ports {
        println!("{}", p.port_name);
    }

    let port_name = ports[0].port_name.clone();
    let mut port = serialport::new(port_name, 115_200)
    .timeout(Duration::from_millis(10))
    .open().expect("Failed to open port");

    let mut incoming_stream: VecDeque<u8> = VecDeque::with_capacity(256);
    let mut ctr = 0;
    loop {
        // serial_buf may contain less or more than one whole msg (i.e. chars delimited by \n)
        let delimiter = b'\n'; // Change this to the delimiter character(s) you're using
        let delimiter_header = ':'; // Change this to the delimiter character(s) you're using
        let mut read_buf: [u8; 256] = [0; 256];
        // let mut leftover_buf: Vec<u8> = vec![0; 256];
        // let mut remaining_len = 0;
        match port.read(&mut read_buf){
            Ok(bits_read) =>{
                // println!("DBG: Bits read: {}", bits_read);
                incoming_stream.extend(&read_buf[..bits_read]);
            },
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (
                thread::sleep(Duration::from_millis(delay_between_rereads))
            ),
            Err(_e) => {},
        }
        // Process queue to see if there are any complete messages (end with \n)
        // If it does, remove those elements from the queue and process them
        // If it doesn't, continue reading from the serial port and appending to queue
        // let mut message;
        if read_buf.contains(&delimiter)
        {
            while let Some(index_end) = incoming_stream.iter().position(|&c| c == delimiter) {
                let message = incoming_stream.drain(..index_end + 1).map(|c| c as char).into_iter().collect::<Vec<char>>();//collect::<Vec<u8>>();

                if let Some(index_header_end) = message.iter().position(|&c| c == delimiter_header){
                    let header = message[..index_header_end].into_iter().collect::<String>();
                    let data = message[index_header_end+1..].into_iter().map(|c| *c as u8).into_iter().collect::<Vec<u8>>();

                    if header == blackboard.bus_voltage.0 {
                        if let Ok(f) = Parser::parse_float(&data)
                        {
                            blackboard.bus_voltage.1.lock().unwrap().add(measurements::Measurement::new(ctr as f64, f as f64));
                            ctr = ctr + 1;
                        }
                    }
                    if header == blackboard.encoder_position.0 {
                        if let Ok(f) = Parser::parse_float(&data)
                        {
                            blackboard.encoder_position.1.lock().unwrap().add(measurements::Measurement::new(ctr as f64, f as f64));
                            ctr = ctr + 1;
                        }
                    }
                    if header == blackboard.encoder_velocity.0 {
                        if let Ok(f) = Parser::parse_float(&data)
                        {
                            blackboard.encoder_velocity.1.lock().unwrap().add(measurements::Measurement::new(ctr as f64, f as f64));
                            ctr = ctr + 1;
                        }
                    }
                    
                    // if header == blackboard.dbg_msg.0 {
                    //     let mut queue = blackboard.dbg_msg.1.lock().unwrap();
                    //     queue.push_back(Parser::parse_string(&data));
                    // }
                    
                } else {
                    println!("ERROR: No header found");
                }
            }
        }
    }
}

// type BlackboardRow<T> = (String, Arc<Mutex<VecDeque<T>>>);
type BlackboardRow = (String, Arc<Mutex<MeasurementWindow>>);

#[derive(Iterable)]
struct Blackboard
{
    bus_voltage: BlackboardRow,
    encoder_position: BlackboardRow,
    encoder_velocity: BlackboardRow,
    // dbg_msg: BlackboardRow<String>,
}

pub struct EncoderPositionsPlot
{
    paused: bool,
    pause_cache: MeasurementWindow,

    show_axis0: bool,
    show_axis1: bool,
    show_axis2: bool,
    show_axis3: bool,
}

impl EncoderPositionsPlot
{
    fn new(look_behind: usize) -> Self
    {
        Self
        {
            paused: false,
            pause_cache: MeasurementWindow::new_with_look_behind(look_behind),
            show_axis0: true,
            show_axis1: true,
            show_axis2: true,
            show_axis3: true,
        }
    }
}

pub struct EncoderVelocitiesPlot
{
    paused: bool,
    pause_cache: egui_plot::PlotPoint,

    show_axis0: bool,
    show_axis1: bool,
    show_axis2: bool,
    show_axis3: bool,
}

impl EncoderVelocitiesPlot
{
    fn new(look_behind: usize) -> Self
    {
        Self
        {
            paused: false,
            pause_cache: egui_plot::PlotPoint::new(0.0, 0.0),
            show_axis0: true,
            show_axis1: true,
            show_axis2: true,
            show_axis3: true,
        }
    }
}

pub struct AppState
{
    com_port: String,
    encoder_positions: EncoderPositionsPlot,
    encoder_velocities: EncoderVelocitiesPlot,
}

impl AppState
{
    pub fn new(look_behind: usize) -> Self
    {
        Self
        {
            com_port: String::from("#"),
            encoder_positions: EncoderPositionsPlot::new(look_behind),
            encoder_velocities: EncoderVelocitiesPlot::new(look_behind),
        }
    }
}

pub struct MonitorApp {
    include_y: Vec<f64>,

    window_size: usize,

    // Buffers used by the listing thread to store the incoming data
    bus_voltage: Arc<Mutex<MeasurementWindow>>,
    encoder_position: Arc<Mutex<MeasurementWindow>>,
    encoder_velocity: Arc<Mutex<MeasurementWindow>>,


    app_state: AppState,
}

impl MonitorApp {
    fn new(look_behind: usize) -> Self {
        Self {
            bus_voltage: Arc::new(Mutex::new(MeasurementWindow::new_with_look_behind(look_behind,))),
            encoder_position: Arc::new(Mutex::new(MeasurementWindow::new_with_look_behind(look_behind,))),
            encoder_velocity: Arc::new(Mutex::new(MeasurementWindow::new_with_look_behind(look_behind,))),
            include_y: Vec::new(),
            app_state: AppState::new(look_behind),

            window_size: look_behind,
        }
    }
}

impl eframe::App for MonitorApp {
    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // println!("here.");
        // egui::CentralPanel::default();

        let plot_height = 250.0;
        let plot_width = 500.0;
        let padding = 40.0;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("Mission Control");

            ui.horizontal(|ui| {
                ui.label(format!("Serial port: COM{}", self.app_state.com_port));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            egui::Window::new("Bus Voltage")
            .default_width(plot_width)
            .default_height(plot_height)
            .collapsible(false)
            .anchor(Align2::LEFT_TOP, [0.0,0.0])
            .show(ctx, |ui: &mut egui::Ui| {
                let mut plot = Plot::new("bus_voltage").legend(Legend::default());
                    let include_y_bus_voltage = vec![14.0,16.0];
                    for y in include_y_bus_voltage.iter() {
                        plot = plot.include_y(*y);
                    }

                plot.show(ui, |plot_ui| {
                    plot_ui.line(Line::new(
                        self.bus_voltage.lock().unwrap().plot_values(),
                    ));
                });
                
            });

            egui::Window::new("Encoder Position")
            .default_width(plot_width)
            .default_height(plot_height)
            .collapsible(false)
            .drag_to_scroll(false)
            .anchor(Align2::LEFT_TOP, [0.0,plot_height+padding])
            .show(ctx, |ui| {

                // Checkboxes to select which axes get plotted 
                ui.horizontal(|ui| {

                    ui.checkbox(&mut self.app_state.encoder_positions.show_axis0, "Show Axis 0")
                        .on_hover_text("Uncheck to hide all the widgets.");

                    if ui.button("Pause").on_hover_text("Pause the plot.").clicked() {
                        self.app_state.encoder_positions.paused = !self.app_state.encoder_positions.paused;
                        self.app_state.encoder_positions.pause_cache = MeasurementWindow{
                            values: self.encoder_position.lock().unwrap().values.clone(),
                            window_size: 0,
                        }
                    };
                });
                ui.separator();

                // Encoder positions plots
                let mut encoder_positions_plot = Plot::new("encoder_position");
                for y in self.include_y.iter() {
                    encoder_positions_plot = encoder_positions_plot.include_y(*y);
                }

                encoder_positions_plot.show(ui, |plot_ui| {
                    if self.app_state.encoder_positions.show_axis0{
                        if !self.app_state.encoder_positions.paused {
                            plot_ui.line(Line::new(self.encoder_position.lock().unwrap().plot_values()));
                        } else {
                            // todo!("Plot the self.app_state.encoder_positions.pause_cache");
                            plot_ui.line(Line::new(self.app_state.encoder_positions.pause_cache.plot_values()));
                        }
                    }
                });
            });


            egui::Window::new("Encoder Velocity")
                .default_width(plot_width)
                .default_height(plot_height)
                .collapsible(false)
                .anchor(Align2::LEFT_TOP, [0.0,2.0*(plot_height+padding)])
                .show(ctx, |ui: &mut egui::Ui| {

                    // Encoder velocities plots
                    let mut encoder_velocities_plot = Plot::new("encoder_velocities");
                    for y in self.include_y.iter() {
                        encoder_velocities_plot = encoder_velocities_plot.include_y(*y);
                    }

                    encoder_velocities_plot.show(ui, |plot_ui| {
                        if self.app_state.encoder_velocities.show_axis0{
                            plot_ui.line(Line::new(self.encoder_velocity.lock().unwrap().plot_values()));
                        }
                    });
                });

        });

        ctx.request_repaint();
    }
}

fn main() {
    let mut app = MonitorApp::new(100);
    app.include_y.push(-5.0);
    app.include_y.push(5.0);

    let bus_voltage_thread = app.bus_voltage.clone();
    let encoder_position_thread = app.encoder_position.clone();
    let encoder_velocity_thread = app.encoder_velocity.clone();
    
    let _handle = thread::spawn({
        move || {

            let blackboard = Blackboard{
                bus_voltage: (String::from("bus_voltage"), bus_voltage_thread),
                encoder_position: (String::from("encoder_position"), encoder_position_thread),
                encoder_velocity: (String::from("encoder_velocity"), encoder_velocity_thread),
            };

            // Inf loop, does not return
            serial_listener(&blackboard);
        }
    });

    let mut native_options = eframe::NativeOptions::default();
    native_options.maximized = true;
    native_options.decorated = false;
    native_options.default_theme = Theme::Dark;
    
    let _ = eframe::run_native("Monitor app", native_options, Box::new(
        |creation_context| {
            let style = Style {
                visuals: Visuals::dark(),
                ..Style::default()
            };
        creation_context.egui_ctx.set_style(style);
        Box::new(app)}));

    // println!("In main");
    // let mut ctr = 0;
    // loop{
    //     let mut bus_voltage_lock = bus_voltage_queue.try_lock();
    //     if let Ok(ref mut queue) = bus_voltage_lock{
    //         if let Some(m) = queue.pop_front(){
    //             println!("Message from bus_voltage_queue #{}: {:.3}", ctr, m);
    //             ctr = ctr+1;
    //         }
    //     }

    //     let mut dbg_msg_lock = bus_voltage_queue.try_lock();
    //     if let Ok(ref mut queue) = dbg_msg_lock{
    //         if let Some(m) = queue.pop_front(){
    //             println!("Message from dbg_msg_lock_queue: {}", m);
    //         }
    //     }

    //     thread::sleep(Duration::from_millis(100));
    // }
}