use rerun;
use std::time::{Duration, SystemTime};
use std::io;
use std::collections::VecDeque;
use std::thread;
use std::num::ParseFloatError;
use crossbeam_channel;

// System command sender
use eframe::Theme;
use eframe::egui;
use eframe::egui::{Style, Visuals};

struct Parser{}

impl Parser {
    fn parse_float(buffer: &[u8]) -> Result<f32, ParseFloatError>
    {
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

fn serial_listener(cmds_to_dispatch_r: crossbeam_channel::Receiver<String>, dbg_msgs_s: crossbeam_channel::Sender<String>) -> Result<(), Box<dyn std::error::Error>>
{
    let opts = rerun::SpawnOptions {
        memory_limit: "10%".into(),
        ..Default::default()
    };
    let rec = rerun::RecordingStreamBuilder::new("sensor_stream_viewer").spawn_opts(&opts, None)?;

    let delay_between_rereads = 10; // In millis
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

    // serial_buf may contain less or more than one whole msg (i.e. chars delimited by \n)
    let delimiter = b'\n'; // Change this to the delimiter character(s) you're using
    let delimiter_header = ':'; // Change this to the delimiter character(s) you're using

    let mut quaternion :[f32; 4] = [0.0, 0.0, 0.0, 0.0];

    // To measure time offset for the rerun timeline
    let start_time = SystemTime::now();

    loop {
        let mut read_buf: [u8; 256] = [0; 256];

        match port.read(&mut read_buf){
            Ok(bits_read) =>{
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
        if read_buf.contains(&delimiter)
        {
            while let Some(index_end) = incoming_stream.iter().position(|&c| c == delimiter) {
                let message = incoming_stream.drain(..index_end + 1).map(|c| c as char).into_iter().collect::<Vec<char>>();//collect::<Vec<u8>>();

                if let Some(index_header_end) = message.iter().position(|&c| c == delimiter_header){
                    let header = message[..index_header_end].into_iter().collect::<String>();
                    let data = message[index_header_end+1..].into_iter().map(|c| *c as u8).into_iter().collect::<Vec<u8>>();

                    match header.as_str() {
                        "bus_voltage" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "bus/V",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("Voltage"),
                                );
                            }
                        },
                        "bus_current" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "bus/I",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("Current"),
                                );
                            }
                        },
                        "enc_pos_0" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "encoder_positions/0",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("Axis 0 position"),
                                );
                            }
                        },
                        "enc_vel_0" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "encoder_velocities/0",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("Axis 0 velocity"),
                                );
                            }
                        },
                        "enc_pos_1" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "encoder_positions/1",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("Axis 1 position"),
                                );
                            }
                        },
                        "enc_vel_1" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "encoder_velocities/1",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("Axis 1 velocity"),
                                );
                            }
                        },
                        "ctrl_u_0" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }
                                
                                let _ = rec.log(
                                    "ctrl_u/0",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("Axis 0 Control U"),
                                );
                            }
                        },
                        // "q_w" => {
                        //     if let Ok(f) = Parser::parse_float(&data)
                        //     {
                        //         rec.set_time_sequence("step", ctr);
                                
                        //         quaternion[0] = f;

                        //         rec.log(
                        //             "simple",
                        //             &rerun::Boxes3D::from_centers_and_half_sizes(
                        //                 [(0.0, 0.0, 0.0)],
                        //                 [(2.0, 2.0, 1.0)],
                        //             ).with_rotations([rerun::Quaternion::from_xyzw([quaternion[1], quaternion[2], quaternion[3], quaternion[0]])]),
                        //         )?;

                        //         ctr = ctr + 1;
                        //     }
                        // },
                        // "q_x" => {
                        //     if let Ok(f) = Parser::parse_float(&data)
                        //     {
                        //         rec.set_time_sequence("step", ctr);
                                
                        //         quaternion[1] = f;

                        //         rec.log(
                        //             "simple",
                        //             &rerun::Boxes3D::from_centers_and_half_sizes(
                        //                 [(0.0, 0.0, 0.0)],
                        //                 [(2.0, 2.0, 1.0)],
                        //             ).with_rotations([rerun::Quaternion::from_xyzw([quaternion[1], quaternion[2], quaternion[3], quaternion[0]])]),
                        //         )?;

                        //         ctr = ctr + 1;
                        //     }
                        // },
                        // "q_y" => {
                        //     if let Ok(f) = Parser::parse_float(&data)
                        //     {
                        //         rec.set_time_sequence("step", ctr);
                                
                        //         quaternion[2] = f;

                        //         rec.log(
                        //             "simple",
                        //             &rerun::Boxes3D::from_centers_and_half_sizes(
                        //                 [(0.0, 0.0, 0.0)],
                        //                 [(2.0, 2.0, 1.0)],
                        //             ).with_rotations([rerun::Quaternion::from_xyzw([quaternion[1], quaternion[2], quaternion[3], quaternion[0]])]),
                        //         )?;

                        //         ctr = ctr + 1;
                        //     }
                        // },
                        // "q_z" => {
                        //     if let Ok(f) = Parser::parse_float(&data)
                        //     {
                        //         rec.set_time_sequence("step", ctr);
                                
                        //         quaternion[3] = f;

                        //         rec.log(
                        //             "simple",
                        //             &rerun::Boxes3D::from_centers_and_half_sizes(
                        //                 [(0.0, 0.0, 0.0)],
                        //                 [(2.0, 2.0, 1.0)],
                        //             ).with_rotations([rerun::Quaternion::from_xyzw([quaternion[1], quaternion[2], quaternion[3], quaternion[0]])]),
                        //         )?;

                        //         ctr = ctr + 1;
                        //     }
                        // },
                        "x" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }
                                
                                let _ = rec.log(
                                    "state/x",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("state_x"),
                                );
                            }
                        },
                        "theta" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }
                                
                                let _ = rec.log(
                                    "state/theta",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("state_theta"),
                                );
                            }
                        },
                        "x_dot" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }
                                
                                let _ = rec.log(
                                    "state/x_dot",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("state_x_dot"),
                                );
                            }
                        },
                        "theta_dot" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }
                                
                                let _ = rec.log(
                                    "state/theta_dot",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("state_theta_dot"),
                                );
                            }
                        },


                        "imu_r" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }
                                
                                let _ = rec.log(
                                    "imu/roll",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("imu_roll"),
                                );
                            }
                        },
                        "imu_p" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }
                                
                                let _ = rec.log(
                                    "imu/pitch",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("imu_pitch"),
                                );
                            }
                        },
                        "imu_p_dot" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }
                                
                                let _ = rec.log(
                                    "imu/pitch_dot",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("imu_pitch_dot"),
                                );
                            }
                        },
                        "imu_y" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "imu/yaw",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("imu_yaw"),
                                );
                            }
                        },

                        "acc_x" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "accelerometer/X",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("acc_X"),
                                );
                            }
                        },
                        "acc_y" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "accelerometer/Y",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("acc_Y"),
                                );
                            }
                        },
                        "acc_z" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "accelerometer/Z",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("acc_Z"),
                                );
                            }
                        },
                        "gyr_x" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }

                                let _ = rec.log(
                                    "gyro/X",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("gyro_X"),
                                );
                            }
                        },
                        "gyr_y" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }
                                
                                let _ = rec.log(
                                    "gyro/Y",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("gyro_Y"),
                                );
                            }
                        },
                        "gyr_z" => {
                            if let Ok(f) = Parser::parse_float(&data)
                            {
                                match start_time.elapsed() {
                                    Ok(elapsed) => {
                                        rec.set_time_seconds("step", elapsed.as_secs_f32());
                                    }
                                    Err(e) => {
                                        println!("Could not calculate time offset: {e:?}");
                                        continue;
                                    }
                                }
                                
                                let _ = rec.log(
                                    "gyro/Z",
                                    &rerun::TimeSeriesScalar::new(f as f64)
                                    .with_label("gyro_Z"),
                                );
                            }
                        },
                        "dbg_msg" => {
                            let s = Parser::parse_string(&data);
                            println!("dbg_msg:{}", s);

                            // Hacky way to publish an array as a whole
                            if s.contains("quaternion,")
                            {
                                // Split the input string by ','
                                let parts: Vec<&str> = s.split(',').collect();

                                // Extract the string and floats
                                // let key = parts[0];
                                let floats: Vec<f32> = parts[1..].iter().filter_map(|&s| s.parse().ok()).collect();

                                // println!("s: {}", s);
                                // println!("parts: {:?}", parts);
                                // println!("floats: {:?}", floats.len());
                                quaternion[0] = floats[0];
                                quaternion[1] = floats[1];
                                quaternion[2] = floats[2];
                                quaternion[3] = floats[3];

                                // Publish to rerun
                                rec.log(
                                    "IMU_3D",
                                    &rerun::Boxes3D::from_centers_and_half_sizes(
                                        [(0.0, 0.0, 0.0)],
                                        [(1.0, 2.0, 2.0)],
                                    ).with_rotations([rerun::Quaternion::from_xyzw([quaternion[1], quaternion[2], quaternion[3], quaternion[0]])]),
                                )?;

                            }

                            let _ = dbg_msgs_s.try_send(s);
                        },
                        _ => {
                            let s = Parser::parse_string(&data);
                            println!("No match on serial! {}", s);
                            /* else case is no-op */
                        },
                    }
                    
                }
            }
        }


        // Check for commands to dispatch thru serial. Sent from the egui thread.
        if let Ok(command) = cmds_to_dispatch_r.try_recv() {
            port.write(command.as_bytes()).expect("Failed to write to serial port");
        }
    }
}


