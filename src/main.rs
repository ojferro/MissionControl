mod measurements;

use crate::measurements::MeasurementWindow;
use eframe::Theme;
use eframe::egui;
use egui::Align2;
use crossbeam_channel;
// use egui::TextBuffer;
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

fn serial_listener(senders: SenderBlackboard)
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

    // serial_buf may contain less or more than one whole msg (i.e. chars delimited by \n)
    let delimiter = b'\n'; // Change this to the delimiter character(s) you're using
    let delimiter_header = ':'; // Change this to the delimiter character(s) you're using

    loop {
        let mut read_buf: [u8; 256] = [0; 256];

        if let Ok(msg) = senders.echo_channel.1.try_recv() {
            port.write(msg.as_bytes()).expect("Failed to write to serial port");
        }

        match port.read(&mut read_buf){
            Ok(bits_read) =>{
                // println!("DBG: Bits read: {}", bits_read);
                incoming_stream.extend(&read_buf[..bits_read]);
            },
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut =>{
                thread::sleep(Duration::from_millis(delay_between_rereads))
            },
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

                    if header == senders.bus_voltage.0 {
                        if let Ok(f) = Parser::parse_float(&data)
                        {
                            // println!("Bus voltage: {}", f);

                            // This will not send if the channel is at capacity.
                            // Queueing should happen on the AppState side if desired. This will always send the latest value.
                            let _ = senders.bus_voltage.1.try_send(measurements::Measurement::new(ctr as f64, f as f64));
                            ctr = ctr + 1;
                        }
                    }
                    if header == senders.encoder_position.0 {
                        if let Ok(f) = Parser::parse_float(&data)
                        {
                            // println!("encoder pos: {}", f);
                            // This will not send if the channel is at capacity.
                            // Queueing should happen on the AppState side if desired. This will always send the latest value.
                            let _ = senders.encoder_position.1.try_send(measurements::Measurement::new(ctr as f64, f as f64));
                            ctr = ctr + 1;
                        }
                    }
                    if header == senders.encoder_velocity.0 {
                        if let Ok(f) = Parser::parse_float(&data)
                        {
                            // This will not send if the channel is at capacity.
                            // Queueing should happen on the AppState side if desired. This will always send the latest value.
                            let _ = senders.encoder_velocity.1.try_send(measurements::Measurement::new(ctr as f64, f as f64));
                            ctr = ctr + 1;
                        }
                    }
                    
                    if header == senders.dbg_msgs.0 {
                        let s = Parser::parse_string(&data);
                        println!("dbg_msg:{}", s);
                        let _ = senders.dbg_msgs.1.try_send(s);
                    }
                    
                }
                // else {
                    // println!("ERROR: No header found");
                // }
            }
        }
    }
}

pub struct MeasurementPlot
{
    paused: bool,
    measurements: MeasurementWindow,
}

impl MeasurementPlot
{
    fn new(look_behind: usize) -> Self
    {
        Self
        {
            paused: false,
            measurements: MeasurementWindow::new_with_look_behind(look_behind),
        }
    }
}

pub struct AppState
{
    com_port: String,

    bus_voltages: MeasurementPlot,
    encoder_positions: MeasurementPlot,
    encoder_velocities: MeasurementPlot,
    dbg_msgs: VecDeque<String>,

    axis_state: Buttons,
    controller_setpoint: f32,
}

impl AppState
{
    pub fn new(look_behind: usize) -> Self
    {
        Self
        {
            com_port: String::from("#"),

            bus_voltages: MeasurementPlot::new(look_behind),
            encoder_positions: MeasurementPlot::new(look_behind),
            encoder_velocities: MeasurementPlot::new(look_behind),
            dbg_msgs: VecDeque::<String>::new(),

            axis_state: Buttons::PositionCtrl,
            controller_setpoint: 0.0,
        }
    }
}

// type MsgType = String;

pub struct MonitorApp {
    include_y: Vec<f64>,
    window_size: usize,

