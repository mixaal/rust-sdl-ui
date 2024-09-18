use std::{
    f32::consts::PI,
    sync::{mpsc::Receiver, Arc, RwLock},
    thread,
    time::Duration,
};

use crate::{
    color::{self, RgbColor},
    sdl::{self, sdl_scale_text},
    texcache::TextureCache,
    utils,
    video::{StreamAction, VideoStreamDecoder},
};
use sdl2::{
    controller::GameController,
    gfx::primitives::DrawRenderer,
    pixels::Color,
    rect::Rect,
    render::{Canvas, Texture, TextureCreator},
    ttf::Sdl2TtfContext,
    video::WindowContext,
};

type SdlWin = sdl2::video::Window;

pub trait Widget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext);
}

pub struct Window {
    widgets: Vec<Box<dyn Widget>>,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub event_pump: sdl2::EventPump,
    pub ttf: sdl2::ttf::Sdl2TtfContext,
    pub controller: Option<GameController>,
}

impl Window {
    pub fn new(
        width: u32,
        height: u32,
        fps: u32,
        gamepad: bool,
    ) -> (Self, Canvas<sdl2::video::Window>) {
        let (event_pump, canvas, controller, real_width, real_height) =
            sdl::sdl_init(width, height, gamepad);
        let ttf = sdl2::ttf::init().expect("can't setup ttf context");
        (
            Self {
                widgets: Vec::new(),
                width: real_width,
                height: real_height,
                fps,
                event_pump,
                ttf,
                controller,
            },
            canvas,
        )
    }

    pub fn draw(&mut self, canvas: &mut Canvas<SdlWin>) {
        for widget in self.widgets.iter_mut() {
            widget.draw(canvas, &mut self.ttf);
        }
    }

