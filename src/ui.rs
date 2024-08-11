use std::{
    sync::{mpsc::Receiver, Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

use sdl2::{event::Event, keyboard::Keycode};

use crate::{
    desktop::{self, CommonWidgetProps, RawImage},
    gamepad::{self, Gamepad},
    gfx::sdl,
    utils,
};

pub struct UI {
    width: u32,
    height: u32,
    fps: u32,
}

lazy_static! {
    static ref VIDEO_FRAME: RwLock<Vec<u8>> = RwLock::new(utils::alloc_vec(960 * 720 * 3));
}

impl UI {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            fps: 60,
        }
    }

    fn update_video_frame(rx: Receiver<Vec<u8>>, image: Arc<RwLock<RawImage>>) {
        loop {
            let rgb = rx.recv_timeout(Duration::from_millis(20));
            if rgb.is_ok() {
                let rgb = rgb.unwrap();
                let mut g = image.write().unwrap();
                for (idx, b) in rgb.iter().enumerate() {
                    g.rgb[idx] = *b;
                }
                drop(g);
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn mainloop(&self, rx: Receiver<Vec<u8>>) {
        let (mut event_pump, mut canvas) = sdl::sdl_init(self.width, self.height);
        let mut playing = true;

        let js = Gamepad::new("/dev/input/js0", gamepad::XBOX_MAPPING.clone());
        js.background_handler();

        let mut win = desktop::Window::new();
        let raw_image = desktop::RawImageWidget::new(
            CommonWidgetProps::new(&canvas)
                .place(0.5, 0.3)
                .size(0.4, 0.5),
            &mut canvas,
            960,
            720,
        )
        .on_window(&mut win);

        thread::spawn(move || Self::update_video_frame(rx, raw_image));

        let sensitivity = desktop::HorizSliderWidget::new(
            desktop::CommonWidgetProps::new(&canvas)
                .place(0.8, 0.1)
                .size(0.2, 0.003),
            0.0,
            1.0,
            5.0,
        )
        .on_window(&mut win);

        let action_radius = desktop::HorizSliderWidget::new(
            desktop::CommonWidgetProps::new(&canvas)
                .place(0.8, 0.2)
                .size(0.2, 0.003),
            0.0,
            20.0,
            10.0,
        )
        .on_window(&mut win);

        let left_stick = desktop::GamepadStickWidget::new(
            desktop::CommonWidgetProps::new(&canvas)
                .place(0.2, 0.8)
                .rect(0.1),
        )
        .on_window(&mut win);

        let right_stick = desktop::GamepadStickWidget::new(
            desktop::CommonWidgetProps::new(&canvas)
                .place(0.8, 0.8)
                .rect(0.1),
        )
        .on_window(&mut win);

        let vert_thrust = desktop::VertThrustWidget::new(
            CommonWidgetProps::new(&canvas).place(0.2, 0.2).rect(0.1),
        )
        .on_window(&mut win);

        let battery = desktop::BatteryStatusWidget::new(
            CommonWidgetProps::new(&canvas)
                .place(0.1, 0.3)
                .size(0.01, 0.06),
        )
        .on_window(&mut win);

        let wifi_strength = desktop::WifiStrengthWidget::new(
            CommonWidgetProps::new(&canvas).place(0.1, 0.6).rect(0.1),
        )
        .on_window(&mut win);

        let light_signal = desktop::LightSignalWidget::new(
            CommonWidgetProps::new(&canvas).place(0.2, 0.6).rect(0.1),
        )
        .on_window(&mut win);

        let horizon = desktop::HorizonWidget::new(
            CommonWidgetProps::new(&canvas).place(0.5, 0.8).rect(0.12),
            40.0,
        )
        .on_window(&mut win);

        let image_carousel = desktop::ImageCarouselWidget::new(
            CommonWidgetProps::new(&canvas)
                .place(0.5, 0.1)
                .size(0.8, 0.1),
            "examples/widget-demo/images",
            10,
        )
        .on_window(&mut win);

        battery.write().unwrap().set(0.09);
        wifi_strength.write().unwrap().set(0.4);

        sensitivity.write().unwrap().inc();
        let mut last_state = js.state();
        let mut pitch = 0.0;
        let mut roll = 0.0;
        while playing {
            // reset game state

            // main loop
            let mut now = Instant::now();
            'running: loop {
                let start = Instant::now();
                // handle keyboard events
                if self.keyhandler(&mut event_pump) {
                    playing = false;
                    break 'running;
                }
                // clear before drawing
                sdl::sdl_clear(&mut canvas, 10, 20, 30);

                // finally draw the game and maintain fps
                let st = js.state();

                horizon.write().unwrap().set(pitch, roll, 120.0);

                if st.a() {
                    light_signal.write().unwrap().now();
                }

                let rb = st.button_clicked(gamepad::Buttons::RB, &last_state);
                let lb = st.button_clicked(gamepad::Buttons::LB, &last_state);
                if rb {
                    sensitivity.write().unwrap().inc();
                }
                if lb {
                    sensitivity.write().unwrap().dec();
                }
                let sensitivity_valye = sensitivity.read().unwrap().get();

                let ls = st.l_stick(sensitivity_valye);
                let rs = st.r_stick(sensitivity_valye);
                let lt = st.lt(sensitivity_valye);
                let rt = st.rt(sensitivity_valye);

                let horiz = st.horiz();
                let x_clicked = st.button_clicked(gamepad::Buttons::X, &last_state);
                if x_clicked {
                    image_carousel.write().unwrap().toggle_show();
                }
                if horiz < 0.0 {
                    image_carousel.write().unwrap().turn_left();
                }
                if horiz > 0.0 {
                    image_carousel.write().unwrap().turn_right();
                }
                pitch += ls.1;
                roll += rs.0;
                let vert_speed = lt - rt;
                vert_thrust.write().unwrap().set(vert_speed);

                left_stick.write().unwrap().set_stick(ls);
                right_stick.write().unwrap().set_stick(rs);

                win.draw(&mut canvas);
                // self.draw(&mut canvas, &mut texture);
                canvas.present();
                now = Instant::now();
                sdl::sdl_maintain_fps(start, self.fps);
                last_state = st;
            }
        }
    }

    fn keyhandler(&self, event_pump: &mut sdl2::EventPump) -> bool {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    return true;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    return true;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => {
                    return false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    return false;
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Left),
                    ..
                } => {
                    return false;
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Right),
                    ..
                } => {
                    return false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => {}
                Event::KeyUp {
                    keycode: Some(Keycode::Space),
                    ..
                } => {}
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    ..
                } => {
                    println!("Pausing Music");
                    // sound_manager.stop_sound(&MUSIC_FILENAME.to_string());
                }
                Event::KeyDown {
                    keycode: Some(Keycode::O),
                    ..
                } => {
                    println!("Resuming Music");
                    // sound_manager.resume_sound(&MUSIC_FILENAME.to_string());
                }
                _ => {}
            }
        }
        false
    }
}
