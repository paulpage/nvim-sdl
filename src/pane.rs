use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureQuery, WindowCanvas};
use sdl2::ttf::Font;
use std::collections::HashMap;
use std::rc::Rc;

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
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub cursor_row: i32,
    pub cursor_col: i32,
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
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            scroll_idx: 0,
            scroll_offset: 0,
            row_height: font.height(),
            col_width: font.size_of_char('W').unwrap().0 as i32,
            cursor_row: 0,
            cursor_col: 0,
            font,
            font_cache: HashMap::new(),
        }
    }

    pub fn draw(
        &mut self,
        canvas: &mut WindowCanvas,
        text: &Vec<Vec<String>>,
    ) {
        let bg_color = Color::RGB(0, 0, 50);
        canvas.set_draw_color(bg_color);
        canvas.clear();
        let fg_color = Color::RGB(253, 244, 193);
        canvas.set_draw_color(fg_color);
        for (rownum, row) in text.iter().enumerate() {
            // println!();
            for (colnum, col) in row.iter().enumerate() {
                // print!("{}", col);

                let key = FontCacheKey {
                    c: col.to_string(),
                    color: fg_color,
                };
                let tex = self.font_cache.get(&key).cloned().unwrap_or_else(|| {
                    let surface = self.font.render(&col.to_string()).blended(fg_color).unwrap();
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
                let w = self.col_width;
                let h = self.row_height;
                // let w = min(self.w as i32 - (colnum as i32 + self.col_width as i32) as i32, tex.w as i32) as u32;
                // let h = min(self.h as i32 - rownum as i32, tex.h as i32) as u32;
                let source = Rect::new(0, 0, w as u32, h as u32);
                let target = Rect::new(self.x + colnum as i32 * self.col_width as i32, self.y + rownum as i32 * self.row_height as i32, w as u32, h as u32);
                canvas.copy(&texture, Some(source), Some(target)).unwrap();
            }
        }

        let cursor_rect = Rect::new(self.x + self.cursor_col * self.col_width, self.y + self.cursor_row * self.row_height, 2, self.row_height as u32);
        canvas.fill_rect(cursor_rect);
    }
}