    // installs default Esc+Quit handling event
    pub fn default_keyhandler(&mut self) -> bool {
        for event in self.event_pump.poll_iter() {
            match event {
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

pub struct CommonWidgetProps {
    canvas_width: u32,
    canvas_height: u32,
    aspect_ratio: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    texture_names: Vec<String>,
    textures: Vec<Texture>,
    tc: TextureCreator<WindowContext>,
}

impl CommonWidgetProps {
    pub fn new(canvas: &Canvas<SdlWin>) -> Self {
        let dim = canvas.window().size();
        Self {
            canvas_width: dim.0,
            canvas_height: dim.1,
            aspect_ratio: dim.0 as f32 / dim.1 as f32,
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            texture_names: Vec::new(),
            textures: Vec::new(),
            tc: canvas.texture_creator(),
        }
    }

    fn textures(self, tex_names: Vec<&str>) -> Self {
        let texture_names = tex_names.iter().map(|it| it.to_string()).collect();
        Self {
            canvas_width: self.canvas_width,
            canvas_height: self.canvas_height,
            aspect_ratio: self.aspect_ratio,
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            texture_names,
            textures: self.textures,
            tc: self.tc,
        }
    }

    pub fn place(self, x: f32, y: f32) -> Self {
        Self {
            canvas_width: self.canvas_width,
            canvas_height: self.canvas_height,
            aspect_ratio: self.aspect_ratio,
            x,
            y,
            w: self.w,
            h: self.h,
            texture_names: self.texture_names,
            textures: self.textures,
            tc: self.tc,
        }
    }

    pub fn size(self, w: f32, h: f32) -> Self {
        Self {
            canvas_width: self.canvas_width,
            canvas_height: self.canvas_height,
            aspect_ratio: self.aspect_ratio,
            x: self.x,
            y: self.y,
            w,
            h,
            texture_names: self.texture_names,
            textures: self.textures,
            tc: self.tc,
        }
    }

    pub fn rect(self, w: f32) -> Self {
        let aspect_ratio = self.aspect_ratio;
        Self {
            canvas_width: self.canvas_width,
            canvas_height: self.canvas_height,
            aspect_ratio,
            x: self.x,
            y: self.y,
            w,
            h: w * aspect_ratio,
            texture_names: self.texture_names,
            textures: self.textures,
            tc: self.tc,
        }
    }

    fn compute_dim(&self, canvas: &mut Canvas<SdlWin>) -> (i32, i32, i32, i32) {
        let (info_width, info_height) = canvas.window().size();

        let x = (info_width as f32 * self.x) as i32;
        let y = (info_height as f32 * self.y) as i32;

        let w = (info_width as f32 * self.w) as i32;
        let h = (info_height as f32 * self.h) as i32;
        (x, y, w, h)
    }

    fn load_textures(&mut self, canvas: &mut Canvas<SdlWin>) {
        if self.textures.len() == 0 {
            self.textures = sdl::sdl_load_textures(canvas, self.texture_names.clone());
        }
    }
}

pub struct TextWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<Text>>,
}

impl TextWidget {
    pub fn new(widget: CommonWidgetProps) -> Self {
        Self {
            widget: widget.textures(vec!["images/joy.png", "images/joy-stick.png"]),
            props: Arc::new(RwLock::new(Text {
                value: "".to_owned(),
            })),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<Text>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

impl Widget for TextWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);
        let text = &self.props.read().unwrap().value;
        sdl_scale_text(ttf, canvas, text, 48, color::WHITE.clone(), x, y, w, h);
    }
}

pub struct GamepadStickWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<GamepadStick>>,
}

impl GamepadStickWidget {
    pub fn new(widget: CommonWidgetProps) -> Self {
        Self {
            widget: widget.textures(vec!["images/joy.png", "images/joy-stick.png"]),
            props: Arc::new(RwLock::new(GamepadStick {
                horiz: 0.0,
                vert: 0.0,
            })),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<GamepadStick>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

impl Widget for GamepadStickWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);

        self.widget.load_textures(canvas);
        sdl::sdl_scale_tex(canvas, &self.widget.textures[0], x, y, w, h);

        let p = self.props.read().unwrap();
        let horiz = 0.4 * p.horiz;
        let vert = 0.4 * p.vert;
        drop(p);
        let xx = (x as f32 + horiz * w as f32) as i32;
        let yy = (y as f32 + vert * h as f32) as i32;

        let ww = (0.3 * w as f32) as i32;
        sdl::sdl_scale_tex(canvas, &self.widget.textures[1], xx, yy, ww, ww);
    }
}

pub struct HorizSliderWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<HorizSlider>>,
}

impl Widget for HorizSliderWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);

        self.widget.load_textures(canvas);

        let p = self.props.read().unwrap();
        let dx = -0.5 + p.value / (p.max_value - p.min_value);
        drop(p);
        let place_x = x + (w as f32 * dx) as i32;
        sdl::sdl_scale_tex(canvas, &self.widget.textures[0], x, y, w, h);
        sdl::sdl_render_tex(canvas, &self.widget.textures[1], place_x, y);
    }
}

impl HorizSliderWidget {
    pub fn new(widget: CommonWidgetProps, min_value: f32, max_value: f32, steps: f32) -> Self {
        Self {
            widget: widget.textures(vec!["images/slider-bg.png", "images/slider-button.png"]),
            props: Arc::new(RwLock::new(HorizSlider {
                min_value,
                max_value,
                value: min_value,
                steps,
            })),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<HorizSlider>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct VertThrustWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<VertThrust>>,
}

impl Widget for VertThrustWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let tc = canvas.texture_creator();
        let (x, y, w, h) = self.widget.compute_dim(canvas);
        let p = self.props.read().unwrap();
        let vert_speed = p.vert_value;
        let c1 = p.color1.clone();
        let c2 = p.color2.clone();
        let factor = p.color_scale_factor;
        let scale = p.scale;
        drop(p);

        self.widget.load_textures(canvas);

        sdl::sdl_scale_tex(canvas, &self.widget.textures[0], x, y, w, h);
        let d_color = c2.clone() - c1.clone();
        let dst_color = c1.clone() + d_color.mul(factor * vert_speed.abs() * scale);
        let dw = (w as f32 * 0.12) as i32;
        sdl::draw_horizontal_gradient_box(
            canvas,
            x - dw / 2,
            y,
            dw,
            (vert_speed * scale * h as f32 / 2.0) as i32,
            128,
            c1,
            dst_color,
            true,
        );
        sdl::sdl_text(
            ttf,
            canvas,
            &vert_speed.to_string(),
            24,
            color::WHITE.clone(),
            x,
            y,
        );
    }
}

