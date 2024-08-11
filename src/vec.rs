use std::ops::{Add, Sub};

use crate::utils;

#[derive(Debug, Clone)]
pub struct Vec3 {
    pub(crate) v: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct Vec4 {
    pub(crate) v: [f32; 4],
}

impl Vec3 {
    #[must_use]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { v: [x, y, z] }
    }

    #[must_use]
    pub fn zero() -> Self {
        Self { v: [0.0, 0.0, 0.0] }
    }

    #[must_use]
    pub fn dot(&self, other: Vec3) -> f32 {
        self.v[0] * other.v[0] + self.v[1] * other.v[1] + self.v[2] * other.v[2]
    }

    #[must_use]
    pub fn clamp(&self) -> Self {
        Vec3::new(
            utils::clamp(self.v[0]),
            utils::clamp(self.v[1]),
            utils::clamp(self.v[2]),
        )
    }

    #[must_use]
    pub fn mul(&self, c: f32) -> Self {
        Vec3::new(self.v[0] * c, self.v[1] * c, self.v[2] * c)
    }

    #[must_use]
    pub fn x(&self) -> f32 {
        self.v[0]
    }

    #[must_use]
    pub fn y(&self) -> f32 {
        self.v[1]
    }

    #[must_use]
    pub fn z(&self) -> f32 {
        self.v[2]
    }
}

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Self) -> Self::Output {
        Vec3::new(
            self.v[0] + rhs.v[0],
            self.v[1] + rhs.v[1],
            self.v[2] + rhs.v[2],
        )
    }
}

impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec3::new(
            self.v[0] - rhs.v[0],
            self.v[1] - rhs.v[1],
            self.v[2] - rhs.v[2],
        )
    }
}

impl Vec4 {
    #[must_use]
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { v: [x, y, z, w] }
    }

    #[must_use]
    pub fn zero() -> Self {
        Self {
            v: [0.0, 0.0, 0.0, 0.0],
        }
    }

    #[must_use]
    pub fn dot(&self, other: Vec4) -> f32 {
        self.v[0] * other.v[0]
            + self.v[1] * other.v[1]
            + self.v[2] * other.v[2]
            + self.v[3] * other.v[3]
    }

    #[must_use]
    pub fn clamp(&self) -> Self {
        Vec4::new(
            utils::clamp(self.v[0]),
            utils::clamp(self.v[1]),
            utils::clamp(self.v[2]),
            utils::clamp(self.v[3]),
        )
    }

    #[must_use]
    pub fn mul(&self, c: f32) -> Self {
        Vec4::new(self.v[0] * c, self.v[1] * c, self.v[2] * c, self.v[3] * c)
    }

    #[must_use]
    pub fn x(&self) -> f32 {
        self.v[0]
    }

    #[must_use]
    pub fn y(&self) -> f32 {
        self.v[1]
    }

    #[must_use]
    pub fn z(&self) -> f32 {
        self.v[2]
    }

    #[must_use]
    pub fn w(&self) -> f32 {
        self.v[3]
    }
}

impl Add for Vec4 {
    type Output = Vec4;

    fn add(self, rhs: Self) -> Self::Output {
        Vec4::new(
            self.v[0] + rhs.v[0],
            self.v[1] + rhs.v[1],
            self.v[2] + rhs.v[2],
            self.v[3] + rhs.v[3],
        )
    }
}

impl Sub for Vec4 {
    type Output = Vec4;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec4::new(
            self.v[0] - rhs.v[0],
            self.v[1] - rhs.v[1],
            self.v[2] - rhs.v[2],
            self.v[3] - rhs.v[3],
        )
    }
}
