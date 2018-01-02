use consts::*;
use data::*;
use dangeon::*;
use parse::{MsgParse, StatusParse};
use cgw::{ActionResult, Reactor};
use std::str;
use std::slice::Iter as SliceIter;
use std::slice::IterMut as SliceIterMut;
pub struct EnemyList(Vec<EnemyHist>);

impl EnemyList {
    fn new() -> EnemyList {
        EnemyList(Vec::with_capacity(8))
    }
    fn iter<'a>(&'a self) -> SliceIter<'a, EnemyHist> {
        self.0.iter()
    }
    fn iter_mut<'a>(&'a mut self) -> SliceIterMut<'a, EnemyHist> {
        self.0.iter_mut()
    }
    fn get(&self, cd: Coord) -> Option<&EnemyHist> {
        for enem in self.iter() {
            if enem.cd == cd {
                return Some(enem);
            }
        }
        None
    }
    fn get_mut(&mut self, cd: Coord) -> Option<&mut EnemyHist> {
        for enem in self.iter_mut() {
            if enem.cd == cd {
                return Some(enem);
            }
        }
        None
    }
    fn remove(&mut self, cd: Coord) {
        let mut rem_id = None;
        for (i, enem) in self.iter().enumerate() {
            if enem.cd == cd {
                rem_id = Some(i);
                break;
            }
        }
        if let Some(i) = rem_id {
            self.0.remove(i);
        }
    }
    fn add(&mut self, enem: Enemy, cd: Coord) {
        self.0.push(EnemyHist::new(enem, cd));
    }
    fn all_invisible(&mut self) {
        for enem in self.iter_mut() {
            enem.visible = false;
        }
    }
    fn fetch(&mut self, dangeon: &Dangeon) {
        self.all_invisible();
        for (cell_ref, cd) in dangeon.iter() {
            if let Some(enem) = cell_ref.enemy() {
                let fetched = Dist::vars().any(|dist| {
                    let search_cd = cd + dist.as_cd();
                    let mut res = false;
                    if let Some(enem_hist) = self.get_mut(search_cd) {
                        if enem_hist.name == enem {
                            enem_hist.cd = cd;
                            enem_hist.visible = true;
                            res = true;
                        }
                    }
                    res
                });
                if !fetched && enem.status().has_attr(EnemyAttr::RANDOM) {
                    fetched = Dist::vars().any(|dist| {
                        let mut res = false;
                        for &plus_cd in [dist.as_cd(), dist.rotate().as_cd()].iter() {
                            let search_cd = cd + plus_cd;
                            if let Some(enem_hist) = self.get_mut(search_cd) {
                                if enem_hist.name == enem {
                                    enem_hist.cd = cd;
                                    enem_hist.visible = true;
                                    res = true;
                                }
                            }
                        }
                        res
                    });
                }
                if !fetched {
                    self.add(enem, cd);
                }
            }
        }
    }
}

struct ItemList(Vec<ItemPack>);

impl ItemList {
    fn new() -> ItemList {
        ItemList(Vec::with_capacity(16))
    }
    fn add(&mut self, i: ItemPack) {
        self.0.push(i);
    }
}

pub struct Agent {
    stat_parser: StatusParse,
    msg_parser: MsgParse,
    player_stat: PlayerStatus,
    dangeon: Dangeon,
    enemy_list: EnemyList,
    item_list: ItemList,
    before_act: Action,
}

impl Agent {
    pub fn new() -> Self {
        Agent {
            stat_parser: StatusParse::new(),
            msg_parser: MsgParse::new(),
            player_stat: PlayerStatus::new(),
            dangeon: Dangeon::default(),
            enemy_list: EnemyList::new(),
            item_list: ItemList::new(),
            before_act: Action::None,
        }
    }
}

impl Agent {
    fn action_sub(&mut self) {}
    fn fetch_enemy(enem_list: &mut EnemyList, dangeon: &Dangeon) {}
    fn next_stage(&mut self) {}
}

impl Reactor for Agent {
    fn action(&mut self, action_res: ActionResult, turn: usize) -> Option<Vec<u8>> {
        trace!(LOGGER, "{:?}", action_res);
        match action_res {
            ActionResult::Changed(map) => {
                match self.dangeon.fetch(&map[1..(LINES + 1)]) {
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
                match msg {
                    Msg::Item(itemw) => {
                        let item = itemw.0;
                        let id = itemw.2;
                        let new_item = ItemPack::new(item, id);
                        self.item_list.add(new_item);
                    }
                    Msg::Defeated(enem) => {
                        let delete = |cd: Coord| {};
                        if let Action::Move(cd) = self.before_act {}
                        if let Action::Fight(cd) = self.before_act {}
                    }
                    _ => {}
                }
                let stat_diff = {
                    let stat_str = str::from_utf8(&map[LINES - 1]).unwrap();
                    match self.stat_parser.parse(stat_str) {
                        Some(s) => self.player_stat.fetch(s),
                        None => return None,
                    }
                };
                if stat_diff.stage_level > 0 {
                    self.next_stage();
                }
            }
            ActionResult::NotChanged => {}
            ActionResult::GameEnded => {}
        };
        None
    }
}
