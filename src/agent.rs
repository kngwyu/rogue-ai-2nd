use consts::*;
use data::*;
use dangeon::*;
use damage::*;
use parse::{MsgParse, StatusParse, Val};
use cgw::{ActionResult, Reactor};
use std::str;
use std::cmp::Ordering;
use std::slice::Iter as SliceIter;
use std::slice::IterMut as SliceIterMut;

#[derive(Clone, Debug)]
struct EnemyList(Vec<EnemyHist>);

// getがダブるのが嫌なのでDerefは実装しない
impl EnemyList {
    fn new() -> EnemyList {
        EnemyList(Vec::with_capacity(8))
    }
    fn add(&mut self, enem: Enemy, cd: Coord) {
        self.0.push(EnemyHist::new(enem, cd));
    }
    fn all_invisible(&mut self) {
        for enem in self.iter_mut() {
            enem.visible = false;
        }
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
    fn init(&mut self) {
        *self = EnemyList::new();
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn iter<'a>(&'a self) -> SliceIter<'a, EnemyHist> {
        self.0.iter()
    }
    fn iter_mut<'a>(&'a mut self) -> SliceIterMut<'a, EnemyHist> {
        self.0.iter_mut()
    }
    fn merge(&mut self, dangeon: &Dangeon) {
        self.all_invisible();
        for (cell_ref, cd) in dangeon.iter() {
            if let Some(enem) = cell_ref.enemy() {
                macro_rules! exec_merge {
                    ($cd:ident, $res:ident, $run:expr) => {
                        if let Some(enem_hist) = self.get_mut($cd) {
                            if enem_hist.typ == enem {
                                enem_hist.cd = cd;
                                enem_hist.visible = true;
                                if $run {
                                    enem_hist.running = true;
                                }
                                $res = true;
                            }
                        }
                    }
                };
                let mut merged = Dist::vars().any(|dist| {
                    let search_cd = cd + dist.as_cd();
                    let mut res = false;
                    exec_merge!(search_cd, res, *dist != Dist::Stay);
                    res
                });
                if !merged && enem.has_attr(EnemyAttr::FLYING) {
                    merged = Dist::vars().any(|dist| {
                        let mut res = false;
                        for &plus_cd in [dist.as_cd(), dist.rotate().as_cd()].iter() {
                            let search_cd = cd + plus_cd;
                            exec_merge!(search_cd, res, true);
                        }
                        res
                    });
                }
                if !merged {
                    self.add(enem, cd);
                }
            }
        }
    }
    fn remove(&mut self, cd: Coord) -> bool {
        let mut rem_id = None;
        for (i, enem) in self.iter().enumerate() {
            if enem.cd == cd {
                rem_id = Some(i);
                break;
            }
        }
        if let Some(i) = rem_id {
            self.0.remove(i);
            true
        } else {
            false
        }
    }
}

struct ItemList(Vec<ItemPack>);

impl ItemList {
    fn new() -> ItemList {
        let mut res = ItemList(vec![ItemPack::default(); 26]);
        res.merge(ItemPack::new(b'a', "", 1, Item::Food(Food::Ration)));
        res.merge(ItemPack::new(b'b', "", 1, Item::Armor(Armor::Ring)));
        res.merge(ItemPack::new(b'c', "", 1, Item::Weapon(Weapon::Mace)));
        res.merge(ItemPack::new(b'd', "", 1, Item::Weapon(Weapon::Bow)));
        // Arrowの数は少なめに見つもっておく(どうせ拾った時にわかるから)
        res.merge(ItemPack::new(b'e', "", 20, Item::Weapon(Weapon::Arrow)));
        res
    }
    fn merge(&mut self, i: ItemPack) -> bool {
        if i.id < b'a' || i.id > b'z' {
            warn!(LOGGER, "Unhandled Item Id: {}", i.id);
            return false;
        }
        let id = (i.id - b'a') as usize;
        if self.0[id].typ == Item::None {
            self.0[id] = i;
            true
        } else if self.0[id].typ == i.typ {
            self.0[id].num = i.num;
            true
        } else {
            false
        }
    }
    // b'a' ~ b'z' でアクセスする(からDerefは実装しない)
    fn get(&self, id: u8) -> Option<&ItemPack> {
        if id < b'a' || id > b'z' {
            warn!(LOGGER, "Unhandled Item Id: {}", id);
            return None;
        }
        let id = (id - b'a') as usize;
        Some(&self.0[id])
    }
    fn get_weapon(&self, id: u8) -> Option<Weapon> {
        let item = self.get(id)?;
        if let Item::Weapon(w) = item.typ {
            Some(w)
        } else {
            None
        }
    }
}

