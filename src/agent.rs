use consts::*;
use data::*;
use parse::{MsgParse, StatusParse};
use cgw::{ActionResult, Reactor};
use std::str;

pub struct Agent {
    stat_parser: StatusParse,
    msg_parser: MsgParse,
    player_stat: PlayerStatus,
    dangeon: Dangeon,
}

impl Agent {
    pub fn new() -> Self {
        Agent {
            stat_parser: StatusParse::new(),
            msg_parser: MsgParse::new(),
            player_stat: PlayerStatus::new(),
            dangeon: Dangeon::default(),
        }
    }
}

impl Reactor for Agent {
    fn action(&mut self, action_res: ActionResult, turn: usize) -> Option<Vec<u8>> {
        trace!(LOGGER, "{:?}", action_res);
        match action_res {
            ActionResult::Changed(map) => {
                match self.dangeon.fetch(&map) {
                    DangeonMsg::Die => return Some(Action::Die.into()),
                    _ => {}
                }
                let msg = {
                    let msg_str = str::from_utf8(&map[0]).unwrap();
                    let (msg, has_more) = self.msg_parser.parse(msg_str);
                    if has_more {
                        return Some(Action::Space.into());
                    }
                    msg
                };
                let stat_diff = {
                    let stat_str = str::from_utf8(&map[LINES - 1]).unwrap();
                    match self.stat_parser.parse(stat_str) {
                        Some(s) => self.player_stat.fetch(s),
                        None => return None,
                    }
                };
                for dist in Dist::vars() {}
            }
            ActionResult::NotChanged => {}
            ActionResult::GameEnded => {}
        };
        None
    }
}