impl VertThrustWidget {
    pub fn new(widget: CommonWidgetProps) -> Self {
        Self {
            widget: widget.textures(vec!["images/vert.png"]),
            props: Arc::new(RwLock::new(VertThrust {
                vert_value: 0.0,
                color1: color::YELLOW.clone(),
                color2: color::BLUE.clone(),
                color_scale_factor: 1.0,
                scale: 1.0,
            })),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<VertThrust>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct RawImageWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<RawImage>>,
    image_texture: Texture,
}

impl Widget for RawImageWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);
        let p = self.props.read().unwrap();
        let img_width = p.width;
        let img_height = p.height;
        let rgb = p.rgb.clone();
        drop(p);

        self.image_texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..img_height {
                    for x in 0..img_width {
                        let offset = y as usize * pitch + x as usize * 4;
                        let source_offset = ((y * img_width + x) * 3) as usize;
                        buffer[offset] = rgb[source_offset];
                        buffer[offset + 1] = rgb[source_offset + 1];
                        buffer[offset + 2] = rgb[source_offset + 2];
                        buffer[offset + 3] = 255;
                    }
                }
                drop(rgb);
            })
            .unwrap();

        canvas
            .copy(
                &self.image_texture,
                None,
                Some(sdl2::rect::Rect::new(
                    x - w / 2,
                    y - h / 2,
                    w as u32,
                    h as u32,
                )),
            )
            .unwrap();
    }
}

impl RawImageWidget {
    pub fn new(
        widget: CommonWidgetProps,
        canvas: &mut Canvas<SdlWin>,
        width: u32,
        height: u32,
    ) -> Self {
        let texture_creator = canvas.texture_creator();
        let image_texture = texture_creator
            .create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGBA32, width, height)
            .expect("can't create texture renderer");
        Self {
            image_texture,
            widget,
            props: Arc::new(RwLock::new(RawImage {
                rgb: utils::alloc_vec((width * height * 3).try_into().unwrap()),
                width,
                height,
            })),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<RawImage>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct VideoWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<Video>>,
    image_texture: Texture,
    inner_decoder: Arc<VideoDecoder>,
}

impl Widget for VideoWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);
        let p = self.props.read().unwrap();
        let img_width = p.width;
        let img_height = p.height;
        drop(p);

        let rgb = self.inner_decoder.rgb.read().unwrap();
        self.image_texture
            .with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..img_height {
                    for x in 0..img_width {
                        let offset = y as usize * pitch + x as usize * 4;
                        let source_offset = ((y * img_width + x) * 3) as usize;
                        buffer[offset] = rgb[source_offset];
                        buffer[offset + 1] = rgb[source_offset + 1];
                        buffer[offset + 2] = rgb[source_offset + 2];
                        buffer[offset + 3] = 255;
                    }
                }
            })
            .unwrap();
        drop(rgb);
        canvas
            .copy(
                &self.image_texture,
                None,
                Some(sdl2::rect::Rect::new(
                    x - w / 2,
                    y - h / 2,
                    w as u32,
                    h as u32,
                )),
            )
            .unwrap();
    }
}

impl VideoWidget {
    pub fn new(
        widget: CommonWidgetProps,
        canvas: &mut Canvas<SdlWin>,
        width: u32,
        height: u32,
        skip_frames: usize,
    ) -> Self {
        let texture_creator = canvas.texture_creator();
        let image_texture = texture_creator
            .create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGBA32, width, height)
            .expect("can't create texture renderer");
        // let rx = Arc::new(RwLock::new(video_stream));
        // thread::spawn(move || Self::decode_video(video_stream, width, height));
        Self {
            image_texture,
            widget,
            props: Arc::new(RwLock::new(Video::new(width, height))),
            inner_decoder: Arc::new(VideoDecoder::new(width, height, skip_frames)),
        }
    }

    pub fn on_window(self, window: &mut Window, rx: Receiver<Vec<u8>>) -> Arc<VideoDecoder> {
        let hz = self.inner_decoder.clone();
        let inner = self.inner_decoder.clone();
        thread::spawn(move || inner.decode_video(rx));
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct BatteryStatusWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<FloatClampedValue>>,
    timer: utils::GameTimer,
}

