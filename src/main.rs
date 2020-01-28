extern crate clipboard;
extern crate sdl2;

use std::env;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use std::thread;
use std::sync::mpsc::channel;
// use std::time::Instant;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::MouseButton;
use sdl2::render::WindowCanvas;

mod pane;
use pane::Pane;

mod neovim_connector;
use neovim_connector::{NvimEvent, ClientEvent};

struct InputState {
    alt_down: bool,
    ctrl_down: bool,
    shift_down: bool,
    mouse_row: i32,
    mouse_col: i32,
}

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

    let (server_sender, server_receiver) = channel();
    let (client_sender, client_receiver) = channel();
    thread::spawn(move|| {
        neovim_connector::start(server_sender, client_receiver, env::args());
    });

    let path = match select_font() {
        Some(p) => p,
        None => PathBuf::new(),
    };

    let mut input = InputState {
        alt_down: false,
        ctrl_down: false,
        shift_down: false,
        mouse_row: 0,
        mouse_col: 0,
    };

    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();
    let window = video_subsys
        .window("Neovim", 800, 600)
        .position_centered()
        .resizable()
        .maximized()
        .opengl()
        .build()
        .unwrap();
    let mut canvas: WindowCanvas = window.into_canvas().build().unwrap();

    let mut text = vec![vec![" ".to_string(); 80]; 30];

    let mut pane = Pane::new(
        ttf_context.load_font(&path, 16).unwrap()
    );

    let font = ttf_context.load_font(&path, 16).unwrap();
    let col_width = font.size_of_char('W').unwrap().0 as i32;
    let row_height = font.height();

    let mut ctrl_pressed = false;
    let mut alt_pressed = false;

    'mainloop: loop {
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            match event {
                Event::Quit { .. } => break 'mainloop,
                Event::KeyDown { keycode: Some(kc), keymod, .. } => {
                    input.shift_down = keymod.contains(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
                    input.ctrl_down = keymod.contains(Mod::LCTRLMOD | Mod::RCTRLMOD);
                    input.alt_down = keymod.contains(Mod::LALTMOD | Mod::RALTMOD);
                    if kc == Keycode::F1 {
                        client_sender.send(ClientEvent::Mouse {
                            button: "wheel".into(),
                            action: "down".into(),
                            modifier: "".into(),
                            grid: 0,
                            row: 0,
                            col: 0,
                        });
                    }
                    let key_to_send = match kc {
                        Keycode::Return => "CR",
                        Keycode::Backspace => "BS",
                        Keycode::Escape => "ESC",
                        Keycode::Tab => "Tab",
                        _ => {
                            // println!("Unimplemented keycode: {}", kc);
                            ""
                        },
                    };
                    if key_to_send != "" {
                        client_sender.send(ClientEvent::Text(
                            format!("<{}{}{}{}>",
                                if input.alt_down { "M-" } else { "" },
                                if input.ctrl_down { "C-" } else { "" },
                                if input.shift_down { "S-" } else { "" },
                                key_to_send,
                            )
                            // key_to_send.to_string()
                        )).unwrap();
                    }
                }
                Event::TextInput { text, .. } => {
                    // let is_modified = input.alt_down || input.ctrl_down || input.shift_down;
                    // if is_modified {
                    //     client_sender.send(ClientEvent::Text(
                    //             format!("<{}{}{}{}>",
                    //                 if input.alt_down { "M-" } else { "" },
                    //                 if input.ctrl_down { "C-" } else { "" },
                    //                 if input.shift_down { "S-" } else { "" },
                    //                 text.to_lowercase(),
                    //             )
                    //     )).unwrap();
                    // } else {
                        client_sender.send(ClientEvent::Text(text)).unwrap();
                    // }
                }
                Event::KeyUp { keymod, .. } => {
                    input.shift_down = keymod.contains(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
                    input.ctrl_down = keymod.contains(Mod::LCTRLMOD | Mod::RCTRLMOD);
                    input.alt_down = keymod.contains(Mod::LALTMOD | Mod::RALTMOD);
                }
                // Event::Window { win_event, .. } => {
                // }
                Event::MouseButtonDown { x, y, clicks, mouse_btn, .. } => {
                    input.mouse_col = x / col_width;
                    input.mouse_row = y / row_height;
                    let button = match mouse_btn {
                        MouseButton::Left => "left",
                        MouseButton::Right => "right",
                        MouseButton::Middle => "middle",
                        _ => "",
                    };
                    if button != "" {
                        for _ in 0..clicks {
                            client_sender.send(ClientEvent::Mouse {
                                button: button.into(),
                                action: "press".into(),
                                modifier: "".into(), // TODO
                                grid: 0,
                                col: input.mouse_col.into(),
                                row: input.mouse_row.into(),
                            }).unwrap();
                        }
                    }
                }
                Event::MouseMotion {
                    mousestate, x, y, ..
                } => {
                    input.mouse_col = x / col_width;
                    input.mouse_row = y / row_height;
                    // TODO: drag
                }
                Event::MouseWheel { x, y, .. } => {
                    let action = match (x, y) {
                        (0, 1) => "up",
                        (0, -1) => "down",
                        (1, 0) => "right",
                        (-1, 0) => "left",
                        _ => "",
                    };
                    client_sender.send(ClientEvent::Mouse {
                        button: "wheel".into(),
                        action: action.into(),
                        modifier: "".into(), // TODO
                        grid: 0,
                        col: input.mouse_col.into(),
                        row: input.mouse_row.into(),
                    }).unwrap();
                }
                _ => {}
            }
        }

        let has_received = match server_receiver.try_recv() {
            Ok(e) => {
                match e {
                    NvimEvent::GridLine(entries) => {
                        for entry in entries {
                            let row = entry.row as usize;
                            let mut col = entry.col as usize;
                            for cell in entry.cells {
                                for _ in 0..cell.repeat {
                                    text[row][col] = cell.text.clone();
                                    col += 1;
                                }
                            }
                        }
                        false
                    }
                    NvimEvent::Flush => true,
                    NvimEvent::GridCursorGoto(_grid, row, col) => {
                        pane.cursor_row = row as i32;
                        pane.cursor_col = col as i32;
                        false
                    }
                    NvimEvent::GridClear(_) => {
                        text = vec![vec![" ".to_string(); 80]; 30];
                        false
                    }
                    NvimEvent::GridScroll(e) => {
                        if e.rows > 0 {
                            for y in e.top..e.bot {
                                for x in e.left..e.right {
                                    let y_idx = y - e.rows;
                                    if y_idx >= 0 {
                                        text[y_idx as usize][x as usize] = text[y as usize][x as usize].clone();
                                        text[y as usize][x as usize] = " ".to_string();
                                    }
                                }
                            }
                        } else {
                            for y in (e.top..e.bot-1).rev() {
                                for x in (e.left..e.right).rev() {
                                    let y_idx = (y - e.rows) as usize;
                                    if y_idx < text.len() {
                                        text[y_idx as usize][x as usize] = text[y as usize][x as usize].clone();
                                        text[y as usize][x as usize] = " ".to_string();
                                    }
                                }
                            }
                        }
                        false
                    }
                    NvimEvent::Close => {
                        break 'mainloop;
                    }
                }
            }
            Err(_) => {
                false
            }
        };

        if has_received {
            pane.draw(&mut canvas, &text);
            canvas.present();
        }
        sleep(Duration::from_millis(1));

    }
}
