#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#[macro_use]
extern crate assert_approx_eq;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate clap;
extern crate curses_game_wrapper as cgw;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate num_alias;
extern crate num_cpus;
extern crate rand;
extern crate regex;
#[macro_use]
extern crate slog;
extern crate sloggers;

#[macro_use]
mod data;
mod parse;
mod consts;
#[macro_use]
mod agent;
mod dangeon;
mod damage;

use agent::FeudalAgent as Agent;
use cgw::{GameSetting, Severity};
use consts::*;
use data::*;
use std::time::Duration;

fn main() {
    let max_loop = MATCHES
        .value_of("MAX_LOOP")
        .unwrap_or("100")
        .parse::<usize>()
        .expect("usage: --maxloop 1000");
    let draw_interval = MATCHES
        .value_of("INTERVAL")
        .unwrap_or("100")
        .parse::<u64>()
        .expect("usage: --interval 100");
    let mut gs = GameSetting::new("rogue")
        .env("ROGUEUSER", "2ndAI")
        .lines(LINES + 2)
        .columns(COLUMNS)
        .debug_file("debug_cgw.txt")
        .debug_level(Severity::Debug)
        .max_loop(max_loop);
    if MATCHES.is_present("VIS") {
        gs = gs.draw_on(Duration::from_millis(draw_interval));
    }
    let mut ai = Agent::new();
    let game = gs.build();
    game.play(&mut ai);
}