#[derive(Debug, PartialEq)]
enum ControlModes {
    PositionCtrl,
    VelocityCtrl,
    VoltageCtrl,
    TorqueCtrl
}

pub struct CommandDispatcherApp {
    dbg_msgs: VecDeque<String>,
    control_mode: ControlModes,
    controller_setpoint: f32,
    dbg_msg_channel_r: crossbeam_channel::Receiver<String>,
    dispatch_command_s: crossbeam_channel::Sender<String>,
}

impl eframe::App for CommandDispatcherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::CentralPanel::default()
            .show(ctx, |ui: &mut egui::Ui| {
                
                // ui.horizontal(|ui| {
                //     ui.selectable_value(&mut self.control_mode, ControlModes::PositionCtrl, "Position Ctrl");
                //     ui.selectable_value(&mut self.control_mode, ControlModes::VelocityCtrl, "Velocity Ctrl");
                //     ui.selectable_value(&mut self.control_mode, ControlModes::TorqueCtrl, "Torque Ctrl");
                //     ui.selectable_value(&mut self.control_mode, ControlModes::VoltageCtrl, "Voltage Ctrl");
                // });


                ui.horizontal(|ui| {
                    if ui.button("Calibration Rtn").clicked() {
                        let _ = self.dispatch_command_s.try_send(String::from("calib_rtn"));
                    };
                    if ui.button("Clear Errors").clicked() {
                        let _ = self.dispatch_command_s.try_send(String::from("clear_err"));
                    };
                });

                ui.end_row();
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Position Ctrl").clicked() {

                        self.control_mode = ControlModes::PositionCtrl;
                        let _ = self.dispatch_command_s.try_send(String::from("posn_ctrl"));
                    };
                    if ui.button("Velocity Ctrl").clicked() {

                        self.control_mode = ControlModes::VelocityCtrl;
                        let _ = self.dispatch_command_s.try_send(String::from("velo_ctrl"));
                    };
                    if ui.button("Torque Ctrl").clicked() {

                        self.control_mode = ControlModes::TorqueCtrl;
                        let _ = self.dispatch_command_s.try_send(String::from("torq_ctrl"));
                    };
                    if ui.button("Voltage Ctrl").clicked() {

                        self.control_mode = ControlModes::VoltageCtrl;
                        let _ = self.dispatch_command_s.try_send(String::from("volt_ctrl"));
                    };
                });

                ui.end_row();
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Idle").clicked() {
                        let _ = self.dispatch_command_s.try_send(String::from("idle_ctrl"));
                    };
                    if ui.button("Start Auto Control").clicked() {
                        let _ = self.dispatch_command_s.try_send(String::from("auto_ctrl"));
                    };
                });

                ui.end_row();
                ui.separator();

                ui.style_mut().spacing.slider_width = 500.0;

                let mut new_setpoint = self.controller_setpoint;
                ui.horizontal(|ui| {
                    if ui.button("Zero Setpoint").clicked() {
                        new_setpoint = 0.0;
                    }
                    ui.add(egui::Slider::new(&mut new_setpoint, -5.0..=5.0).text("Controller Setpoint"));
                });
                
                // If value changed, send it to the ODrive
                if new_setpoint != self.controller_setpoint
                {
                    self.controller_setpoint = new_setpoint;
                    let max_msg_len = 9; // TODO: Make configurable, later. This is a limitation of DMA Serial read on the Nucleo
                    let msg = format!("sp:{:.4}", self.controller_setpoint)[..max_msg_len].to_string();
                    let _ = self.dispatch_command_s.send(msg);
                }

                ui.separator();
                
                // Get any new dbg msgs from the other thread
                if let Ok(dbg_msg) = self.dbg_msg_channel_r.try_recv()
                {
                    let max_num_display_msgs = 10; // TODO: Make this configurable in the AppState
                    self.dbg_msgs.push_back(dbg_msg);
                    if self.dbg_msgs.len() > max_num_display_msgs {
                        self.dbg_msgs.pop_front();
                    }
                }
                
                // Debug message display
                if ui.button("   Clear All Messages   ").clicked() {
                    self.dbg_msgs.clear();
                }
                let mut display_string = self.dbg_msgs.iter().fold(String::new(), |acc, s| acc + s + "\n");
                ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut display_string));

                ui.add_space(10.0);
            });
    }
}

