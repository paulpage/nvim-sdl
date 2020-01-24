extern crate neovim_lib;

use std::process::Command;
use neovim_lib::{Value, Neovim, NeovimApi, Session, UiAttachOptions, Handler, RequestHandler};
use std::sync::mpsc::{Sender, Receiver};
use std::env::Args;

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

fn parse_nvim_value(v: &Value) {
    match v {
        Value::Nil => (),
        Value::Boolean(b) => println!("Bool: {}", b),
        Value::Integer(i) => println!("Int: {}", i),
        Value::F32(f) => println!("Float: {}", f),
        Value::F64(f) => println!("Float: {}", f),
        Value::String(s) => println!("String: {}", s),
        Value::Binary(b) => {
            println!("Binary:");
            for x in b {
                println!("{}", x);
            }
        }
        Value::Array(v) => {
            println!("Value:");
            for val in v {
                parse_nvim_value(val);
            }
        }
        Value::Map(vv) => {
            println!("Map:");
            for (k, v) in vv {
                println!("Key:");
                parse_nvim_value(k);
                println!("Value:");
                parse_nvim_value(v);
            }
        }
        Value::Ext(i, uu) => {
            println!("Ext: {}", i);
            for u in uu {
                println!("{}", u);
            }
        }
    }
}

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
pub enum NvimEvent {
    GridLine(Vec<GridLine>),
    GridCursorGoto(i64, i64, i64),
    GridClear(i64),
    Flush,
    Close,
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
        match line {
            Value::Array(val) => {
                entries.push(GridLine {
                    grid: val[0].as_i64().unwrap(),
                    row: val[1].as_i64().unwrap(),
                    col: val[2].as_i64().unwrap(),
                    cells: parse_grid_cells(val[3].as_array().unwrap().to_vec()),
                });
            }
            _ => {},
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
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => println!("Unknown notify: {} {:?}", name, args)
        }
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
        match rx.recv() {
            Ok(s) => {
                nvim.input(&s).unwrap();
            }
            Err(_) => {}
        }
    }
}