impl Widget for BatteryStatusWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);
        let p = self.props.read().unwrap();
        let percentage = p.value;
        drop(p);
        if percentage < 0.1 && self.timer.blink() {
            return; // do not draw when blinking
        }
        let cyber_blue = color::CYBER_COOL_BLUE.to_sdl_rgba();
        let red = color::RED.to_sdl_rgba();
        let yellow = color::YELLOW.to_sdl_rgba();
        canvas.set_draw_color(cyber_blue);
        let sx = x - w / 2;
        let sy = y - h / 2;
        let _ = canvas.draw_rect(Rect::new(sx, sy, w as u32, h as u32));
        if percentage >= 0.9 {
            canvas.set_draw_color(cyber_blue);
        } else if percentage > 0.1 {
            canvas.set_draw_color(yellow);
        } else {
            canvas.set_draw_color(red);
        }
        let top_y = sy + ((1.0 - percentage) * h as f32) as i32;
        let bottom_y = sy + h - 3;
        let _ = canvas.fill_rect(Rect::new(
            sx + 3,
            top_y,
            w as u32 - 6,
            (bottom_y - top_y) as u32,
        ));
        let val = (percentage * 100.0) as i32;
        let text = format!("{val}%");
        sdl::sdl_text(ttf, canvas, &text, 24, color::WHITE.clone(), x, y);
    }
}

impl BatteryStatusWidget {
    pub fn new(widget: CommonWidgetProps) -> Self {
        Self {
            widget,
            props: Arc::new(RwLock::new(FloatClampedValue { value: 0.0 })),
            timer: utils::GameTimer::new(Duration::from_secs(1)),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<FloatClampedValue>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct WifiStrengthWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<FloatClampedValue>>,
    timer: utils::GameTimer,
}

impl Widget for WifiStrengthWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);
        self.widget.load_textures(canvas);

        let p = self.props.read().unwrap();
        let value = p.value;
        let radius = value * w as f32 * 0.4 * self.timer.range();
        drop(p);
        if value < 0.45 && self.timer.blink() {
            return;
        }
        sdl::sdl_scale_tex(canvas, &self.widget.textures[0], x, y, w, h);
        let mut alpha = 1.0;
        let dx = x + (w as f32 * 0.007) as i32;
        let dy = y + (w as f32 * 0.009) as i32;
        let signal_color = if value < 0.45 {
            color::RED.clone()
        } else {
            color::YELLOW.clone()
        };

        for r in (0..radius as usize).step_by(5) {
            let _ = canvas.circle(
                dx as i16,
                dy as i16,
                r as i16,
                signal_color.with_alpha(alpha).to_sdl_rgba(),
            );
            alpha -= 0.1;
            if alpha < 0.4 {
                alpha = 0.4;
            }
        }
        let strength = (value * 100.0) as i32;
        sdl::sdl_text(
            ttf,
            canvas,
            &strength.to_string(),
            48,
            color::WHITE.clone(),
            x,
            y - 2 * h / 5,
        );
    }
}

impl WifiStrengthWidget {
    pub fn new(widget: CommonWidgetProps) -> Self {
        Self {
            widget: widget.textures(vec!["images/radar-bg.png"]),
            props: Arc::new(RwLock::new(FloatClampedValue { value: 0.0 })),
            timer: utils::GameTimer::new(Duration::from_millis(800)),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<FloatClampedValue>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct LightSignalWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<LightSignal>>,
    timer: utils::GameTimer,
}

impl Widget for LightSignalWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);
        self.widget.load_textures(canvas);

        let p = self.props.read().unwrap();
        let last_signal = p.tm;
        drop(p);
        let elapsed = (utils::now_msecs() - last_signal) / 1000;
        let mut alpha = 255 as i64 - 10 * elapsed as i64;
        if alpha < 0 {
            alpha = 0;
        }

        let red = 255;
        let mut green = 255;

        let radius = w as f32 * 0.2 * self.timer.range();
        let secs_elapsed = elapsed;
        if secs_elapsed > 10 {
            green = 0;
            if self.timer.blink() {
                return;
            }
        }
        sdl::sdl_scale_tex(canvas, &self.widget.textures[0], x, y, w, h);

