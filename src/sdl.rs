use std::time::{Duration, Instant};

use sdl2::{
    controller::GameController,
    image::LoadTexture,
    pixels::Color,
    rect::{Point, Rect},
    render::{Canvas, Texture},
    video::Window,
    EventPump, Sdl,
};

use crate::{utils, vec::Vec4};

use super::color::RgbColor;
lazy_static! {
    static ref FONT_PATH: String = utils::get_env(
        "SDL_UI_FONT",
        "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf"
    );
}

pub fn sdl_init(
    width: u32,
    height: u32,
    gamepad: bool,
) -> (EventPump, Canvas<Window>, Option<GameController>, u32, u32) {
    let sdl_context = sdl2::init().unwrap();

    let mut controller = None;
    if gamepad {
        let r = sdl_joy_init(sdl_context.clone());
        if r.is_err() {
            tracing::error!("error initializing gamepad");
        } else {
            controller = Some(r.unwrap());
        }
    }
    let event_pump = sdl_context.event_pump().unwrap();

    let video_subsystem = sdl_context.video().unwrap();

    let (w, h) = if let Ok(video_mode) = video_subsystem.current_display_mode(0) {
        (video_mode.w as u32, video_mode.h as u32)
    } else {
        (width, height)
    };
    tracing::info!("Using video mode {w}x{h}");

    let window = video_subsystem
        .window("Rustvaders", width, height)
        .fullscreen_desktop()
        .position_centered()
        .build()
        .expect("could not initialize video subsystem");

    let border_size = window.drawable_size();
    let canvas = window
        .into_canvas()
        .build()
        .expect("could not make a canvas");

    tracing::info!("border_size={:?}", border_size);
    (event_pump, canvas, controller, border_size.0, border_size.1)
}

pub fn sdl_joy_init(sdl_context: Sdl) -> Result<GameController, String> {
    let game_controller_subsystem = sdl_context.game_controller()?;

    let available = game_controller_subsystem
        .num_joysticks()
        .map_err(|e| format!("can't enumerate joysticks: {}", e))?;

    tracing::info!("{} joysticks available", available);

    // Iterate over all available joysticks and look for game controllers.
    let controller = (0..available)
        .find_map(|id| {
            if !game_controller_subsystem.is_game_controller(id) {
                tracing::warn!("{} is not a game controller", id);
                return None;
            }

            tracing::info!("Attempting to open controller {}", id);

            match game_controller_subsystem.open(id) {
                Ok(c) => {
                    // We managed to find and open a game controller,
                    // exit the loop
                    tracing::info!("Success: opened \"{}\"", c.name());
                    Some(c)
                }
                Err(e) => {
                    tracing::error!("failed: {:?}", e);
                    None
                }
            }
        })
        .expect("Couldn't open any controller");

    tracing::info!("Controller mapping: {}", controller.mapping());
    tracing::info!("is attached: {}", controller.attached());
    Ok(controller)
}

pub fn sdl_load_textures(canvas: &Canvas<Window>, images: Vec<String>) -> Vec<Texture> {
    let mut textures: Vec<Texture> = Vec::new();
    let tc = canvas.texture_creator();
    for img in images.iter() {
        let tex = tc.load_texture(img).unwrap();
        textures.push(tex);
    }
    textures
}

pub fn sdl_render_rect_with_caption(
    canvas: &mut Canvas<Window>,
    text: &str,
    ttf: &mut sdl2::ttf::Sdl2TtfContext,
    font_size: u16,
    (x1, y1): (i32, i32),
    (x2, y2): (i32, i32),
    text_color: Vec4,
    color: Vec4,
) {
    sdl_text(ttf, canvas, text, font_size, text_color, (x1 + x2) / 2, y1);
    sdl_render_rect(canvas, (x1, y1), (x2, y2), color);
}

pub fn sdl_render_rect(
    canvas: &mut Canvas<Window>,
    (x1, y1): (i32, i32),
    (x2, y2): (i32, i32),
    color: Vec4,
) {
    if x1 > x2 {
        return;
    }
    if y1 > y2 {
        return;
    }
    canvas.set_draw_color(color.to_sdl_rgba());
    let rect = Rect::new(x1, y1, (x2 - x1) as u32, (y2 - y1) as u32);
    let _ = canvas.draw_rect(rect);
}

pub fn sdl_render_tex(canvas: &mut Canvas<Window>, texture: &Texture, x: i32, y: i32) {
    let h = texture.query().height;
    let w = texture.query().width;

    let sprite = Rect::new(0, 0, w, h);
    canvas
        .copy(
            texture,
            sprite,
            Rect::from_center(Point::new(x, y), sprite.width(), sprite.height()),
        )
        .unwrap();
}

