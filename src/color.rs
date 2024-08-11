use crate::{utils, vec::Vec4};

pub type RgbColor = Vec4;

lazy_static! {
    pub static ref BLACK: RgbColor = RgbColor::new(0.0, 0.0, 0.0, 1.0);
    pub static ref WHITE: RgbColor = RgbColor::new(1.0, 1.0, 1.0, 1.0);
    pub static ref RED: RgbColor = RgbColor::new(1.0, 0.0, 0.0, 1.0);
    pub static ref GREEN: RgbColor = RgbColor::new(0.0, 1.0, 0.0, 1.0);
    pub static ref BLUE: RgbColor = RgbColor::new(0.0, 0.0, 1.0, 1.0);
    pub static ref CYAN: RgbColor = RgbColor::new(0.0, 1.0, 1.0, 1.0);
    pub static ref MAGENTA: RgbColor = RgbColor::new(1.0, 0.0, 1.0, 1.0);
    pub static ref YELLOW: RgbColor = RgbColor::new(1.0, 1.0, 0.0, 1.0);
    pub static ref GREY_50: RgbColor = RgbColor::new(0.5, 0.5, 0.5, 1.0);
    pub static ref GREY_20: RgbColor = RgbColor::new(0.2, 0.2, 0.2, 1.0);
    pub static ref GREY_80: RgbColor = RgbColor::new(0.8, 0.8, 0.8, 1.0);
    pub static ref CYBER_COOL_BLUE: RgbColor = RgbColor::new(0.5, 0.5, 0.7, 1.0);
}

impl Default for RgbColor {
    fn default() -> RgbColor {
        RgbColor::zero()
    }
}

impl RgbColor {
    pub fn from_frgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Vec4::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    pub fn to_sdl_rgba(&self) -> sdl2::pixels::Color {
        let c = self.to_rgba();
        sdl2::pixels::Color::RGBA(c[0], c[1], c[2], c[3])
    }

    pub fn with_alpha(&self, w: f32) -> Self {
        RgbColor::new(self.x(), self.y(), self.z(), utils::clamp(w))
    }

    pub fn dot_rgb(&self, other: RgbColor) -> f32 {
        let dot = self.v[0] * other.v[0] + self.v[1] * other.v[1] + self.v[2] * other.v[2];
        dot
    }

    pub fn to_rgba(&self) -> [u8; 4] {
        let n = self.clamp();

        let out: [u8; 4] = [
            (n.v[0] * 255.0) as u8,
            (n.v[1] * 255.0) as u8,
            (n.v[2] * 255.0) as u8,
            (n.v[3] * 255.0) as u8,
        ];
        out
    }

    pub fn blend_normal(&self, other: RgbColor) -> RgbColor {
        let a = utils::clamp(other.v[3]);
        let bg = self.clone();
        let out = bg.mul(1.0 - a) + other.mul(a);
        out
    }
}