        let dx = x - (w as f32 * 0.007) as i32;
        let dy = y - (w as f32 * 0.009) as i32;

        let _ = canvas.filled_circle(
            dx as i16,
            dy as i16,
            radius as i16,
            Color::RGBA(red, green, 0, alpha as u8),
        );
        sdl::sdl_text(
            ttf,
            canvas,
            &secs_elapsed.to_string(),
            48,
            color::RED.clone(),
            x,
            y,
        );
    }
}

impl LightSignalWidget {
    pub fn new(widget: CommonWidgetProps) -> Self {
        Self {
            widget: widget.textures(vec!["images/light-bg.png"]),
            props: Arc::new(RwLock::new(LightSignal {
                tm: utils::now_msecs(),
            })),
            timer: utils::GameTimer::new(Duration::from_millis(800)),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<LightSignal>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct HorizonWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<DroneOrientation>>,
    max_pitch: f32,
    horizon_color: RgbColor,
}

impl Widget for HorizonWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);
        self.widget.load_textures(canvas);

        let p = self.props.read().unwrap();
        let roll = p.roll;
        let pitch = p.pitch;
        drop(p);

        let circle_radius = w as f32 / 3.0;
        let left_angle = (roll - 90.0) * PI / 180.0;
        let right_angle = (roll + 90.0) * PI / 180.0;
        let roll_rad = roll * PI / 180.0;

        // want max_pitch is w/3
        if pitch < self.max_pitch && pitch > -self.max_pitch {
            let sr = w as f32 * pitch / (3.0 * self.max_pitch);
            let dx = (sr * roll_rad.sin()) as i32;
            let dy = (sr * roll_rad.cos()) as i32;

            let x1 = (circle_radius * left_angle.sin()) as i32;
            let y1 = (circle_radius * left_angle.cos()) as i32;

            let x2 = (circle_radius * right_angle.sin()) as i32;
            let y2 = (circle_radius * right_angle.cos()) as i32;

            canvas.set_draw_color(self.horizon_color.to_sdl_rgba());
            let _ = canvas.draw_line((x + x1 + dx, y - y1 - dy), (x + x2 + dx, y - y2 - dy));
        }

        sdl::sdl_scale_tex(canvas, &self.widget.textures[0], x, y, w, h);
    }
}

impl HorizonWidget {
    pub fn new(widget: CommonWidgetProps, max_pitch: f32, horizon_color: RgbColor) -> Self {
        Self {
            max_pitch,
            horizon_color,
            widget: widget.textures(vec!["images/horizon-gauge-fg.png"]),
            props: Arc::new(RwLock::new(DroneOrientation {
                pitch: 0.0,
                roll: 0.0,
                yaw: 0.0,
            })),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<DroneOrientation>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct ImageCarouselWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<ImageCarousel>>,
    texcache: TextureCache,
}

impl Widget for ImageCarouselWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);
        let zw: f32 = self.widget.canvas_width as f32 * 0.7;

        let p = self.props.read().unwrap();
        let images_no = p.number_of_images;
        let image_dir = p.image_dir.clone();
        let offset = p.offset;
        let show = p.show;
        drop(p);

        let dw = w as usize / images_no;

        let files = utils::DirectoryReader::new(&image_dir).list();
        let mut images = Vec::new();
        let mut zoomed_image = None;
        for i in 0..images_no {
            if i + offset >= files.len() {
                break;
            }
            let image_file = files[i + offset].clone();

            let r =
                self.texcache
                    .load_texture(canvas, image_file.clone(), dw as u32, h as u32, None);
            if r.is_err() {
                tracing::error!("error loading texture: {}", r.err().unwrap());
                continue;
            }
            let tex = r.unwrap();
            let original_aspect_ratio = tex.original_aspect;

            if show && i == 0 {
                let zh = zw / original_aspect_ratio;
                let zoomed = self
                    .texcache
                    .load_texture(canvas, image_file, zw as u32, zh as u32, None);
                if zoomed.is_err() {
                    tracing::error!("zoomed image: {}", zoomed.err().unwrap());
                } else {
                    zoomed_image = Some(zoomed.unwrap());
                }
            }

            images.push(tex);
        }