fn main() {

    
    let channel_capacity = 10;
    let (dispatch_command_s, dispatch_command_r) = crossbeam_channel::bounded::<String>(channel_capacity);
    let (dbg_msgs_s, dbg_msgs_r) = crossbeam_channel::bounded::<String>(channel_capacity);

    // Listen and parse serial stream, publish to rerun viewer
    thread::spawn(move || {
        let _ = serial_listener(dispatch_command_r, dbg_msgs_s);
    });


    let command_dispatcher_app = CommandDispatcherApp {
        dbg_msgs: VecDeque::<String>::new(),
        control_mode: ControlModes::PositionCtrl,
        controller_setpoint: 0.0,
        dbg_msg_channel_r: dbg_msgs_r,
        dispatch_command_s: dispatch_command_s,
    };
    // Egui app to send system commands
    let mut native_options = eframe::NativeOptions::default();
    native_options.maximized = false;
    native_options.decorated = true;
    native_options.default_theme = Theme::Dark;
    native_options.hardware_acceleration = eframe::HardwareAcceleration::Preferred;
    native_options.initial_window_size = Option::from(egui::Vec2::new(1000.0, 400.0));

    let _ = eframe::run_native("Mission Control", native_options, Box::new(
        |creation_context| {
            let style = Style {
                visuals: Visuals::dark(),
                ..Style::default()
            };
        creation_context.egui_ctx.set_style(style);
        Box::new(command_dispatcher_app)}));
}