    app_state: AppState,
    receivers: ReceiverBlackboard,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Buttons {
    PositionCtrl,
    VelocityCtrl,
    VoltageCtrl,
    TorqueCtrl
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
        let debug_msgs_width = 800.0;
        let debug_msgs_height = 250.0;
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
                    let include_y_bus_voltage = vec![0.0,24.0];
                    for y in include_y_bus_voltage.iter() {
                        plot = plot.include_y(*y);
                    }

                if let Ok(mut bus_voltage) = self.receivers.bus_voltage.1.try_recv()
                {
                    self.app_state.bus_voltages.measurements.add(bus_voltage);
                }

                plot.show(ui, |plot_ui| {
                    plot_ui.line(Line::new(
                        self.app_state.bus_voltages.measurements.plot_values()
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

                    // ui.checkbox(&mut self.app_state.encoder_positions.show_axis0, "Show Axis 0")
                    //     .on_hover_text("Uncheck to hide all the widgets.");

                    if ui.button("Pause").on_hover_text("Pause the plot.").clicked() {
                        self.app_state.encoder_positions.paused = !self.app_state.encoder_positions.paused;
                    };
                });
                ui.separator();

                // Encoder positions plots
                let mut encoder_positions_plot = Plot::new("encoder_position");
                for y in self.include_y.iter() {
                    encoder_positions_plot = encoder_positions_plot.include_y(*y);
                }
                
                // Only update the values if not paused
                if !self.app_state.encoder_positions.paused {
                    if let Ok(mut encoder_position) = self.receivers.encoder_position.1.try_recv()
                    {
                        self.app_state.encoder_positions.measurements.add(encoder_position);
                    }
                }

                encoder_positions_plot.show(ui, |plot_ui| {
                    plot_ui.line(Line::new(self.app_state.encoder_positions.measurements.plot_values()));
                });
            });


            egui::Window::new("Encoder Velocity")
                .default_width(plot_width)
                .default_height(plot_height)
                .collapsible(false)
                .anchor(Align2::LEFT_TOP, [0.0,2.0*(plot_height+padding)])
                .show(ctx, |ui: &mut egui::Ui| {

                    // Checkboxes to select which axes get plotted 
                    ui.horizontal(|ui| {

                        // ui.checkbox(&mut self.app_state.encoder_positions.show_axis0, "Show Axis 0")
                        //     .on_hover_text("Uncheck to hide all the widgets.");

                        if ui.button("Pause").on_hover_text("Pause the plot.").clicked() {
                            self.app_state.encoder_velocities.paused = !self.app_state.encoder_velocities.paused;
                        };
                    });
                    ui.separator();

                    // Only update if not paused
                    if !self.app_state.encoder_velocities.paused {
                        if let Ok(mut encoder_velocity) = self.receivers.encoder_velocity.1.try_recv()
                        {
                            self.app_state.encoder_velocities.measurements.add(encoder_velocity);
                        }
                    }

                    // Encoder velocities plots
                    let mut encoder_velocities_plot = Plot::new("encoder_velocities");
                    for y in self.include_y.iter() {
                        encoder_velocities_plot = encoder_velocities_plot.include_y(*y);
                    }

                    encoder_velocities_plot.show(ui, |plot_ui| {
                        plot_ui.line(Line::new(self.app_state.encoder_velocities.measurements.plot_values()));
                    });
                });

            egui::Window::new("Debug Messages")
                .default_width(debug_msgs_width)
                .default_height(debug_msgs_height*2.0)
                .collapsible(false)
                .anchor(Align2::RIGHT_BOTTOM, [0.0,0.0])
                .show(ctx, |ui: &mut egui::Ui| {
                    
                    ui.horizontal(|ui| {
                        if ui.button("ODrv Calibration").clicked() {
                            // self.master_msgs.lock().unwrap().push_back(String::from("dbg_msg:calibrate"));
                            // todo!("Calibrate not implemented yet");

                            // This will block. It's important that all msgs are processed. Use blocking send sparingly
                            let _ = self.receivers.echo_channel.1.send(String::from("calib_rtn"));
                        };
                        if ui.button("ODrv Closed Loop Ctrl").clicked() {

                            // todo!("Calibrate not implemented yet");
                            // self.master_msgs.lock().unwrap().push_back(String::from("dbg_msg:closed_loop_ctrl"));
                            if &mut self.app_state.axis_state == &mut Buttons::PositionCtrl{
                                let _ = self.receivers.echo_channel.1.send(String::from("posn_ctrl"));
                            } else if  &mut self.app_state.axis_state == &mut Buttons::VelocityCtrl{
                                let _ = self.receivers.echo_channel.1.send(String::from("velo_ctrl"));
                            }
                            else if  &mut self.app_state.axis_state == &mut Buttons::TorqueCtrl{
                                let _ = self.receivers.echo_channel.1.send(String::from("torq_ctrl"));
                            }
                            else if  &mut self.app_state.axis_state == &mut Buttons::VoltageCtrl{
                                let _ = self.receivers.echo_channel.1.send(String::from("volt_ctrl"));
                            }

                            let _ = self.receivers.echo_channel.1.send(String::from("ClLp_ctrl"));
                        };
                        if ui.button("ODrv Idle").clicked() {
                            // todo!("Calibrate not implemented yet");
                            // self.master_msgs.lock().unwrap().push_back(String::from("dbg_msg:idle_ctrl"));
                            let _ = self.receivers.echo_channel.1.send(String::from("idle_ctrl"));
                        };
                    });
                    
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.app_state.axis_state, Buttons::PositionCtrl, "Position Ctrl");
                        ui.selectable_value(&mut self.app_state.axis_state, Buttons::VelocityCtrl, "Velocity Ctrl");
                        ui.selectable_value(&mut self.app_state.axis_state, Buttons::TorqueCtrl, "Torque Ctrl");
                        ui.selectable_value(&mut self.app_state.axis_state, Buttons::VoltageCtrl, "Voltage Ctrl");
                    });
                    ui.end_row();

