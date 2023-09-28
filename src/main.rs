mod measurements;

use crate::measurements::MeasurementWindow;
use eframe::egui;

use std::io::BufRead;
use std::sync::*;
use std::thread;
use tracing::{error, info, warn};
use std::time::Duration;
use std::io;
use std::collections::VecDeque;
use std::fmt;

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

#[derive(Debug)]
enum MessageDType {
    Float,
    Int,
    String,
}
impl fmt::Display for MessageDType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self{
            MessageDType::Float => "Float",
            MessageDType::Int => "Int",
            MessageDType::String => "String",
        })
    }
}

#[derive(Debug)]
struct Message{
    header: String,
    dtype: MessageDType,
    data: Vec<u8>,
}

impl Message{
    fn new() -> Self {
        Message{
            header: "".to_string(),
            dtype: MessageDType::String,
            data: Vec::new(),
        }
    }
    fn dtype(d: char) -> MessageDType {
        match d {
            'f' => MessageDType::Float,
            'i' => MessageDType::Int,
            's' => MessageDType::String,
            _ => MessageDType::String,
        }
    }
    fn parse_float(chars: &[char]) -> Option<f32> {
        let s: String = chars.iter().collect();
        s.parse().ok()
    }
    
    fn parse_int(chars: &[char]) -> Option<i32> {
        let s: String = chars.iter().collect();
        s.parse().ok()
    }
    
    fn parse_string(chars: &[char]) -> String {
        chars.iter().collect()
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} : {} : {:?}", self.header, self.dtype, self.data)
    }
}
struct MessageQueue {
    incoming_stream: Arc<Mutex<VecDeque<Message>>>,
}


fn serial_listener(message_queue: Arc<Mutex<VecDeque<Message>>>)
{
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
            Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
            Err(_e) => {},
        }
        // Process queue to see if there are any complete messages (end with \n)
        // If it does, remove those elements from the queue and process them
        // If it doesn't, continue reading from the serial port and appending to queue
        // let mut message;
        if read_buf.contains(&delimiter)
        {
            while let Some(index_end) = incoming_stream.iter().position(|&c| c == delimiter) {
                let mut message = incoming_stream.drain(..index_end + 1).map(|c| c as char).into_iter().collect::<Vec<char>>();//collect::<Vec<u8>>();

                // while let Some(index_header_end) = message.iter().position(|&c| c == delimiter_header) {
                if let Some(index_header_end) = message.iter().position(|&c| c == delimiter_header){
                    let header = message[..index_header_end + 1].into_iter().collect::<String>();
                    let dtype = message[index_header_end+1];
                    let data = message[index_header_end+1..].into_iter().map(|c| *c as u8).into_iter().collect::<Vec<u8>>();
                    let m = Message{header: header, dtype: Message::dtype(dtype), data: data};
                    {
                        let mut queue = message_queue.lock().unwrap();
                        queue.push_back(m);
                    }
                } else {
                    println!("ERROR: No header found");
                }
            }
        }
    }
}


fn main() {

    let message_queue = Arc::new(Mutex::new(VecDeque::new()));

    let handle = thread::spawn({
        let message_queue = message_queue.clone();
        move || {
            // Inf loop, does not return
            serial_listener(message_queue);
        }
    });

    println!("In main");
    loop{
        match message_queue.lock().unwrap().pop_front() {
            Some(m) => println!("Message from main: {}", m),
            None => continue,
        };

        thread::sleep(Duration::from_millis(100));
    }
    // let app = MonitorApp::new(1000);
    // let native_options = eframe::NativeOptions::default();
    // eframe::run_native("Monitor app", native_options, Box::new(|_| Box::new(app)));
}