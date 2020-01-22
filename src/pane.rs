use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureQuery, WindowCanvas};
use sdl2::ttf::Font;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::rc::Rc;

extern crate unicode_segmentation;
use unicode_segmentation::UnicodeSegmentation;

pub enum PaneType {
    Buffer,
    FileManager,
}

#[derive(Hash, PartialEq)]
struct FontCacheKey {
    c: String,
    color: Color,
}

struct FontCacheEntry {
    texture: Texture,
    w: u32,
    h: u32,
}

impl Eq for FontCacheKey {}

pub struct Pane<'a> {
    pub pane_type: PaneType,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub buffer_id: usize,
    pub scroll_idx: usize,
    pub scroll_offset: i32,
    pub row_height: i32,
    col_width: i32,
    pub font: Font<'a, 'static>,
    font_cache: HashMap<FontCacheKey, Rc<FontCacheEntry>>,
}

impl<'a> Pane<'a> {
    pub fn new(font: Font<'a, 'static>) -> Self {
        Pane {
            pane_type,
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            buffer_id,
            scroll_idx: 0,
            scroll_offset: 0,
            row_height: font.height(),
            col_width: font.size_of_char('W').unwrap().0 as i32,
            font,
            font_cache: HashMap::new(),
        }
    }

    pub fn draw(
        &mut self,
        mut canvas: &mut WindowCanvas,
        text: Vec<Vec<String>>,
    ) {
        for (rownum, row) in text.iter().enumerate() {
            for (colnum, col) in row.iter().enumerate() {

                let key = FontCacheKey {
                    c: col.to_string(),
                    color,
                };
                let tex = self.font_cache.get(&key).cloned().unwrap_or_else(|| {
                    let surface = self.font.render(&col.to_string()).blended(color).unwrap();
                    let texture = canvas
                        .texture_creator()
                        .create_texture_from_surface(&surface)
                        .unwrap();
                    let TextureQuery { width, height, .. } = texture.query();
                    let resource = Rc::new(FontCacheEntry {
                        texture,
                        w: width,
                        h: height,
                    });
                    self.font_cache.insert(key, resource.clone());
                    resource
                });
                let texture = &tex.texture;
                let w = min(self.w as i32 - (x + self.col_width) as i32, tex.w as i32) as u32;
                let h = min(self.h as i32 - y as i32, tex.h as i32) as u32;
                let source = Rect::new(0, 0, w, h);
                let target = Rect::new(self.x + x, self.y + y, w, h);
                canvas.copy(&texture, Some(source), Some(target)).unwrap();
            }
        }
    }
}