                    ui.separator();

                    let mut new_setpoint = self.app_state.controller_setpoint;
                    ui.add(egui::Slider::new(&mut new_setpoint, -10.0..=10.0).text("Controller Setpoint"));
                    
                    // If value changed, send it to the ODrive
                    if new_setpoint != self.app_state.controller_setpoint
                    {
                        self.app_state.controller_setpoint = new_setpoint;
                        let MAX_MSG_LEN = 9; // TODO: Make configurable, later
                        let msg = format!("sp:{:.4}", self.app_state.controller_setpoint)[..MAX_MSG_LEN].to_string();
                        let _ = self.receivers.echo_channel.1.send(msg);
                    }

                    ui.separator();
                    
                    // Get any new msgs
                    if let Ok(dbg_msg) = self.receivers.dbg_msgs.1.try_recv()
                    {
                        let MAX_LEN = 10; // TODO: Make this configurable in the AppState
                        self.app_state.dbg_msgs.push_back(dbg_msg);
                        if self.app_state.dbg_msgs.len() > MAX_LEN {
                            self.app_state.dbg_msgs.pop_front();
                        }
                    }
                    
                    let mut display_string = self.app_state.dbg_msgs.iter().fold(String::new(), |acc, s| acc + s + "\n");
                    ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut display_string))
                });

        });

        ctx.request_repaint();
    }
}

// All rows consist of a name and a Sender
type MeasurementRowTx = (String, crossbeam_channel::Sender<measurements::Measurement>);
type MsgRowTx = (String, crossbeam_channel::Sender<String>);

type MeasurementRowRx = (String, crossbeam_channel::Receiver<measurements::Measurement>);
type MsgRowRx = (String, crossbeam_channel::Receiver<String>);

struct SenderBlackboard
{
    bus_voltage: MeasurementRowTx,
    encoder_position: MeasurementRowTx,
    encoder_velocity: MeasurementRowTx,
    dbg_msgs : MsgRowTx,

    echo_channel: MsgRowRx,
}

struct ReceiverBlackboard
{
    bus_voltage: MeasurementRowRx,
    encoder_position: MeasurementRowRx,
    encoder_velocity: MeasurementRowRx,
    dbg_msgs : MsgRowRx,

    echo_channel: MsgRowTx,
}

fn main() {

    let sender_queue_capacity = 2;

    let (bus_voltage_s, bus_voltage_r) = crossbeam_channel::bounded(sender_queue_capacity);
    let (encoder_position_s, encoder_position_r) = crossbeam_channel::bounded(sender_queue_capacity);
    let (encoder_velocity_s, encoder_velocity_r) = crossbeam_channel::bounded(sender_queue_capacity);
    let (dbg_msgs_s, dbg_msgs_r) = crossbeam_channel::bounded(sender_queue_capacity);
    let (echo_channel_s, echo_channel_r) = crossbeam_channel::bounded(sender_queue_capacity);


    let mut app = MonitorApp {
        include_y: Vec::new(),
        window_size: 100,
        app_state: AppState::new(100),

        receivers: ReceiverBlackboard{
            bus_voltage: ("bus_voltage".to_string(), bus_voltage_r),
            encoder_position: ("encoder_position".to_string(), encoder_position_r),
            encoder_velocity: ("encoder_velocity".to_string(), encoder_velocity_r),
            dbg_msgs: ("dbg_msg".to_string(), dbg_msgs_r),

            echo_channel: ("echo_channel".to_string(), echo_channel_s),
        },
    };
    

    app.include_y.push(-5.0);
    app.include_y.push(5.0);
    
    {
        thread::spawn({
            move || {
                
                let senders = SenderBlackboard{
                    bus_voltage: ("bus_voltage".to_string(), bus_voltage_s),
                    encoder_position: ("encoder_position".to_string(), encoder_position_s),
                    encoder_velocity: ("encoder_velocity".to_string(), encoder_velocity_s),
                    dbg_msgs: ("dbg_msg".to_string(), dbg_msgs_s),
                    echo_channel: ("echo_channel".to_string(), echo_channel_r),
                };

                // Inf loop, does not return
                serial_listener(senders);
            }
        });
    }

    let mut native_options = eframe::NativeOptions::default();
    native_options.maximized = true;
    native_options.decorated = false;
    native_options.default_theme = Theme::Dark;
    native_options.hardware_acceleration = eframe::HardwareAcceleration::Preferred;

    let _ = eframe::run_native("Mission Control", native_options, Box::new(
        |creation_context| {
            let style = Style {
                visuals: Visuals::dark(),
                ..Style::default()
            };
        creation_context.egui_ctx.set_style(style);
        Box::new(app)}));

    
}