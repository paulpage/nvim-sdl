extern crate clipboard;
extern crate sdl2;

use std::env;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use std::thread;
use std::sync::mpsc::channel;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::WindowCanvas;

mod pane;
use pane::Pane;

mod neovim_connector;
use neovim_connector::NvimEvent;

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

    let mut ctrl_pressed = false;
    let mut alt_pressed = false;

    'mainloop: loop {
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            match event {
                Event::Quit { .. } => break 'mainloop,
                Event::KeyDown { keycode: Some(kc), keymod, .. } => {
                    let key_to_send = match kc {
                        Keycode::Return => "<CR>",
                        Keycode::Backspace => "<BS>",
                        Keycode::Escape => "<ESC>",
                        _ => {
                            println!("Unimplemented keycode: {}", kc);
                            ""
                        },
                    };
                    if key_to_send != "" {
                        client_sender.send(key_to_send.to_string()).unwrap();
                    }
                    // match kc {
                    //     Keycode::Return => client_sender.send("<CR>".to_string()).unwrap(),
                    //     Keycode::Backspace => client_sender.send("<BS>".to_string()).unwrap(),
                    //     Keycode::Escape => client_sender.send("<ESC>".to_string()).unwrap(),
                    //     _ => println!("Unhandled keycode: {:?}", kc),
                    // }
                }
                Event::TextInput { text, .. } => {
                    client_sender.send(text).unwrap();
                }
                // Event::KeyUp { keymod, .. } => {
                // }
                // Event::Window { win_event, .. } => {
                // }
                // Event::TextInput { text, .. } => {},
                // Event::MouseButtonDown { x, y, clicks, .. } => {
                // }
                // Event::MouseMotion {
                //     mousestate, x, y, ..
                // } => {
                // }
                // Event::MouseWheel { y, .. } => pane.scroll(buffer, y * -5),
                // Event::KeyDown { .. } => {}
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
                    NvimEvent::Close => {
                        break 'mainloop;
                    }
                }
            }
            Err(_) => {
                false
            }
        };

        // if has_received {
            pane.draw(&mut canvas, &text);
            canvas.present();
        // }

        sleep(Duration::from_millis(5));
    }
}