        let sx = x - w / 2;
        let sy = y - h / 2;

        for i in 0..images_no {
            let dx = i * dw;
            let x1 = sx + dx as i32;

            if i < images.len() {
                let tex = &images[i];
                let g = tex.texture.read().unwrap();
                sdl::sdl_render_tex(canvas, &g, x1 + dw as i32 / 2, y);
                drop(g);
                if show {
                    if let Some(ref zimage) = zoomed_image {
                        let g = zimage.texture.read().unwrap();
                        sdl::sdl_render_tex(
                            canvas,
                            &g,
                            (self.widget.canvas_width / 2) as i32,
                            (self.widget.canvas_height / 2) as i32,
                        );
                        drop(g);
                    }
                }
            }
            canvas.set_draw_color(color::CYBER_COOL_BLUE.to_sdl_rgba());
            let _ = canvas.draw_rect(Rect::new(x1, sy, dw as u32, h as u32));
        }
    }
}

impl ImageCarouselWidget {
    pub fn new(widget: CommonWidgetProps, image_dir: &str, number_of_images: usize) -> Self {
        Self {
            widget,
            props: Arc::new(RwLock::new(ImageCarousel {
                image_dir: image_dir.to_owned(),
                number_of_images,
                offset: 0,
                show: false,
            })),
            texcache: TextureCache::new(),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<ImageCarousel>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct DroneYawWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<FloatGenericValue>>,
    texcache: TextureCache,
}

impl Widget for DroneYawWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);
        let p = self.props.read().unwrap();
        let angle = p.value;
        drop(p);

        let bg = self
            .texcache
            .load_texture(
                canvas,
                "images/yaw-bg.png".to_owned(),
                w as u32,
                h as u32,
                None,
            )
            .expect("can't load yaw bg texture");

        let fg = self
            .texcache
            .load_texture(
                canvas,
                "images/yaw-fg.png".to_owned(),
                w as u32 * 4 / 5, // somewhat smaller than the background
                h as u32 * 4 / 5,
                None,
            )
            .expect("can't load yaw bg texture");

        bg.render(canvas, x, y);
        fg.render_rot(canvas, x, y, angle);

        let text = format!("{:.1}{}", angle, 176 as char);
        sdl::sdl_text(ttf, canvas, &text, 24, color::BLACK.clone(), x, y);
    }
}

