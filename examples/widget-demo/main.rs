use std::{
    env,
    fs::File,
    io::{self, Read},
    sync::mpsc,
    thread,
    time::Instant,
};

use rust_sdl_ui::{
    color::{self, RgbColor},
    desktop::{self, CommonWidgetProps},
    sdl,
};
use sdl2::{controller::Axis, event::Event, EventPump};

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let mut playing = true;

    // initialize window
    let (mut win, mut canvas) = desktop::Window::new(3440, 1440, 60, true);

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let video_file = env::var("TEST_VIDEO");
        if video_file.is_err() {
            return;
        }
        let video_file = video_file.unwrap();
        let file = File::open(video_file);
        if file.is_err() {
            return;
        }
        let file = file.unwrap();
        let mut reader = io::BufReader::new(file);
        let mut buf: [u8; 1460] = [0; 1460];
        loop {
            let nread = reader.read(&mut buf);
            if nread.is_err() {
                break;
            }
            let nread = nread.unwrap();
            if nread == 0 {
                break;
            }
            let _ = tx.send(buf[0..nread].to_vec());
        }
    });

    let _video = desktop::VideoWidget::new(
        desktop::CommonWidgetProps::new(&canvas)
            .place(0.5, 0.3)
            .size(0.5, 0.25),
        &mut canvas,
        960,
        720,
        5,
    )
    .on_window(&mut win, rx);

    let sensitivity = desktop::HorizSliderWidget::new(
        desktop::CommonWidgetProps::new(&canvas)
            .place(0.2, 0.5)
            .size(0.15, 0.003),
        0.0,
        1.0,
        5.0,
    )
    .on_window(&mut win);

    let left_stick = desktop::GamepadStickWidget::new(
        desktop::CommonWidgetProps::new(&canvas)
            .place(0.2, 0.7)
            .rect(0.1),
    )
    .on_window(&mut win);

    let right_stick = desktop::GamepadStickWidget::new(
        desktop::CommonWidgetProps::new(&canvas)
            .place(0.8, 0.7)
            .rect(0.1),
    )
    .on_window(&mut win);

    let vert_thrust =
        desktop::VertThrustWidget::new(CommonWidgetProps::new(&canvas).place(0.2, 0.2).rect(0.1))
            .on_window(&mut win);

    let temperature =
        desktop::VertThrustWidget::new(CommonWidgetProps::new(&canvas).place(0.3, 0.2).rect(0.1))
            .on_window(&mut win);

    let mut t = temperature.write().unwrap();
    t.set_color1(RgbColor::new(0.0, 0.3, 1.0, 1.0));
    t.set_color2(RgbColor::new(1.0, 0.0, 0.0, 1.0));
    t.set_color_scale_factor(0.65);
    t.set(-65.0);
    t.set_scale(0.01);
    drop(t);

    let battery = desktop::BatteryStatusWidget::new(
        CommonWidgetProps::new(&canvas)
            .place(0.1, 0.5)
            .size(0.02, 0.12),
    )
    .on_window(&mut win);

    let wifi_strength =
        desktop::WifiStrengthWidget::new(CommonWidgetProps::new(&canvas).place(0.8, 0.2).rect(0.1))
            .on_window(&mut win);

    let _light_signal =
        desktop::LightSignalWidget::new(CommonWidgetProps::new(&canvas).place(0.8, 0.45).rect(0.1))
            .on_window(&mut win);

    let horizon = desktop::HorizonWidget::new(
        CommonWidgetProps::new(&canvas).place(0.5, 0.7).rect(0.12),
        40.0,
        color::YELLOW.clone(),
    )
    .on_window(&mut win);

    let image_carousel = desktop::ImageCarouselWidget::new(
        CommonWidgetProps::new(&canvas)
            .place(0.5, 0.9)
            .size(0.8, 0.1),
        "examples/widget-demo/images",
        10,
    )
    .on_window(&mut win);

    let drone_yaw =
        desktop::DroneYawWidget::new(CommonWidgetProps::new(&canvas).place(0.35, 0.7).rect(0.12))
            .on_window(&mut win);

    let _flight_log =
        desktop::FlightLogWidget::new(CommonWidgetProps::new(&canvas).place(0.65, 0.7).rect(0.12))
            .on_window(&mut win);

    battery.write().unwrap().set(0.09);
    wifi_strength.write().unwrap().set(0.4);

    sensitivity.write().unwrap().inc();

    let mut pitch = 0.0;
    let mut roll = 0.0;
    let mut angle = 0.0;
    let mut drone = DroneHandling::default();
    while playing {
        // reset game state

        // main loop
        'running: loop {
            let start = Instant::now();
            // handle keyboard events
            if drone.drone_handler(&mut win.event_pump) {
                playing = false;
                break 'running;
            }

            tracing::info!("drone={:?}", drone);
            // clear before drawing
            sdl::sdl_clear(&mut canvas, 10, 20, 30);

            // finally draw the game and maintain fps

            horizon.write().unwrap().set(pitch, roll, 120.0);
            drone_yaw.write().unwrap().set(angle);
            angle += drone.turn_clockwise;

            // if st.a() {
            //     light_signal.write().unwrap().timestamp(utils::now_msecs);
            // }

            sensitivity.write().unwrap().set(drone.sensitivity);

            let ls = (
                drone.slide_right * drone.sensitivity,
                drone.forward * drone.sensitivity,
            );
            let rs = (drone.turn_clockwise * drone.sensitivity, 0.0);

            if drone.img_carousel_toggle_zoom {
                image_carousel.write().unwrap().toggle_show();
            }
            if drone.img_carousel_left {
                image_carousel.write().unwrap().turn_left();
            }
            if drone.img_carousel_right {
                image_carousel.write().unwrap().turn_right();
            }
            pitch += ls.1;
            roll += rs.0;
            let vert_speed = (drone.vert_accel - drone.vert_decel) * drone.sensitivity;
            vert_thrust.write().unwrap().set(vert_speed);

            left_stick.write().unwrap().set_stick(ls);
            right_stick.write().unwrap().set_stick(rs);

            drone.zero_state();

            win.draw(&mut canvas);
            // self.draw(&mut canvas, &mut texture);
            canvas.present();
            sdl::sdl_maintain_fps(start, win.fps);
        }
    }
}

