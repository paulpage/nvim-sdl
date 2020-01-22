extern crate clipboard;
extern crate sdl2;

use std::cmp::max;
use std::env;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Mod;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::render::WindowCanvas;

mod pane;
use pane::Pane;

fn select_font() -> Option<PathBuf> {
    match font_kit::source::SystemSource::new().select_best_match(
        &[font_kit::family_name::FamilyName::Monospace],
        &font_kit::properties::Properties::new(),
    ) {
        Ok(font_kit::handle::Handle::Path { path, .. }) => Some(path),
        _ => None,
    }
}

fn main() {
    let path = match select_font() {
        Some(p) => p,
        None => PathBuf::new(),
    };

    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();
    let window = video_subsys
        .window("SDL2_TTF Example", 800, 600)
        .position_centered()
        .resizable()
        .maximized()
        .opengl()
        .build()
        .unwrap();
    let mut canvas: WindowCanvas = window.into_canvas().build().unwrap();

    let mut text = vec![vec![" ".to_string(); 80]; 30];

    let args: Vec<String> = env::args().collect();

    let mut pane = Pane::new(
        ttf_context.load_font(&path, 16).unwrap()
    ));
    arrange(&canvas, &mut panes);

    let mut ctrl_pressed = false;
    let mut alt_pressed = false;
    let mut needs_redraw;

    'mainloop: loop {
        needs_redraw = false;
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            needs_redraw = true;
            if let Event::KeyDown {
                keycode: Some(kc),
                keymod,
                ..
            } = event
            {
                let mut key_string = String::new();
                if keymod.contains(Mod::RCTRLMOD) || keymod.contains(Mod::LCTRLMOD) {
                    key_string.push_str("C-");
                    ctrl_pressed = true;
                }
                if keymod.contains(Mod::RALTMOD) || keymod.contains(Mod::LALTMOD) {
                    key_string.push_str("A-");
                    alt_pressed = true;
                }
                if keymod.contains(Mod::RSHIFTMOD) || keymod.contains(Mod::LSHIFTMOD) {
                    key_string.push_str("S-");
                }
                key_string.push_str(&kc.name());

                let kstr: &str = &key_string.clone();
                match kstr {
                    "C-'" => {
                        panes.push(Pane::new(
                            ttf_context.load_font(&path, 16).unwrap(),
                            PaneType::Buffer,
                            0,
                        ));
                        arrange(&canvas, &mut panes);
                        pane_idx += 1;
                    }
                    "C-B" => panes[pane_idx].buffer_id = next(&buffers, panes[pane_idx].buffer_id),
                    "C-S-B" => {
                        panes[pane_idx].buffer_id = prev(&buffers, panes[pane_idx].buffer_id)
                    }
                    "C-J" => pane_idx = next(&panes, pane_idx),
                    "C-K" => pane_idx = prev(&panes, pane_idx),
                    "C-Q" => break 'mainloop,
                    "C-O" => {
                        let mut buffer = Buffer::new();
                        fm.current_dir = env::current_dir().unwrap();
                        fm.update(&mut buffer);
                        let pane = &mut panes[pane_idx];
                        pane.buffer_id = buffers.len();
                        pane.pane_type = PaneType::FileManager;
                        buffers.push(buffer);
                    }
                    "C-W" => {
                        if panes.len() > 1 {
                            panes.remove(pane_idx);
                            pane_idx = prev(&panes, pane_idx);
                            arrange(&canvas, &mut panes);
                        }
                    }
                    _ => {
                        let pane = &mut panes[pane_idx];
                        let buffer = &mut buffers[pane.buffer_id];
                        match pane.pane_type {
                            PaneType::Buffer => {
                                if pane.handle_keystroke(buffer, kstr) {
                                    break 'mainloop;
                                }
                            }
                            PaneType::FileManager => {
                                fm.handle_key(pane, buffer, kstr);
                            }
                        }
                    }
                }
            } else {
                let pane = &mut panes[pane_idx];
                let mut buffer = &mut buffers[pane.buffer_id];
                match event {
                    Event::Quit { .. } => break 'mainloop,
                    Event::KeyUp { keymod, .. } => {
                        if keymod.contains(Mod::RCTRLMOD) || keymod.contains(Mod::LCTRLMOD) {
                            ctrl_pressed = false;
                        }
                        if keymod.contains(Mod::RALTMOD) || keymod.contains(Mod::LALTMOD) {
                            alt_pressed = false;
                        }
                    }
                    Event::Window { win_event, .. } => {
                        if let WindowEvent::Resized(w, h) = win_event {
                            pane.w = max(0, w - 40) as u32;
                            pane.h = max(0, h - 40) as u32;
                            arrange(&canvas, &mut panes);
                        }
                    }
                    Event::TextInput { text, .. } => match pane.pane_type {
                        PaneType::Buffer => {
                            if !ctrl_pressed && !alt_pressed {
                                buffer.action_insert_text(text);
                            }
                        }
                        PaneType::FileManager => {
                            fm.current_search.push_str(&text);
                            buffer.name = fm.current_search.clone();
                            let mut selection = buffer.cursor_y;
                            'searchloop: for (i, line) in
                                buffer.contents[buffer.cursor_y..].iter().enumerate()
                            {
                                if line.starts_with(&fm.current_search) {
                                    selection = i + buffer.cursor_y;
                                    break 'searchloop;
                                }
                            }
                            buffer.select_line(selection);
                        }
                    },
                    Event::MouseButtonDown { x, y, clicks, .. } => {
                        pane.set_selection_from_screen(&mut buffer, x, y, false);
                        if clicks > 1 {
                            let (x, y) = buffer.prev_word(buffer.cursor_x, buffer.cursor_y);
                            buffer.sel_x = x;
                            buffer.sel_y = y;
                            let (x, y) = buffer.next_word(buffer.cursor_x, buffer.cursor_y);
                            buffer.cursor_x = x;
                            buffer.cursor_y = y;
                        }
                    }
                    Event::MouseMotion {
                        mousestate, x, y, ..
                    } => {
                        if mousestate.is_mouse_button_pressed(MouseButton::Left) {
                            pane.set_selection_from_screen(&mut buffer, x, y, true);
                        }
                    }
                    Event::MouseWheel { y, .. } => pane.scroll(buffer, y * -5),
                    Event::KeyDown { .. } => {}
                    _ => {}
                }
            }
        }

        if needs_redraw {
            draw(&mut panes, &mut buffers, pane_idx, &mut canvas);
            canvas.present();
        }

        sleep(Duration::from_millis(5));
    }
}