pub fn sdl_scale_tex(
    canvas: &mut Canvas<Window>,
    texture: &Texture,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    let sprite = Rect::new(0, 0, texture.query().width, texture.query().height);
    canvas
        .copy(
            texture,
            sprite,
            Rect::from_center(Point::new(x, y), w as u32, h as u32),
        )
        .unwrap();
}

pub fn sdl_clear(canvas: &mut Canvas<Window>, r: u8, g: u8, b: u8) {
    canvas.set_draw_color(Color::RGBA(r, g, b, 255));
    canvas.clear();
}

fn text2tex(
    ttf: &mut sdl2::ttf::Sdl2TtfContext,
    canvas: &mut Canvas<Window>,
    text: &str,
    font_size: u16,
    color: RgbColor,
    x: i32,
    y: i32,
) -> Option<Texture> {
    let mut fsize = font_size;
    if fsize == 0 {
        fsize = 24;
    }
    let font = ttf.load_font(FONT_PATH.clone(), fsize);
    if font.is_err() {
        return None;
    }

    let tc = canvas.texture_creator();

    // let val = vert_speed as i32;
    let font = font.unwrap();
    //font.set_style(sdl2::ttf::FontStyle::BOLD);
    let surface = font.render(text).blended(color.to_sdl_rgba());
    if surface.is_err() {
        return None;
    }
    let surface = surface.unwrap();
    let texture = tc.create_texture_from_surface(&surface);
    if texture.is_err() {
        return None;
    }
    let texture = texture.unwrap();
    Some(texture)
}

pub fn sdl_text(
    ttf: &mut sdl2::ttf::Sdl2TtfContext,
    canvas: &mut Canvas<Window>,
    text: &str,
    font_size: u16,
    color: RgbColor,
    x: i32,
    y: i32,
) {
    if let Some(texture) = text2tex(ttf, canvas, text, font_size, color, x, y) {
        sdl_render_tex(canvas, &texture, x, y);
    }
}

pub fn sdl_scale_text(
    ttf: &mut sdl2::ttf::Sdl2TtfContext,
    canvas: &mut Canvas<Window>,
    text: &str,
    font_size: u16,
    color: RgbColor,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    if let Some(texture) = text2tex(ttf, canvas, text, font_size, color, x, y) {
        sdl_scale_tex(canvas, &texture, x, y, w, h);
    }
}

pub fn sdl_maintain_fps(start: Instant, fps: u32) {
    let frame_duration = Duration::new(0, 1_000_000_000u32 / fps);
    let elapsed = start.elapsed();
    match frame_duration.checked_sub(elapsed) {
        Some(dt) => ::std::thread::sleep(dt),
        None => {}
    }
}

pub fn draw_horizontal_gradient_box(
    canvas: &mut Canvas<Window>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    steps: usize,
    src_color: RgbColor,
    dst_color: RgbColor,
    fill: bool,
) {
    let c1 = src_color.to_rgba();
    let c1 = Color::RGBA(c1[0], c1[1], c1[2], c1[3]);
    let c2 = dst_color.to_rgba();
    let c2 = Color::RGBA(c2[0], c2[1], c2[2], c2[3]);
    /* Acumulator initial position */
    let mut yt = y as f32;
    let mut rt = c1.r as f32;
    let mut gt = c1.g as f32;
    let mut bt = c1.b as f32;
    let mut at = c1.a as f32;

    /* Changes in each attribute */
    let ys = h as f32 / steps as f32;

    let rs = (c2.r as f32 - rt) / steps as f32;
    let gs = (c2.g as f32 - gt) / steps as f32;
    let bs = (c2.b as f32 - bt) / steps as f32;
    let a_s = (c2.a as f32 - at) / steps as f32;

    for _ in 0..steps {
        /* Create an horizontal rectangle sliced by the number of steps */
        let rect = Rect::new(x, yt as i32, w as u32, (ys + 1.0) as u32);

        /* Sets the rectangle color based on iteration */
        canvas.set_draw_color(Color::RGBA(rt as u8, gt as u8, bt as u8, at as u8));

        /* Paint it or coverit*/
        if fill {
            let _ = canvas.fill_rect(rect);
        } else {
            let _ = canvas.draw_rect(rect);
        }

        /* Update colors and positions */
        yt += ys;
        rt += rs;
        gt += gs;
        bt += bs;
        at += a_s;
    }
}
