mod measurements;

use crate::measurements::MeasurementWindow;
use eframe::egui;

use std::num::ParseFloatError;
use std::sync::*;
use std::thread;
use std::time::Duration;
use std::io;
use std::collections::VecDeque;
use struct_iterable::Iterable;

pub struct MonitorApp {
    include_y: Vec<f64>,
    measurements: Arc<Mutex<MeasurementWindow>>,
}

impl MonitorApp {
    fn new(look_behind: usize) -> Self {
        Self {
            measurements: Arc::new(Mutex::new(MeasurementWindow::new_with_look_behind(
                look_behind,
            ))),
            include_y: Vec::new(),
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
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut plot = egui::plot::Plot::new("measurements");
            for y in self.include_y.iter() {
                plot = plot.include_y(*y);
            }

            plot.show(ui, |plot_ui| {
                plot_ui.line(egui::plot::Line::new(
                    self.measurements.lock().unwrap().plot_values(),
                ));
            });
        });
        // make it always repaint. TODO: can we slow down here?
        ctx.request_repaint();
    }
}

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

                    // println!("Header: {}", header);
                    // println!("msg: {:?}", message);
                    // println!("data: {:?}", data);
                    // println!("index_header_end: {:?}", index_header_end);

                    if header == blackboard.bus_voltage.0 {
                        if let Ok(f) = Parser::parse_float(&data)
                        {
                            let mut queue = blackboard.bus_voltage.1.lock().unwrap();
                            queue.push_back(f);
                        }
                    }
                    if header == blackboard.dbg_msg.0 {
                        let mut queue = blackboard.dbg_msg.1.lock().unwrap();
                        queue.push_back(Parser::parse_string(&data));
                    }
                    
                } else {
                    println!("ERROR: No header found");
                }
            }
        }
    }
}

type BlackboardRow<T> = (String, Arc<Mutex<VecDeque<T>>>);

#[derive(Iterable)]
struct Blackboard
{
    bus_voltage: BlackboardRow<f32>,
    dbg_msg: BlackboardRow<String>,
}

fn main() {
    let bus_voltage_queue = Arc::new(Mutex::new(VecDeque::<f32>::new()));
    let dbg_msg_queue = Arc::new(Mutex::new(VecDeque::<String>::new()));


    let bus_voltage_queue_thread = bus_voltage_queue.clone();
    let dbg_msg_queue_thread = dbg_msg_queue.clone();

    
    let _handle = thread::spawn({
        move || {

            let blackboard = Blackboard{
                bus_voltage: (String::from("bus_voltage"), bus_voltage_queue_thread),
                dbg_msg: (String::from("dbg_msg"), dbg_msg_queue_thread),
            };

            // Inf loop, does not return
            serial_listener(&blackboard);
        }
    });

    println!("In main");
    let mut ctr = 0;
    loop{
        let mut bus_voltage_lock = bus_voltage_queue.try_lock();
        if let Ok(ref mut queue) = bus_voltage_lock{
            if let Some(m) = queue.pop_front(){
                println!("Message from bus_voltage_queue #{}: {:.3}", ctr, m);
                ctr = ctr+1;
            }
        }

        let mut dbg_msg_lock = bus_voltage_queue.try_lock();
        if let Ok(ref mut queue) = dbg_msg_lock{
            if let Some(m) = queue.pop_front(){
                println!("Message from dbg_msg_lock_queue: {}", m);
            }
        }

        thread::sleep(Duration::from_millis(100));
    }
    let app = MonitorApp::new(1000);
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("Monitor app", native_options, Box::new(|_| Box::new(app)));
}