#[derive(Debug)]
struct DroneHandling {
    take_picture: bool,
    toggle_video: bool,
    take_off: bool,
    hover: bool,
    sensitivity: f32,
    vert_accel: f32,
    vert_decel: f32,
    slide_right: f32,
    forward: f32,
    turn_clockwise: f32,
    img_carousel_left: bool,
    img_carousel_right: bool,
    img_carousel_toggle_zoom: bool,
}

impl DroneHandling {
    pub fn zero_state(&mut self) {
        self.take_off = false;
        self.hover = false;
        self.toggle_video = false;
        self.img_carousel_toggle_zoom = false;
        self.img_carousel_left = false;
        self.img_carousel_right = false;
    }

    pub fn drone_handler(&mut self, event_pump: &mut EventPump) -> bool {
        tracing::info!("running drone event");
        for event in event_pump.poll_iter() {
            tracing::info!("events={:?}", event);
            match event {
                Event::ControllerButtonUp { button, .. } => {
                    tracing::info!("Button {:?} up", button);
                    match button {
                        sdl2::controller::Button::A => self.take_picture = true,
                        sdl2::controller::Button::B => self.toggle_video = true,
                        sdl2::controller::Button::X => self.img_carousel_toggle_zoom = true,

                        sdl2::controller::Button::Guide => self.hover = true,
                        sdl2::controller::Button::Start => self.take_off = true,

                        sdl2::controller::Button::LeftShoulder => self.sensitivity -= 0.2,
                        sdl2::controller::Button::RightShoulder => self.sensitivity += 0.2,
                        sdl2::controller::Button::DPadLeft => self.img_carousel_left = true,
                        sdl2::controller::Button::DPadRight => self.img_carousel_right = true,
                        _ => {}
                    }
                }

                Event::ControllerAxisMotion {
                    axis, value: val, ..
                } => {
                    // Axis motion is an absolute value in the range
                    // [-32768, 32767]. Let's simulate a very rough dead
                    // zone to ignore spurious events.
                    // let dead_zone = 10_000;
                    // if val > dead_zone || val < -dead_zone {
                    tracing::info!("Axis {:?} moved to {}", axis, val);
                    match axis {
                        Axis::LeftX => self.slide_right = val as f32 / 32767.0,
                        Axis::LeftY => self.forward = val as f32 / 32767.0,
                        Axis::RightX => self.turn_clockwise = val as f32 / 32767.0,
                        Axis::TriggerRight => self.vert_accel = val as f32 / 32767.0,
                        Axis::TriggerLeft => self.vert_decel = val as f32 / 32767.0,
                        _ => {}
                    }
                    // }
                }

                sdl2::event::Event::Quit { .. } => {
                    return true;
                }
                sdl2::event::Event::KeyDown {
                    keycode: Some(sdl2::keyboard::Keycode::Escape),
                    ..
                } => {
                    return true;
                }

                _ => {}
            }
        }
        false
    }
}

impl Default for DroneHandling {
    fn default() -> Self {
        Self {
            take_off: false,
            hover: false,
            take_picture: false,
            toggle_video: false,
            img_carousel_left: false,
            img_carousel_right: false,
            img_carousel_toggle_zoom: false,
            sensitivity: 0.2,
            vert_accel: Default::default(),
            vert_decel: Default::default(),
            slide_right: Default::default(),
            forward: Default::default(),
            turn_clockwise: Default::default(),
        }
    }
}