impl DroneYawWidget {
    pub fn new(widget: CommonWidgetProps) -> Self {
        Self {
            widget,
            props: Arc::new(RwLock::new(FloatGenericValue { value: 0.0 })),
            texcache: TextureCache::new(),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<FloatGenericValue>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct FlightLogWidget {
    widget: CommonWidgetProps,
    props: Arc<RwLock<FlightLog>>,
    texcache: TextureCache,
}

impl Widget for FlightLogWidget {
    fn draw(&mut self, canvas: &mut Canvas<SdlWin>, ttf: &mut Sdl2TtfContext) {
        let (x, y, w, h) = self.widget.compute_dim(canvas);

        let bg = self
            .texcache
            .load_texture(
                canvas,
                "images/radius-bg.png".to_owned(),
                w as u32,
                h as u32,
                None,
            )
            .expect("can't load yaw bg texture");

        bg.render(canvas, x, y);
    }
}

impl FlightLogWidget {
    pub fn new(widget: CommonWidgetProps) -> Self {
        Self {
            widget,
            props: Arc::new(RwLock::new(FlightLog {})),
            texcache: TextureCache::new(),
        }
    }

    pub fn on_window(self, window: &mut Window) -> Arc<RwLock<FlightLog>> {
        let hz = self.props.clone();
        window.widgets.push(Box::new(self));
        hz
    }
}

pub struct Text {
    value: String,
}

impl Text {
    pub fn set(&mut self, value: String) {
        self.value = value;
    }
}

pub struct VideoDecoder {
    pub rgb: Arc<RwLock<Vec<u8>>>,
    pub skip_frames: usize,
}

pub struct Video {
    width: u32,
    height: u32,
}

impl Video {
    fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl VideoDecoder {
    fn new(width: u32, height: u32, skip_frames: usize) -> Self {
        Self {
            rgb: Arc::new(RwLock::new(utils::alloc_vec((width * height * 3) as usize))),
            skip_frames,
        }
    }

    fn decode_video(&self, rx: Receiver<Vec<u8>>) {
        let mut vd = VideoStreamDecoder::new(self.skip_frames);
        loop {
            let stream = rx.recv();
            if stream.is_err() {
                tracing::error!("error decoding stream: {}", stream.err().unwrap());
                thread::sleep(Duration::from_millis(500));
                continue;
            }
            let mut stream = stream.unwrap();
            // tracing::info!("stream.len={}", stream.len());
            vd.send_stream(&mut stream);
            loop {
                let r = vd.decode_images(&self.rgb);
                if r != StreamAction::CallNext {
                    break;
                }
            }
        }
    }
}

pub struct FlightLog {}

pub struct ImageCarousel {
    image_dir: String,
    number_of_images: usize,
    offset: usize,
    show: bool,
}

impl ImageCarousel {
    pub fn turn_right(&mut self) {
        self.offset += 1;
    }

    pub fn turn_left(&mut self) {
        if self.offset >= 1 {
            self.offset -= 1;
        }
    }

    pub fn toggle_show(&mut self) {
        self.show = if self.show { false } else { true };
    }
}

pub struct DroneOrientation {
    pitch: f32,
    roll: f32,
    yaw: f32,
}

impl DroneOrientation {
    pub fn set(&mut self, pitch: f32, roll: f32, yaw: f32) {
        self.pitch = pitch;
        self.roll = roll;
        self.yaw = yaw;
    }
}

pub struct LightSignal {
    tm: u128,
}

impl LightSignal {
    pub fn timestamp(&mut self, tm: u128) {
        self.tm = tm;
    }
}

pub struct FloatClampedValue {
    value: f32,
}

impl FloatClampedValue {
    pub fn set(&mut self, value: f32) {
        self.value = utils::clamp(value);
    }

    pub fn get(&self) -> f32 {
        self.value
    }
}

pub struct FloatGenericValue {
    value: f32,
}

impl FloatGenericValue {
    pub fn set(&mut self, value: f32) {
        self.value = value;
    }

    pub fn get(&self) -> f32 {
        self.value
    }
}

pub struct RawImage {
    pub(crate) rgb: Vec<u8>,
    width: u32,
    height: u32,
}

impl RawImage {
    pub fn set_size(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn set_image(&mut self, rgb: &[u8]) {
        self.rgb = rgb.to_vec();
    }
}

pub struct GamepadStick {
    horiz: f32,
    vert: f32,
}

impl GamepadStick {
    pub fn set_stick(&mut self, p: (f32, f32)) {
        self.horiz = p.0;
        self.vert = p.1;
    }
}

pub struct HorizSlider {
    min_value: f32,
    max_value: f32,
    value: f32,
    steps: f32,
}

impl HorizSlider {
    pub fn inc(&mut self) {
        if self.value < self.max_value {
            self.value += (self.max_value - self.min_value) / self.steps;
        }
        if self.value > self.max_value {
            self.value = self.max_value;
        }
    }

    pub fn dec(&mut self) {
        if self.value > self.min_value {
            self.value -= (self.max_value - self.min_value) / self.steps;
        }
        if self.value < self.min_value {
            self.value = self.min_value;
        }
    }

    pub fn set(&mut self, v: f32) {
        self.value = utils::clamp_range(v, self.min_value, self.max_value);
    }

    pub fn get(&self) -> f32 {
        self.value
    }
}

pub struct VertThrust {
    vert_value: f32,
    color1: RgbColor,
    color2: RgbColor,
    color_scale_factor: f32,
    scale: f32,
}

impl VertThrust {
    pub fn set(&mut self, v: f32) {
        self.vert_value = v;
    }

    pub fn set_color_scale_factor(&mut self, v: f32) {
        self.color_scale_factor = utils::clamp(v);
    }

    pub fn set_scale(&mut self, v: f32) {
        self.scale = v;
    }

    pub fn get(&self) -> f32 {
        self.vert_value
    }

    pub fn set_color1(&mut self, color1: RgbColor) {
        self.color1 = color1;
    }

    pub fn set_color2(&mut self, color2: RgbColor) {
        self.color2 = color2;
    }
}
