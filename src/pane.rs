use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureQuery, WindowCanvas};
use sdl2::ttf::Font;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Instant, Duration};

use crate::neovim_connector::Highlight;

#[derive(Hash, PartialEq)]
struct FontCacheKey {
    c: String,
    color: Color,
}

#[derive(Clone)]
pub struct TextCell {
    pub text: String,
    pub hl_id: i64,
}

impl TextCell {
    pub fn new() -> Self {
        Self {
            text: " ".into(),
            hl_id: 0,
        }
    }
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
    pub row_height: u32,
    bg_color: Color,
    fg_color: Color,
    special_color: Color,
    col_width: u32,
    pub font: Font<'a, 'static>,
    font_cache: HashMap<FontCacheKey, Rc<FontCacheEntry>>,
}

fn parse_color(c: i64) -> Color {
    Color::RGB(
        ((c & 0xff0000) >> 16) as u8,
        ((c & 0x00ff00) >> 8) as u8,
        (c & 0x0000ff) as u8,
    )
}

impl<'a> Pane<'a> {
    pub fn set_colors(&mut self, fg: i64, bg: i64, special: i64) {
        self.bg_color = parse_color(bg);
        self.fg_color = parse_color(fg);
        self.special_color = parse_color(special);
    }

    pub fn new(font: Font<'a, 'static>) -> Self {
        Pane {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            scroll_idx: 0,
            scroll_offset: 0,
            row_height: font.height() as u32,
            col_width: font.size_of_char('W').unwrap().0,
            cursor_row: 0,
            cursor_col: 0,
            bg_color: Color::RGB(0, 0, 0),
            fg_color: Color::RGB(0, 255, 0),
            special_color: Color::RGB(0, 0, 255),
            font,
            font_cache: HashMap::new(),
        }
    }

    pub fn draw(
        &mut self,
        canvas: &mut WindowCanvas,
        text: &[Vec<TextCell>],
        highlight_table: &HashMap<i64, Highlight>,
    ) {
        let char_rect = Rect::new(0, 0, self.col_width, self.row_height);
        let mut color = self.fg_color;
        canvas.set_draw_color(self.bg_color);
        canvas.clear();

        for (rownum, row) in text.iter().enumerate() {
            for (colnum, col) in row.iter().enumerate() {
                let last_color = color;
                color = if highlight_table.contains_key(&col.hl_id) {
                    let fg = highlight_table[&col.hl_id].fg;
                    if fg == -1 {
                        self.fg_color
                    } else {
                        parse_color(fg)
                    }
                } else {
                    self.fg_color
                };
                if color != last_color {
                    canvas.set_draw_color(color);
                }

                let key = FontCacheKey {
                    c: col.text.to_string(),
                    color,
                };
                let tex = self.font_cache.get(&key).cloned().unwrap_or_else(|| {
                    let surface = self
                        .font
                        .render(&col.text.to_string())
                        .blended(color)
                        .unwrap();
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

                let target = Rect::new(
                    self.x + colnum as i32 * self.col_width as i32,
                    self.y + rownum as i32 * self.row_height as i32,
                    self.col_width as u32,
                    self.row_height as u32,
                );
                if highlight_table.contains_key(&col.hl_id) {
                    let color = highlight_table[&col.hl_id].bg;
                    if color != -1 {
                        canvas.set_draw_color(parse_color(color));
                        canvas.fill_rect(target).unwrap();
                    }
                }
                canvas
                    .copy(&texture, Some(char_rect), Some(target))
                    .unwrap();
            }
        }

        canvas.set_draw_color(self.fg_color);
        let cursor_rect = Rect::new(
            self.x + self.cursor_col * self.col_width as i32,
            self.y + self.cursor_row * self.row_height as i32,
            2,
            self.row_height as u32,
        );
        canvas.fill_rect(cursor_rect).unwrap();
    }
}
