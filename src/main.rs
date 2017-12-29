#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate clap;
extern crate curses_game_wrapper as cgw;
#[macro_use]
extern crate lazy_static;
extern crate regex;
#[macro_use]
extern crate slog;
extern crate sloggers;

mod data;
mod parse;
mod consts;
mod agent;

use consts::*;
use data::*;
use parse::*;
use agent::Agent;
use cgw::{ActionResult, GameSetting, LogType, OpenMode, Reactor};
use std::time::Duration;
use std::error::Error;
fn main() {
    let iter = match MATCHES.value_of("ITER").unwrap_or("1").parse::<usize>() {
        Ok(i) => i,
        Err(why) => panic!("usage: --iter 10, {:?}", why.description()),
    };
    for _ in 0..iter {
        let gs = GameSetting::new("rogue")
            .env("ROGUEUSER", "2ndAI")
            .lines(LINES)
            .columns(COLUMNS)
            .debug_type(LogType::File((
                "debug_cgw.txt".to_owned(),
                *LEVEL,
                OpenMode::Truncate,
            )))
            .max_loop(10)
            .draw_on(Duration::from_millis(150));
        let game = gs.build();
        let mut ai = Agent::new();
        game.play(&mut ai);
    }
}
