use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use sdl2::{
    image::LoadTexture,
    rect::Rect,
    render::{Canvas, Texture},
    video::Window,
};

use tracing;

pub(crate) struct TextureCache {
    lookup: HashMap<String, Vec<TexInfo>>,
}

#[derive(Clone)]
pub(crate) struct TexInfo {
    pub(crate) texture: Arc<RwLock<Texture>>,
    w: u32,
    h: u32,
    last_modified: u128, // last modified time in ms
    pub(crate) original_aspect: f32,
}

impl TextureCache {
    pub(crate) fn new() -> Self {
        Self {
            lookup: HashMap::new(),
        }
    }

    fn find_dim(tex_infos: &Vec<TexInfo>, w: u32, h: u32) -> Option<&TexInfo> {
        for tex in tex_infos {
            if tex.w == w && tex.h == h {
                return Some(tex);
            }
        }
        return None;
    }

    fn get(&self, name: &String, w: u32, h: u32, tm: Option<u128>) -> Option<TexInfo> {
        let tex_info = self.lookup.get(name);
        if tex_info.is_none() {
            return None;
        }

        let tex_info = Self::find_dim(tex_info.unwrap(), w, h);
        if tex_info.is_none() {
            return None;
        }
        let tex_info = tex_info.unwrap();

        if let Some(modified) = tm {
            if modified != tex_info.last_modified {
                return None;
            }
        }
        if w != tex_info.w {
            return None;
        }
        if h != tex_info.h {
            return None;
        }

        Some(tex_info.clone())
    }

    pub(crate) fn load_texture(
        &mut self,
        canvas: &mut Canvas<Window>,
        name: String,
        w: u32,
        h: u32,
        last_modified: Option<u128>,
    ) -> Result<TexInfo, String> {
        let tex = self.get(&name, w, h, last_modified);
        if tex.is_some() {
            return Ok(tex.unwrap());
        }
        let tc = canvas.texture_creator();
        let src_texture = tc.load_texture(&name)?;
        let original_aspect = src_texture.query().width as f32 / src_texture.query().height as f32;
        let mut dst_texture = tc
            .create_texture(
                src_texture.query().format,
                sdl2::render::TextureAccess::Target,
                w,
                h,
            )
            .expect("can't create texture");
        let dst = Rect::new(0, 0, w, h);
        let result = canvas.with_texture_canvas(&mut dst_texture, |texture_canvas| {
            texture_canvas
                .copy(&src_texture, None, dst)
                .expect("can't copy/scale texture");
        });
        if result.is_err() {
            let err_msg = format!("load_texture: {}", result.err().unwrap());
            tracing::error!(err_msg);
            return Err(err_msg);
        }

        let tex_info = TexInfo {
            texture: Arc::new(RwLock::new(dst_texture)),
            w,
            h,
            last_modified: last_modified.unwrap_or(0),
            original_aspect,
        };

        self.lookup
            .entry(name.clone())
            .and_modify(|e| e.push(tex_info.clone()))
            .or_insert(vec![tex_info]);

        let tex = self.get(&name, w, h, last_modified);
        let tex = tex.unwrap();

        return Ok(tex);
    }
}
