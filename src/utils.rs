// use crate::gfx::color::RgbColor;

use std::time::{Duration, Instant};

pub(crate) fn clamp(x: f32) -> f32 {
    if x < 0.0 {
        return 0.0;
    }
    if x > 1.0 {
        return 1.0;
    }
    x
}

pub fn alloc_vec(size: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(size);
    unsafe {
        v.set_len(size);
    }
    v
}

pub(crate) struct DirectoryReader {
    path: String,
}

impl DirectoryReader {
    pub(crate) fn new(path: &str) -> Self {
        Self {
            path: path.to_owned(),
        }
    }

    pub(crate) fn list(&self) -> Vec<String> {
        let mut output = Vec::new();
        let files = std::fs::read_dir(&self.path);
        if files.is_err() {
            return Vec::new();
        }
        let files = files.unwrap();

        for file in files {
            if file.is_err() {
                continue;
            }

            let image_file = file.unwrap();
            let meta = image_file.metadata();
            if meta.is_err() {
                continue;
            }
            let meta = meta.unwrap();
            if !meta.is_file() {
                continue;
            }
            let modfied_tm = if let Ok(tm) = meta.modified() {
                tm.elapsed().expect("elapsed time").as_millis()
            } else {
                0
            };

            output.push((image_file.file_name(), modfied_tm));
        }
        output.sort_by(|a, b| a.1.cmp(&b.1));
        let v = output
            .iter()
            .map(|e| {
                format!(
                    "{}/{}",
                    self.path,
                    <std::ffi::OsString as Clone>::clone(&e.0)
                        .into_string()
                        .unwrap()
                )
            })
            .collect();
        v
    }
}

pub(crate) struct GameTimer {
    tm: Instant,
    period: Duration,
}

impl GameTimer {
    pub(crate) fn new(period: Duration) -> Self {
        let tm = Instant::now();
        Self { tm, period }
    }

    pub(crate) fn blink(&self) -> bool {
        let now = Instant::now();
        let elapsed = (now - self.tm).as_millis();
        let number_of_periods = elapsed / self.period.as_millis();
        number_of_periods & 0x1 == 0x1
    }

    pub(crate) fn range(&self) -> f32 {
        let now = Instant::now();
        let elapsed = (now - self.tm).as_millis();
        let period = self.period.as_millis();
        let left = elapsed % period;
        left as f32 / period as f32
    }
}
