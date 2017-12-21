use consts::*;
use data::*;
use parse::{MsgParse, StatusParse};
use cgw::{ActionResult, Reactor};
use std::str;

pub struct Agent {
    stat_parse: StatusParse,
    msg_parse: MsgParse,
    player_stat: PlayerStatus,
}

impl Agent {
    pub fn new() -> Self {
        Agent { stat_parse: StatusParse::new(),
                msg_parse: MsgParse::new(),
                player_stat: PlayerStatus::default(), }
    }
}

impl Reactor for Agent {
    fn action(&mut self, action_res: ActionResult, turn: usize) -> Option<Vec<u8>> {
        warn!(LOGGER, "{:?}", action_res);
        match action_res {
            ActionResult::Changed(map) => {
                let msg = str::from_utf8(&map[0]).unwrap();
                let msg_parsed = self.msg_parse.parse(msg);
                let stat = str::from_utf8(&map[LINES - 1]).unwrap();
                let stat_parsed = self.stat_parse.parse(stat);
                warn!(LOGGER, "{:?}", msg_parsed);
                warn!(LOGGER, "{:?}", stat_parsed);
            }
            ActionResult::NotChanged => {}
            ActionResult::GameEnded => {}
        };
        None
    }
}
