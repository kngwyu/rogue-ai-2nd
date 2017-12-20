#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#[macro_use]
extern crate bitflags;
extern crate regex;
extern crate curses_game_wrapper as cgw;

mod data;
mod parse;
use data::*;
use parse::*;
use cgw::{Reactor, ActionResult, GameSetting, LogType, Severity};
use std::time::Duration;
const COLUMNS: usize = 80;
const LINES: usize = 24;
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
    let gs = GameSetting::new("rogue")
        .env("ROGUEUSER", "2ndAI")
        .lines(LINES)
        .columns(COLUMNS)
        .debug_type(LogType::File(("debug_cgw.txt".to_owned(), Severity::Debug)))
        .max_loop(1000)
        .draw_on(Duration::from_millis(150));
    let game = gs.build();
    // game.play();
}
