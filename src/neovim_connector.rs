extern crate neovim_lib;

use neovim_lib::{Handler, Neovim, NeovimApi, RequestHandler, Session, UiAttachOptions, Value};
use std::env::Args;
use std::process::Command;
use std::sync::mpsc::{Receiver, Sender};

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

#[derive(Debug, Default)]
pub struct Highlight {
    pub fg: i64,
    pub bg: i64,
    pub special: i64,
    pub reverse: bool,
    pub italic: bool,
    pub bold: bool,
    pub strikethrough: bool,
    pub underline: bool,
    pub undercurl: bool,
    pub blend: i64,
}

impl Highlight {
    fn default() -> Self {
        Self {
            fg: -1,
            bg: -1,
            special: -1,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct ModeInfo {
    cursor_shape: String,
    cell_percentage: i64,
    // TODO cursor-blinking (blinkwait, blinkon, blinkoff)
    attr_id: String,
    attr_id_lm: String,
    short_name: String,
    name: String,
    // TODO mouse_shape (not yet implemented in nvim)
}

#[derive(Debug)]
pub enum NvimMode {
    Normal,
    Insert,
    Command,
}

#[derive(Debug)]
pub enum NvimEvent {
    GridLine(Vec<GridLine>),
    GridCursorGoto(i64, i64, i64),
    GridClear(i64),
    GridScroll(GridScroll),
    DefaultColorsSet { fg: i64, bg: i64, special: i64 },
    Flush,
    Close,
    ModeChange(NvimMode),
    ModeInfoSet(ModeInfo),
    HighlightAttrDefine { id: i64, hl: Highlight },
    GridResize { grid: i64, cols: i64, rows: i64 },
}

pub enum ClientEvent {
    Text(String),
    Mouse {
        button: String,
        action: String,
        modifier: String,
        grid: i64,
        row: i64,
        col: i64,
    },
    WindowResize {
        cols: i64,
        rows: i64,
    },
}

pub struct NvimBridge {
    tx: Sender<NvimEvent>,
}

impl NvimBridge {
    pub fn new(tx: Sender<NvimEvent>) -> Self {
        Self { tx }
    }
}

fn pretty_print_value(v: &Value, indent_level: usize) {
    match v {
        Value::Nil => {
            print!("(nil value)");
        }
        Value::Boolean(b) => {
            print!("{}{}", " ".repeat(indent_level), b);
        }
        Value::Integer(i) => {
            print!("{}{}", " ".repeat(indent_level), i);
        }
        Value::F32(f) => {
            print!("{}{}", " ".repeat(indent_level), f);
        }
        Value::F64(f) => {
            print!("{}{}", " ".repeat(indent_level), f);
        }
        Value::String(s) => {
            print!("{}{}", " ".repeat(indent_level), s);
        }
        Value::Binary(_) => {
            print!("(skipping binary value)");
        }
        Value::Array(vs) => {
            print!("[");
            for v in vs {
                pretty_print_value(v, indent_level + 2);
                print!(", ");
            }
            print!("]");
        }
        Value::Map(vvs) => {
            print!("{{");
            for (k, v) in vvs {
                pretty_print_value(k, indent_level);
                print!(":");
                pretty_print_value(v, indent_level + 2);
            }
            print!(" }}");
        }
        Value::Ext(_, _) => {
            print!("(skipping ext value)");
        }
    }
}

impl RequestHandler for NvimBridge {
    fn handle_request(&mut self, name: &str, _args: Vec<Value>) -> Result<Value, Value> {
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
        cells.push(GridCell {
            text,
            highlight,
            repeat,
        });
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
                                "grid_line" => self
                                    .tx
                                    .send(NvimEvent::GridLine(parse_gridline_event(event.to_vec())))
                                    .unwrap(),
                                "flush" => self.tx.send(NvimEvent::Flush).unwrap(),
                                "grid_cursor_goto" => {
                                    let goto_args = event[1].as_array().unwrap();
                                    let grid = goto_args[0].as_i64().unwrap();
                                    let row = goto_args[1].as_i64().unwrap();
                                    let col = goto_args[2].as_i64().unwrap();
                                    self.tx
                                        .send(NvimEvent::GridCursorGoto(grid, row, col))
                                        .unwrap();
                                }
                                "grid_clear" => self
                                    .tx
                                    .send(NvimEvent::GridClear(
                                        event[1].as_array().unwrap()[0].as_i64().unwrap(),
                                    ))
                                    .unwrap(),
                                // "grid_clear" => println!("CLEAR {:?}", event),
                                "grid_scroll" => {
                                    let scroll_args = event[1].as_array().unwrap();
                                    self.tx
                                        .send(NvimEvent::GridScroll(GridScroll {
                                            grid: scroll_args[0].as_i64().unwrap(),
                                            top: scroll_args[1].as_i64().unwrap(),
                                            bot: scroll_args[2].as_i64().unwrap(),
                                            left: scroll_args[3].as_i64().unwrap(),
                                            right: scroll_args[4].as_i64().unwrap(),
                                            rows: scroll_args[5].as_i64().unwrap(),
                                            cols: scroll_args[6].as_i64().unwrap(),
                                        }))
                                        .unwrap();
                                }
                                "default_colors_set" => {
                                    let color_args = event[1].as_array().unwrap();
                                    self.tx
                                        .send(NvimEvent::DefaultColorsSet {
                                            fg: color_args[0].as_i64().unwrap(),
                                            bg: color_args[1].as_i64().unwrap(),
                                            special: color_args[2].as_i64().unwrap(),
                                        })
                                        .unwrap();
                                }
                                "mouse_on" => {}
                                "mouse_off" => {}
                                "mode_info_set" => {
                                    for e in event {
                                        let mut mode_info = ModeInfo::default();
                                        // let map = e[1].as_map().unwrap();
                                        // for (k, v) in map {
                                        //     match k.as_str().unwrap() {
                                        //         "cursor_shape" => mode_info.cursor_shape = v.as_str().unwrap().into(),
                                        //         "cell_percentage" => mode_info.cell_percentage = v.as_i64().unwrap(),
                                        //         "attr_id" => mode_info.attr_id = v.as_str().unwrap().into(),
                                        //         "attr_id_lm" => mode_info.attr_id_lm = v.as_str().unwrap().into(),
                                        //         "short_name" => mode_info.short_name = v.as_str().unwrap().into(),
                                        //         "name" => mode_info.name = v.as_str().unwrap().into(),
                                        //         _ => {}
                                        //     }
                                        // }
                                        pretty_print_value(e, 0);
                                        println!();
                                    }
                                    // println!("MODE INFO SET: {:?}", event);
                                }
                                "mode_change" => {
                                    // mode_args = event[0].as_array().unwrap();
                                    // println!("MODE CHANGE: {:?}", event);
                                }
                                "hl_attr_define" => {
                                    for hl_definition in event.iter().skip(1) {
                                        let mut hl = Highlight::default();
                                        let args = hl_definition.as_array().unwrap();
                                        let id = args[0].as_i64().unwrap();
                                        let map = args[1].as_map().unwrap();
                                        for (k, v) in map {
                                            match k.as_str().unwrap() {
                                                "foreground" => {
                                                    hl.fg = v.as_i64().unwrap();
                                                }
                                                "background" => {
                                                    hl.bg = v.as_i64().unwrap();
                                                }
                                                "special" => {
                                                    hl.special = v.as_i64().unwrap();
                                                }
                                                "reverse" => {
                                                    hl.reverse = v.as_bool().unwrap();
                                                }
                                                "italic" => {
                                                    hl.italic = v.as_bool().unwrap();
                                                }
                                                "bold" => {
                                                    hl.bold = v.as_bool().unwrap();
                                                }
                                                "strikethrough" => {
                                                    hl.strikethrough = v.as_bool().unwrap();
                                                }
                                                "underline" => {
                                                    hl.underline = v.as_bool().unwrap();
                                                }
                                                "undercurl" => {
                                                    hl.undercurl = v.as_bool().unwrap();
                                                }
                                                "blend" => {
                                                    hl.blend = v.as_i64().unwrap();
                                                }
                                                _ => {}
                                            }
                                        }
                                        self.tx
                                            .send(NvimEvent::HighlightAttrDefine { id, hl })
                                            .unwrap();
                                    }
                                }
                                "hl_group_set" => {
                                    // println!("HL GROUP SET:");
                                    // for arg in event {
                                    // println!();
                                    // pretty_print_value(arg, 0);
                                    // }
                                }
                                "option_set" => {
                                    // for arg in event {
                                    // pretty_print_value(arg, 0);
                                    // println!();
                                    // }
                                }
                                "grid_resize" => {
                                    let args = event[1].as_array().unwrap();
                                    self.tx
                                        .send(NvimEvent::GridResize {
                                            grid: args[0].as_i64().unwrap(),
                                            cols: args[1].as_i64().unwrap(),
                                            rows: args[2].as_i64().unwrap(),
                                        })
                                        .unwrap();
                                }
                                _ => {
                                    println!("Unknown redraw: {:?}", event_name);
                                }
                            }
                        }
                    }
                }
            }
            _ => println!("Unknown notify: {} {:?}", name, args),
        }
    }
    fn handle_close(&mut self) {
        self.tx.send(NvimEvent::Close).unwrap();
    }
}

pub fn start(tx: Sender<NvimEvent>, rx: Receiver<ClientEvent>, args: Args) {
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
            match s {
                ClientEvent::Text(s) => {
                    nvim.input(&s).unwrap();
                }
                ClientEvent::Mouse {
                    button,
                    action,
                    modifier,
                    grid,
                    row,
                    col,
                } => {
                    nvim.call_function(
                        "nvim_input_mouse",
                        vec![
                            button.into(),
                            action.into(),
                            modifier.into(),
                            grid.into(),
                            row.into(),
                            col.into(),
                        ],
                    )
                    .unwrap();
                }
                ClientEvent::WindowResize { cols, rows } => {
                    nvim.ui_try_resize(cols, rows).unwrap();
                    // nvim.call_function(
                    //     "nvim_ui_try_resize",
                    //     vec![cols.into(), rows.into()],
                    // ).unwrap();
                }
            }
        }
    }
}
