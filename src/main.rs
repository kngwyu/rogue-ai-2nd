#[macro_use]
extern crate bitflags;
extern crate regex;
extern crate curses_game_wrapper as cgw;

mod data;

use data::*;
use regex::Regex;
use cgw::{Reactor, ActionResult, GameSetting, LogType, Severity};
use std::time::Duration;
const COLUMNS: usize = 80;
const LINES: usize = 24;
struct StatusParse {
    re: Regex,
}
impl StatusParse {
    fn new() -> Self {
        StatusParse { re: Regex::new(r"(?P<stage>)").unwrap() }
    }
    fn parse(&self, s: &str) {
        let cap = self.re.captures(s).unwrap();
        println!("{:?}", cap);
    }
}
struct Player {
    status: PlayerStatus,
}
struct MyAI {}
impl Reactor for MyAI {
    fn action(&mut self, screen: ActionResult, turn: usize) -> Option<Vec<u8>> {
        match screen {
            ActionResult::Changed(map) => {}
            ActionResult::NotChanged => {}
            ActionResult::GameEnded => {}
        };
        None
    }
}
fn main() {
    let text = "Level: 3  Gold: 237    Hp: 18(25)  Str: 16(16)  Arm: 4   Exp: 3/23";
    let parse = StatusParse::new();
    parse.parse(text);
    let gs = GameSetting::new("rogue")
        .env("ROGUEUSER", "2ndAI")
        .lines(LINES)
        .columns(COLUMNS)
        .debug_type(LogType::File(("debug_cgw.txt".to_owned(), Severity::Debug)))
        .max_loop(1000)
        .draw_on(Duration::from_millis(150));
    let game = gs.build();
    // game.play();
    println!("Hello World")
}