struct ItemCall(i64);

impl Iterator for ItemCall {
    type Item = Vec<u8>;
    fn next(&mut self) -> Option<Vec<u8>> {
        let mut res = Vec::new();
        let mut cur = self.0;
        if cur < 0 {
            return None;
        }
        while cur >= 26 {
            let p = (cur % 26) as u8 + b'a';
            res.push(p);
            cur /= 26;
        }
        res.push(cur as u8 + b'a');
        self.0 += 1;
        Some(res)
    }
}

float_alias!(ActionVal, f64);

impl Eq for ActionVal {}

impl Ord for ActionVal {
    fn cmp(&self, other: &ActionVal) -> Ordering {
        self.partial_cmp(other).expect("NAN value is compared!")
    }
}

impl ActionVal {}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Tactics {
    PickItem,
    Fight,
    Escape,
    Explore,
    ToStair,
    Recover,
    None,
}

default_none!(Tactics);

#[derive(Default, Debug, Clone)]
struct PlayInfo {
    act: Action,
    cd: Coord,
    dest: Option<Coord>,
    priority: ActionVal,
    tact: Tactics,
}

impl PlayInfo {
    fn init_tact(&mut self) {
        self.tact = Tactics::None;
        self.dest = None;
        self.priority = ActionVal::default();
    }
}

#[derive(Default, Debug, Clone, Copy)]
struct MsgFLags {
    defeated: bool,
}

struct Equipment {
    weapon_id: Option<u8>,
    armor_id: Option<u8>,
    rring_id: Option<u8>,
    lring_id: Option<u8>,
}

impl Equipment {
    fn initial() -> Equipment {
        Equipment {
            weapon_id: Some(b'b'),
            armor_id: Some(b'c'),
            rring_id: None,
            lring_id: None,
        }
    }
}

pub struct FeudalAgent {
    stat_parser: StatusParse,
    msg_parser: MsgParse,
    player_stat: PlayerStatus,
    dangeon: Dangeon,
    enemy_list: EnemyList,
    item_list: ItemList,
    play_info: PlayInfo,
    item_call: ItemCall,
    msg_flags: MsgFLags,
    equipment: Equipment,
}

impl FeudalAgent {
    pub fn new() -> Self {
        FeudalAgent {
            stat_parser: StatusParse::new(),
            msg_parser: MsgParse::new(),
            player_stat: PlayerStatus::new(),
            dangeon: Dangeon::default(),
            enemy_list: EnemyList::new(),
            item_list: ItemList::new(),
            play_info: PlayInfo::default(),
            item_call: ItemCall(0),
            msg_flags: MsgFLags::default(),
            equipment: Equipment::initial(),
        }
    }
    fn cur_weapon(&self) -> Option<Weapon> {
        let id = self.equipment.weapon_id?;
        self.item_list.get_weapon(id)
    }
    fn is_dest(&self) -> bool {
        if let Some(cd) = self.play_info.dest {
            self.play_info.cd == cd
        } else {
            false
        }
    }
    fn next_stage(&mut self) {
        self.enemy_list.init();
        self.dangeon.init();
        self.play_info.init_tact();
    }
}

