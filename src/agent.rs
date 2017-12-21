use consts::*;
use data::*;
use parse::{MsgParse, StatusParse};
use cgw::{ActionResult, Reactor};

pub struct Agent {
    stat_parse: StatusParse,
    msg_parse: MsgParse,
}


impl Reactor for Agent {
    fn action(&mut self, screen: ActionResult, turn: usize) -> Option<Vec<u8>> {
        match screen {
            ActionResult::Changed(map) => {}
            ActionResult::NotChanged => {}
            ActionResult::GameEnded => {}
        };
        None
    }
}
