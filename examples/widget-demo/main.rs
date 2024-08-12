use std::time::Instant;

use rust_gamepad::gamepad::{self, Gamepad};
use rust_sdl_ui::{
    desktop::{self, CommonWidgetProps},
    sdl,
};
use sdl2::{event::Event, keyboard::Keycode};

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let mut playing = true;

    // initialize window
    let (mut win, mut event_pump, mut canvas) = desktop::Window::new(3440, 1440, 60);

    // create gamepad handler
    let js = Gamepad::new("/dev/input/js0", gamepad::XBOX_MAPPING.clone());
    js.background_handler();

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

    let battery = desktop::BatteryStatusWidget::new(
        CommonWidgetProps::new(&canvas)
            .place(0.1, 0.1)
            .size(0.01, 0.06),
    )
    .on_window(&mut win);

    let wifi_strength =
        desktop::WifiStrengthWidget::new(CommonWidgetProps::new(&canvas).place(0.8, 0.2).rect(0.1))
            .on_window(&mut win);

    let light_signal =
        desktop::LightSignalWidget::new(CommonWidgetProps::new(&canvas).place(0.8, 0.45).rect(0.1))
            .on_window(&mut win);

    let horizon = desktop::HorizonWidget::new(
        CommonWidgetProps::new(&canvas).place(0.5, 0.7).rect(0.12),
        40.0,
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

    let flight_log =
        desktop::FlightLogWidget::new(CommonWidgetProps::new(&canvas).place(0.65, 0.7).rect(0.12))
            .on_window(&mut win);

    battery.write().unwrap().set(0.09);
    wifi_strength.write().unwrap().set(0.4);

    sensitivity.write().unwrap().inc();
    let mut last_state = js.state();
    let mut pitch = 0.0;
    let mut roll = 0.0;
    let mut angle = 0.0;
    while playing {
        // reset game state

        // main loop
        'running: loop {
            let start = Instant::now();
            // handle keyboard events
            if keyhandler(&mut event_pump) {
                playing = false;
                break 'running;
            }
            // clear before drawing
            sdl::sdl_clear(&mut canvas, 10, 20, 30);

            // finally draw the game and maintain fps
            let st = js.state();

            horizon.write().unwrap().set(pitch, roll, 120.0);
            drone_yaw.write().unwrap().set(angle);
            angle += 0.1;

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
            sdl::sdl_maintain_fps(start, win.fps);
            last_state = st;
        }
    }
}

fn keyhandler(event_pump: &mut sdl2::EventPump) -> bool {
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

            _ => {}
        }
    }
    false
}