impl Reactor for FeudalAgent {
    fn action(&mut self, action_res: ActionResult, turn: usize) -> Option<Vec<u8>> {
        trace!(LOGGER, "{:?} {}", action_res, turn);
        match action_res {
            ActionResult::Changed(map) => {
                // More で複数ターンぶんの状況を受け取る場合を考慮
                // Mergeはこのブロック内で全部終わらせる
                // 伝播が必要な情報はmsg_flagsに記録する
                let mut ret_early = None;
                let msg = {
                    let msg_str = str::from_utf8(&map[0]).unwrap();
                    let (msg, has_more) = self.msg_parser.parse(msg_str);
                    if has_more {
                        ret_early = Some(Action::Space.into());
                    }
                    msg
                };
                match msg {
                    GameMsg::Item(item_pack) => if item_pack.typ != Item::Gold {
                        self.item_list.merge(item_pack);
                    },
                    GameMsg::Defeated(_) => {
                        self.msg_flags.defeated = true;
                        let _removed = match self.play_info.act {
                            Action::Move(d) | Action::Fight(d) => self.enemy_list.remove(d.as_cd()),
                            Action::Throw((d, _)) => {
                                let mut diter = self.play_info.cd.dist_iter(d);
                                diter.any(|cd| self.enemy_list.remove(cd))
                            }
                            _ => false,
                        };
                    }
                    GameMsg::Scored(_) => match self.play_info.act {
                        Action::Move(d) | Action::Fight(d) => {
                            let dam = {
                                if let Some(w) = self.cur_weapon() {
                                    w.wield().expect_val()
                                } else {
                                    DamageVal::default()
                                }
                            };
                            if let Some(hist_mut) = self.enemy_list.get_mut(d.as_cd()) {
                                hist_mut.hp_ex -= dam;
                            }
                        }
                        Action::Throw((d, id)) => {
                            if let Some(w) = self.item_list.get_weapon(id) {
                                let dam = w.throw().expect_val();
                                self.play_info.cd.dist_iter(d).any(|cd| {
                                    if let Some(hist_mut) = self.enemy_list.get_mut(cd) {
                                        hist_mut.hp_ex -= dam;
                                        true
                                    } else {
                                        false
                                    }
                                });
                            }
                        }
                        _ => {}
                    },
                    GameMsg::CallIt => ret_early = Some(self.item_call.next().unwrap()),
                    _ => {}
                }
                let stat_diff = {
                    let stat_str = str::from_utf8(&map[LINES - 1]).unwrap();
                    match self.stat_parser.parse(stat_str) {
                        Some(s) => self.player_stat.merge(s),
                        None => return None, // これは死んでるからreturnしていい
                    }
                };
                // 必ずmergeする前に呼ぶ
                if stat_diff.stage_level > 0 {
                    self.next_stage();
                }
                let dangeon_msg = self.dangeon.merge(&map[1..(LINES + 1)]);
                if dangeon_msg == DangeonMsg::None {
                    return Some(Action::Die.into());
                }
                if let Some(cd) = self.dangeon.player_cd() {
                    self.play_info.cd = cd;
                }
                self.enemy_list.merge(&self.dangeon);
                if ret_early != None {
                    return ret_early;
                }
                self.action_sub(dangeon_msg)
            }
            ActionResult::NotChanged => self.action_sub(DangeonMsg::None),
            ActionResult::GameEnded => None,
        }
    }
}

// 探索部はこっちに持ってきた(見づらいから)
// 探索用のPlayerState
const SEARCH_DEPTH_MAX: usize = 8;
#[derive(Clone, Debug)]
struct SearchPlayer {
    cd: Coord,
    hp_exp: DamageVal,
}
// マップは持たなくていいよね？
#[derive(Clone, Debug)]
struct SearchState {
    enemy_list: EnemyList,
    player: SearchPlayer,
    actions: Vec<Action>,
    val: ActionVal,
}
impl FeudalAgent {
    fn init_serch_player(&self) -> SearchPlayer {
        SearchPlayer {
            cd: self.play_info.cd,
            hp_exp: DamageVal(self.player_stat.cur_hp as _),
        }
    }
    fn enemy_search(&self) {
        if self.enemy_list.is_empty() {
            return;
        }
        let init_state = SearchState {
            enemy_list: self.enemy_list.clone(),
            player: self.init_serch_player(),
            actions: Vec::with_capacity(SEARCH_DEPTH_MAX),
            val: ActionVal::default(),
        };
    }
    // 食糧・敵への対処など優先度の高い処理
    fn interupput(&self) {}
    fn rethink(&mut self, prev: ActionVal) {
        // prevは現在のTacticsの優先度
    }
    fn action_sub(&mut self, dangeon_msg: DangeonMsg) -> Option<Vec<u8>> {
        match self.play_info.tact {
            Tactics::Escape => {
                // ?
            }
            Tactics::Explore => {
                // 割込み処理 終了判定は？
            }
            Tactics::Fight => {
                // 相互
            }
            Tactics::PickItem => {
                // 座標の確認 割込みまたはReThink
            }
            Tactics::Recover => {
                // HPの確認 割込み処理
            }
            Tactics::ToStair => {
                // 座標の確認 割込み処理
                if self.is_dest() {
                    self.play_info.init_tact();
                    return Some(Action::DownStair.into());
                }
            }
            Tactics::None => {}
        }
        None
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn num_cpus() {
        use num_cpus;
        let num = num_cpus::get();
        println!("{}", num);
    }
}
