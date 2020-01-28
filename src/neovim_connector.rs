extern crate neovim_lib;

use std::process::Command;
use neovim_lib::{Value, Neovim, NeovimApi, Session, UiAttachOptions, Handler, RequestHandler};
use std::sync::mpsc::{Sender, Receiver};
use std::env::Args;

#[derive(Debug)]
pub struct GridCell {
    pub text: String,
    pub highlight: i64,
    pub repeat: i64,
}

#[derive(Debug)]
pub struct GridLine {
    pub grid: i64,
    pub row: i64,
    pub col: i64,
    pub cells: Vec<GridCell>,
}

#[derive(Debug)]
pub struct GridScroll {
    pub grid: i64,
    pub top: i64,
    pub bot: i64,
    pub left: i64,
    pub right: i64,
    pub rows: i64,
    pub cols: i64,
}

#[derive(Debug)]
pub enum NvimEvent {
    GridLine(Vec<GridLine>),
    GridCursorGoto(i64, i64, i64),
    GridClear(i64),
    GridScroll(GridScroll),
    Flush,
    Close,
}

pub struct NvimBridge {
    tx: Sender<NvimEvent>,
}

impl NvimBridge {
    pub fn new(tx: Sender<NvimEvent>) -> Self {
        Self { tx }
    }
}

impl RequestHandler for NvimBridge {
    fn handle_request(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, Value> {
        println!("Unknown request: {}", name);
        Err("Unkown request".into())
    }
}

fn parse_grid_cells(entry: Vec<Value>) -> Vec<GridCell> {
    let mut cells = Vec::new();
    for cell in entry {
        let cell = cell.as_array().unwrap();
        let text = cell[0].as_str().unwrap_or("ERR").to_string();
        let highlight = if cell.len() >= 2 {
            cell[1].as_i64().unwrap_or(-1)
        } else {
            -1
        };
        let repeat = if cell.len() >= 3 {
            cell[2].as_i64().unwrap_or(1)
        } else {
            1
        };
        cells.push(GridCell { text, highlight, repeat });
    }
    cells
}

fn parse_gridline_event(event: Vec<Value>) -> Vec<GridLine> {
    let mut entries = Vec::new();
    for line in event {
        if let Value::Array(val) = line {
            entries.push(GridLine {
                grid: val[0].as_i64().unwrap(),
                row: val[1].as_i64().unwrap(),
                col: val[2].as_i64().unwrap(),
                cells: parse_grid_cells(val[3].as_array().unwrap().to_vec()),
            });
        }
    }
    entries
}

impl Handler for NvimBridge {
    fn handle_notify(&mut self, name: &str, args: Vec<Value>) {
        match name {
            "redraw" => {
                for event in &args {
                    if let Value::Array(event) = event {
                        if let Value::String(event_name) = &event[0] {
                            match event_name.as_str().unwrap() {
                                "grid_line" => self.tx.send(NvimEvent::GridLine(parse_gridline_event(event.to_vec()))).unwrap(),
                                "flush" => self.tx.send(NvimEvent::Flush).unwrap(),
                                "grid_cursor_goto" => {
                                    let goto_args = event[1].as_array().unwrap();
                                    let grid = goto_args[0].as_i64().unwrap();
                                    let row = goto_args[1].as_i64().unwrap();
                                    let col = goto_args[2].as_i64().unwrap();
                                    self.tx.send(NvimEvent::GridCursorGoto(grid, row, col)).unwrap();
                                }
                                "clear" => self.tx.send(NvimEvent::GridClear(event[1].as_i64().unwrap())).unwrap(),
                                "grid_scroll" => {
                                    let scroll_args = event[1].as_array().unwrap();
                                    self.tx.send(NvimEvent::GridScroll(GridScroll {
                                        grid: scroll_args[0].as_i64().unwrap(),
                                        top: scroll_args[1].as_i64().unwrap(),
                                        bot: scroll_args[2].as_i64().unwrap(),
                                        left: scroll_args[3].as_i64().unwrap(),
                                        right: scroll_args[4].as_i64().unwrap(),
                                        rows: scroll_args[5].as_i64().unwrap(),
                                        cols: scroll_args[6].as_i64().unwrap(),
                                    })).unwrap();
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => println!("Unknown notify: {} {:?}", name, args)
        }
    }
    fn handle_close(&mut self) {
        self.tx.send(NvimEvent::Close).unwrap();
    }
}

pub fn start(tx: Sender<NvimEvent>, rx: Receiver<String>, args: Args) {
    let mut cmd = Command::new("nvim");
    cmd.arg("--embed");
    let args: Vec<String> = args.collect();
    for arg in &args[1..] {
        cmd.arg(arg);
    }
    let bridge = NvimBridge::new(tx);
    let mut session = Session::new_child_cmd(&mut cmd).unwrap();
    session.start_event_loop_handler(bridge);
    let mut nvim = Neovim::new(session);
    let mut ui_opts = UiAttachOptions::new();
    ui_opts.set_rgb(true);
    ui_opts.set_linegrid_external(true);
    ui_opts.set_popupmenu_external(false);
    ui_opts.set_tabline_external(false);
    ui_opts.set_cmdline_external(false);
    ui_opts.set_wildmenu_external(false);
    nvim.ui_attach(80, 30, &ui_opts).unwrap();

    loop {
        if let Ok(s) = rx.recv() {
            nvim.input(&s).unwrap();
        }
    }
}
