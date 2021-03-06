use cgw::{ActionResult, Reactor};
use consts::*;
use damage::*;
use dangeon::*;
use data::*;
use num_cpus;
use parse::{MsgParse, StatusParse};
use std::cmp::{self, Ordering};
use std::fmt;
use std::slice::Iter as SliceIter;
use std::slice::IterMut as SliceIterMut;
use std::str;

#[derive(Clone, Debug)]
struct EnemyList(Vec<EnemyHist>);

// getがダブるのが嫌なのでDerefは実装しない
impl EnemyList {
    fn new() -> EnemyList {
        EnemyList(Vec::with_capacity(8))
    }
    fn from_vec(v: Vec<EnemyHist>) -> EnemyList {
        EnemyList(v)
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
    fn get_around_mut(&mut self, cd: Coord, enem_arg: Enemy) -> Option<&mut EnemyHist> {
        for enem in self.iter_mut() {
            if enem.typ != enem_arg {
                continue;
            }
            for &d in Direc::vars().take(8) {
                let ncd = cd + d.to_cd();
                if enem.cd == ncd {
                    return Some(enem);
                }
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
    fn iter(&self) -> SliceIter<EnemyHist> {
        self.0.iter()
    }
    fn iter_mut(&mut self) -> SliceIterMut<EnemyHist> {
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
                let mut merged = Direc::vars().any(|direc| {
                    let search_cd = cd + direc.to_cd();
                    let mut res = false;
                    exec_merge!(search_cd, res, *direc != Direc::Stay);
                    res
                });
                if !merged && enem.has_attr(EnemyAttr::FLYING) {
                    merged = Direc::vars().any(|direc| {
                        let mut res = false;
                        for &plus_cd in &[direc.to_cd(), direc.rotate().to_cd()] {
                            let search_cd = cd + plus_cd;
                            exec_merge!(search_cd, res, true);
                            if res {
                                break;
                            }
                        }
                        res
                    });
                }
                if !merged {
                    if let Some(i) = self.find_invisible(enem, cd) {
                        self.0[i].cd = cd;
                        self.0[i].visible = true;
                        self.0[i].running = true;
                    } else {
                        self.add(enem, cd);
                    }
                }
            }
        }
    }
    fn find_invisible(&self, enem_t: Enemy, cd: Coord) -> Option<usize> {
        let mut aim = (10000, EucDist(7000.0));
        for (i, enem) in self.iter().enumerate() {
            if enem.typ == enem_t && !enem.visible {
                let dist = enem.cd.dist_euc(&cd);
                if dist < aim.1 {
                    aim = (i, dist);
                }
            }
        }
        if aim.0 < 10000 {
            Some(aim.0)
        } else {
            None
        }
    }

    fn remove(&mut self, cd: Coord, target: Enemy) -> bool {
        let mut rem_id = None;
        for (i, enem) in self.iter().enumerate() {
            if enem.cd == cd && enem.typ == target {
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

    fn coord_list(&self) -> Vec<Coord> {
        self.iter().map(|enem| enem.cd).collect()
    }
}

struct ItemList(Vec<ItemPack>);

impl ItemList {
    fn new() -> ItemList {
        let mut res = ItemList(vec![ItemPack::default(); 26]);
        res.merge(ItemPack::new(b'a', "", 1, Item::Food(Food::Ration)));
        let mut arm = ItemPack::new(b'b', "", 1, Item::Armor(Armor::Ring));
        arm.val = Some(4);
        res.merge(arm);
        res.merge(ItemPack::new(b'c', "", 1, Item::Weapon(Weapon::Mace)));
        res.merge(ItemPack::new(b'd', "", 1, Item::Weapon(Weapon::Bow)));
        // Arrowの数は少なめに見つもっておく(どうせ拾った時わかるから)
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
    fn get_mut(&mut self, id: u8) -> Option<&mut ItemPack> {
        if id < b'a' || id > b'z' {
            warn!(LOGGER, "Unhandled Item Id: {}", id);
            return None;
        }
        let id = (id - b'a') as usize;
        Some(&mut self.0[id])
    }
    fn get_weapon(&self, id: u8) -> Option<Weapon> {
        let item = self.get(id)?;
        if let Item::Weapon(w) = item.typ {
            Some(w)
        } else {
            None
        }
    }
    fn any_food(&self) -> Option<u8> {
        for it in self.iter() {
            if let Item::Food(_) = it.typ {
                return Some(it.id);
            }
        }
        None
    }
    fn consume(&mut self, id: u8) {
        let need_to_warn = if let Some(item) = self.get_mut(id) {
            if item.num == 0 {
                true
            } else {
                item.num -= 1;
                false
            }
        } else {
            true
        };
        if need_to_warn {
            warn!(
                LOGGER,
                "attempted to use an item which doesn't exist, {}", id as char
            );
        }
    }
    fn iter(&self) -> SliceIter<ItemPack> {
        self.0.iter()
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
        self.partial_cmp(other)
            .expect("ActionVal: NAN value is compared!")
    }
}

impl ActionVal {
    fn from_gold(i: i32) -> ActionVal {
        ActionVal(f64::from(i))
    }
    fn from_my_dam(hp_exp: DamageVal, d: DamageVal) -> ActionVal {
        let base = d / cmp::max(hp_exp, DamageVal::half());
        -ActionVal(*base * 20.0)
    }
    fn from_enem_dam(hp_exp: DamageVal, d: DamageVal) -> ActionVal {
        let base = d / cmp::max(hp_exp, DamageVal::half());
        ActionVal(*base * 10.0)
    }
    fn from_exp(i: i32) -> ActionVal {
        ActionVal(f64::from(i * 20))
    }
    fn from_hung(hung: i8) -> ActionVal {
        match hung {
            1i8 => ActionVal(100.0),
            2i8 => ActionVal(500.0),
            _ => ActionVal::default(),
        }
    }
    pub fn from_item(i: Item) -> ActionVal {
        ActionVal(match i {
            Item::Potion => 14.0,
            Item::Scroll => 10.0,
            Item::Armor(_) => 20.0,
            Item::Weapon(_) => 20.0,
            Item::Wand => 10.0,
            Item::Food(_) => 20.0,
            Item::Gold => 25.0,
            Item::Ring => 10.0,
            Item::Amulet => 100.0,
            Item::None => 0.0,
        })
    }
    pub fn not_visited(around_none: u8) -> ActionVal {
        ActionVal(f64::from(around_none) + 5.0)
    }
    // searchコマンドに対する評価値
    pub fn search(nr_areas: u8) -> ActionVal {
        ActionVal(match nr_areas {
            1 => 3.0,
            2 => 10.0,
            _ => 1.0,
        })
    }
    // ActionValに対し探索成功確率で補正をかける
    pub fn comp_suc_rate(self, rate: f64) -> ActionVal {
        const PARTIRION_RATE: f64 = 0.3;
        let val = *self * PARTIRION_RATE + *self * (1.0 - PARTIRION_RATE) * rate;
        ActionVal(val)
    }
    // ActionValに対し探索の深さで補正をかける
    pub fn comp_search_depth(self, turn: usize) -> ActionVal {
        const BASE: f64 = enemy_search::SEARCH_DEPTH_MAX as f64;
        let comp = (BASE - turn as f64).log(2.0) / 2.0;
        ActionVal(*self * comp)
    }
    // ActionValueに対し移動距離で補正をかける
    pub fn comp_dist(self, steps: i32) -> ActionVal {
        const BASE: f64 = 100f64;
        let steps = cmp::min(99, steps);
        let div = BASE.log(2.0);
        let min_val = ActionVal(1.0);
        let comp = {
            let tmp = ActionVal((BASE - f64::from(steps)).log(2.0));
            cmp::max(tmp, min_val)
        };
        self * comp / ActionVal(div)
    }
    // TODO: Magic Numberを使わないで書く
    fn stair(exp_rate: f64) -> ActionVal {
        let comp = 1.0 - exp_rate.log(2.0) / (-5.0);
        ActionVal(20.0 * comp)
    }
    fn recover(enough_hp: bool) -> ActionVal {
        ActionVal(if enough_hp { 0.0 } else { 30.0 })
    }
    fn death() -> ActionVal {
        -ActionVal(1000.0)
    }
}

#[macro_export]
macro_rules! comp_action {
    ($($val:expr), *) => ({
        let comp_vec = vec![$($val),*];
        let range = 0..comp_vec.len();
        comp_vec.into_iter().zip(range).max_by_key(|x| x.0).unwrap().1
    })
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Tactics {
    PickItem,
    Fight,
    Explore,
    ToStair,
    Recover,
    Search,
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
    // 自分の座標以外は行動決定時に更新する
    fn update(&self, tact: Tactics, act: Action, dest: Option<Coord>, val: ActionVal) -> PlayInfo {
        let mut res = self.clone();
        res.tact = tact;
        res.act = act;
        res.dest = dest;
        res.priority = val;
        res
    }
    // actionのみupdateする(recoverなど)
    fn update_act(&self, act: Action) -> PlayInfo {
        let mut res = self.clone();
        res.act = act;
        res
    }
    // actionのみupdateして、ついでにinitする(searchなど)
    fn update_act_init(&self, act: Action) -> PlayInfo {
        let mut res = self.clone();
        res.act = act;
        res.tact = Tactics::None;
        res.dest = None;
        res.priority = ActionVal::default();
        res
    }
}

#[derive(Default, Debug, Clone, Copy)]
struct MsgFLags {
    defeated: bool,        // 敵を倒したかどうか
    new_cd: Option<Coord>, // 移動したかどうか
    invalid_item: bool,    // invalid itemを使用したかどうか
    invalid_stair: bool,   // invalid stair
}

impl MsgFLags {
    fn upd_with_msg(&mut self, msg: &GameMsg) {
        match *msg {
            GameMsg::NotValid => self.invalid_item = true,
            GameMsg::NoStair => self.invalid_stair = true,
            GameMsg::Defeated(_) => self.defeated = true,
            _ => {}
        }
    }
    fn reset(&mut self) {
        *self = MsgFLags::default();
    }
    fn set_cd(&mut self, cd: Coord) {
        self.new_cd = Some(cd);
    }
    fn need_to_reset(&self) -> bool {
        self.invalid_item || self.invalid_stair
    }
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
            weapon_id: Some(b'c'),
            armor_id: Some(b'b'),
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
    dead: bool,
}

// !!! STUB !!!
impl fmt::Debug for FeudalAgent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "PlayerStatus: {:?}\n", self.player_stat)?;
        writeln!(f, "PlayInfo: {:?}\n", self.play_info)?;
        Ok(())
    }
}

impl FeudalAgent {
    pub fn new() -> Self {
        FeudalAgent {
            stat_parser: StatusParse::new(),
            msg_parser: MsgParse::new(),
            player_stat: PlayerStatus::initial(),
            dangeon: Dangeon::default(),
            enemy_list: EnemyList::new(),
            item_list: ItemList::new(),
            play_info: PlayInfo::default(),
            item_call: ItemCall(0),
            msg_flags: MsgFLags::default(),
            equipment: Equipment::initial(),
            dead: false,
        }
    }
    fn cur_weapon(&self) -> Option<Weapon> {
        let id = self.equipment.weapon_id?;
        self.item_list.get_weapon(id)
    }
    fn throw_weapon(&self) -> Vec<(Weapon, u32)> {
        let mut res = Vec::new();
        for ip in self.item_list.iter() {
            if let Item::Weapon(w) = ip.typ {
                if w.has_attr(WeaponAttr::MISL) {
                    res.push((w, ip.num));
                }
            }
        }
        res
    }
    fn get_weapon_id(&self, w1: Weapon) -> Option<u8> {
        for ip in self.item_list.iter() {
            if let Item::Weapon(w2) = ip.typ {
                if w1 == w2 {
                    return Some(ip.id);
                }
            }
        }
        None
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
    // 食糧・敵への対処など優先度の高い処理
    // prevは現在のTacticsの優先度
    fn interupput(&self) -> Option<PlayInfo> {
        let hung = self.player_stat.hungry_level;
        let eat_val = ActionVal::from_hung(hung);
        let (fight_val, fight_act) = enemy_search::exec(self).unwrap_or_default();
        let prev = self.play_info.priority;
        let max_act = comp_action!(prev, eat_val, fight_val);
        match max_act {
            0 => None,
            1 => Some(self.play_info.update(
                Tactics::None,
                Action::EatFood(self.item_list.any_food()?),
                None,
                eat_val,
            )),
            _ => Some(
                self.play_info
                    .update(Tactics::Fight, fight_act, None, fight_val),
            ),
        }
    }
    fn move_to_dest_sub(&self, dest: Coord) -> Option<Direc> {
        let dist = self.dangeon.make_dist_map(dest)?;
        let cd = self.play_info.cd;
        let cur_dist = *dist.get(cd)?;
        let mut max_diff = 0;
        let mut ret = None;
        for &d in Direc::vars().take(8) {
            let nxt = cd + d.to_cd();
            if self.dangeon.can_move(cd, d) {
                let dist_diff = cur_dist - *dist.get(nxt)?;
                if dist_diff > max_diff {
                    max_diff = dist_diff;
                    ret = Some(d);
                }
            }
        }
        trace!(LOGGER, "move_to_dest_sub {:?}, {}", ret, max_diff);
        ret
    }
    fn move_to_dest(&self) -> Option<PlayInfo> {
        let d = self.move_to_dest_sub(self.play_info.dest?)?;
        let mut res = self.play_info.clone();
        res.act = Action::Move(d);
        res.cd += d.to_cd();
        Some(res)
    }
    fn rethink(&mut self) -> Option<PlayInfo> {
        // 敵→もう書いた
        // Explore, PickItem, Recover, Tostair が必要
        let (fight_val, fight_act) = enemy_search::exec(self).unwrap_or_default();
        let recover_val = ActionVal::recover(self.player_stat.have_enough_hp());

        let cur_cd = self.play_info.cd;
        let dist = self.dangeon.make_dist_map(cur_cd)?;

        let (item_cd, item_val) = self.dangeon.find_nearest_item(&dist).unwrap_or_default();
        let (explore_cd, explore_val) = self.dangeon.explore(&dist).unwrap_or_default();
        let (stair_cd, stair_val) = if let Some(stair_cd) = self.dangeon.find_stair() {
            let exp_rate = self.dangeon.explore_rate();
            (stair_cd, ActionVal::stair(*exp_rate))
        } else {
            (Coord::default(), ActionVal::default())
        };
        let (search_cd, search_val) = self.dangeon.search(&dist, cur_cd).unwrap_or_default();
        let hung = self.player_stat.hungry_level;
        let eat_val = self.item_list
            .any_food()
            .map_or(ActionVal::default(), |_| ActionVal::from_hung(hung));
        let max_act = comp_action!(
            fight_val,
            recover_val,
            item_val,
            explore_val,
            stair_val,
            search_val,
            eat_val
        );
        trace!(
            LOGGER,
            "rethink
 fight: {:?} \n recv: {:?}
 item: {:?} {:?} \n  explore: {:?} {:?}
 stair: {:?} {:?}\n search: {:?} {:?}\n eat: {:?}",
            fight_val,
            recover_val,
            item_val,
            item_cd,
            explore_val,
            explore_cd,
            stair_val,
            stair_cd,
            search_cd,
            search_val,
            eat_val,
        );
        let ret = match max_act {
            0 => Some(
                self.play_info
                    .update(Tactics::Fight, fight_act, None, fight_val),
            ),
            1 => {
                let dir = self.dangeon.recover(cur_cd).unwrap_or_default();
                Some(
                    self.play_info
                        .update(Tactics::Recover, Action::Move(dir), None, recover_val),
                )
            }
            2 => {
                let dir = self.move_to_dest_sub(item_cd).unwrap_or_default();
                Some(self.play_info.update(
                    Tactics::PickItem,
                    Action::Move(dir),
                    Some(item_cd),
                    item_val,
                ))
            }
            3 => {
                let dir = self.move_to_dest_sub(explore_cd).unwrap_or_default();
                Some(self.play_info.update(
                    Tactics::Explore,
                    Action::Move(dir),
                    Some(explore_cd),
                    explore_val,
                ))
            }
            4 => {
                let dir = self.move_to_dest_sub(stair_cd).unwrap_or_default();
                let act = if dir == Direc::Stay {
                    Action::DownStair
                } else {
                    Action::Move(dir)
                };
                Some(
                    self.play_info
                        .update(Tactics::ToStair, act, Some(stair_cd), stair_val),
                )
            }
            5 => {
                let dir = self.move_to_dest_sub(search_cd).unwrap_or_default();
                let act = if dir == Direc::Stay {
                    Action::Search
                } else {
                    Action::Move(dir)
                };
                Some(
                    self.play_info
                        .update(Tactics::Search, act, Some(search_cd), search_val),
                )
            }
            6 => Some(self.play_info.update(
                Tactics::None,
                Action::EatFood(self.item_list.any_food()?),
                None,
                eat_val,
            )),
            _ => None,
        };
        ret
    }
    // 成功判定と同時に失敗判定をする
    fn action_sub(&mut self) -> Option<Vec<u8>> {
        let mut rethinked = false;
        if self.msg_flags.need_to_reset() {
            self.play_info.init_tact();
        }
        if let Some(cd) = self.msg_flags.new_cd {
            self.set_cur_cd(cd);
        } else if let Action::Move(_) = self.play_info.act {
            self.play_info.init_tact();
        }
        trace!(LOGGER, "action_sub: {:?}", self.play_info);
        let mut nxt_playinfo = match self.play_info.tact {
            Tactics::Explore => {
                // 割込み処理 終了判定
                if self.is_dest() {
                    rethinked = true;
                    self.rethink()
                } else {
                    let inter = self.interupput();
                    if inter.is_some() {
                        inter
                    } else {
                        self.move_to_dest()
                    }
                }
            }
            Tactics::Fight => {
                rethinked = true;
                self.rethink()
            }
            Tactics::PickItem => {
                // 座標の確認 割込みまたはReThink
                if self.is_dest() {
                    rethinked = true;
                    self.rethink()
                } else {
                    let inter = self.interupput();
                    if inter.is_some() {
                        inter
                    } else {
                        self.move_to_dest()
                    }
                }
            }
            // 割込み処理
            Tactics::Search => {
                let inter = self.interupput();
                if inter.is_some() {
                    inter
                } else {
                    if self.is_dest() {
                        Some(self.play_info.update_act_init(Action::Search))
                    } else {
                        self.move_to_dest()
                    }
                }
            }
            // HPの確認 割込み処理
            Tactics::Recover => {
                if self.player_stat.have_enough_hp() {
                    rethinked = true;
                    self.rethink()
                } else {
                    let inter = self.interupput();
                    if inter.is_some() {
                        inter
                    } else {
                        let dir = self.dangeon.recover(self.play_info.cd).unwrap_or_default();
                        Some(self.play_info.update_act(Action::Move(dir)))
                    }
                }
            }
            Tactics::ToStair => {
                // 座標の確認 割込み処理
                if self.is_dest() {
                    self.play_info.init_tact();
                    return Some(Action::DownStair.into());
                }
                let inter = self.interupput();
                if inter.is_some() {
                    inter
                } else {
                    self.move_to_dest()
                }
            }
            Tactics::None => {
                rethinked = true;
                self.rethink()
            }
        };
        if nxt_playinfo.is_none() && !rethinked {
            nxt_playinfo = self.rethink();
        }
        trace!(LOGGER, "action_sub: {:?}", nxt_playinfo);
        self.msg_flags.reset();
        let cur_cd = self.play_info.cd;
        self.play_info = nxt_playinfo?;
        match self.play_info.act {
            Action::Move(d) => self.dangeon.moved(cur_cd, d),
            Action::Throw((_, id)) => self.item_list.consume(id),
            _ => {}
        };
        Some(self.play_info.act.into())
    }

    fn set_cur_cd(&mut self, cd: Coord) {
        self.play_info.cd = cd;
        self.dangeon.visit(cd);
    }
}

impl Reactor for FeudalAgent {
    fn action(&mut self, action_res: ActionResult, turn: usize) -> Option<Vec<u8>> {
        trace!(LOGGER, "{:?} {}", action_res, turn);
        trace!(LOGGER, "{:?}", self);
        if self.dead {
            return Some(Action::Enter.into());
        }
        match action_res {
            ActionResult::Changed(map) => {
                // More で複数ターンぶんの状況を受け取る場合を考慮
                // Mergeはこのブロック内で全部終わらせる
                // !!! 伝播が必要な情報はmsg_flagsに記録する !!!
                let mut ret_early = None;
                let msg = {
                    let msg_str = str::from_utf8(&map[0]).unwrap();
                    let (msg, has_more) = self.msg_parser.parse(msg_str);
                    if has_more {
                        ret_early = Some(Action::Space.into());
                    }
                    msg
                };
                self.msg_flags.upd_with_msg(&msg);
                let cur_cd = self.play_info.cd;
                match msg {
                    GameMsg::Item(item_pack) => if item_pack.typ != Item::Gold {
                        self.item_list.merge(item_pack);
                    },
                    GameMsg::Defeated(enemy_name) => {
                        let removed = match self.play_info.act {
                            Action::Move(d) | Action::Fight(d) => {
                                let base = cur_cd + d.to_cd();
                                if !self.enemy_list.remove(cur_cd + d.to_cd(), enemy_name) {
                                    Direc::vars().take(4).any(|d2| {
                                        let cd = base + d2.to_cd();
                                        self.enemy_list.remove(cd, enemy_name)
                                    })
                                } else {
                                    true
                                }
                            }
                            Action::Throw((d, _)) => {
                                if let Some(mut diter) = cur_cd.direc_iter(d) {
                                    diter.any(|cd| self.enemy_list.remove(cd, enemy_name))
                                } else {
                                    false
                                }
                            }
                            _ => false,
                        };
                        if !removed {
                            warn!(LOGGER, "defeated but not removed enemy: {:?}", enemy_name);
                        }
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
                            if let Some(hist_mut) = self.enemy_list.get_mut(cur_cd + d.to_cd()) {
                                hist_mut.hp_ex -= dam;
                            }
                        }
                        Action::Throw((d, id)) => {
                            if let Some(w) = self.item_list.get_weapon(id) {
                                let dam = w.throw().expect_val();
                                if let Some(mut diter) = cur_cd.direc_iter(d) {
                                    diter.any(|cd| {
                                        if let Some(hist_mut) = self.enemy_list.get_mut(cd) {
                                            hist_mut.hp_ex -= dam;
                                            true
                                        } else {
                                            false
                                        }
                                    });
                                }
                            }
                        }
                        _ => {}
                    },
                    GameMsg::Injured(enem) => {
                        if let Some(enem_hist) = self.enemy_list.get_around_mut(cur_cd, enem) {
                            enem_hist.running = true;
                        }
                    }
                    GameMsg::CallIt => ret_early = Some(self.item_call.next().unwrap()),
                    _ => {}
                }
                let stat_diff = {
                    let stat_str = str::from_utf8(&map[LINES + 1]).unwrap();
                    if let Some(stat) = self.stat_parser.parse(stat_str) {
                        self.player_stat.merge(stat)
                    } else {
                        PlayerStatus::default()
                    }
                };
                // 必ずmergeする前に呼ぶ
                if stat_diff.stage_level > 0 {
                    self.next_stage();
                }
                let dangeon_msg = self.dangeon.merge(&map[1..(LINES + 1)]);
                if dangeon_msg == DangeonMsg::Die {
                    self.dead = true;
                    debug!(LOGGER, "Die turn: {}", turn);
                    return Some(Action::Die.into());
                }
                if let Some(cd) = self.dangeon.player_cd() {
                    self.msg_flags.set_cd(cd);
                }
                self.enemy_list.merge(&self.dangeon);
                trace!(LOGGER, "Enemy List {:?}", self.enemy_list);
                if ret_early != None {
                    debug!(LOGGER, "ret_early: {:?}", ret_early);
                    return ret_early;
                }
                self.action_sub()
            }
            ActionResult::NotChanged => self.action_sub(),
            ActionResult::GameEnded => None,
        }
    }
}

// 探索部はこっちに持ってきた(見づらいから)
mod enemy_search {
    use super::*;
    pub const SEARCH_DEPTH_MAX: usize = 10;
    const SEARCH_WIDTH_MAX: usize = 400;
    // 探索用のPlayerState
    #[derive(Clone, Debug)]
    struct SearchPlayer {
        cd: Coord,
        hp_ex: DamageVal,
        wield: Weapon,
        throw: Vec<(Weapon, u32)>,
    }
    impl SearchPlayer {
        fn initial(agent: &FeudalAgent) -> SearchPlayer {
            SearchPlayer {
                cd: agent.play_info.cd,
                hp_ex: DamageVal(f64::from(agent.player_stat.cur_hp)),
                wield: agent.cur_weapon().unwrap_or_default(),
                throw: agent.throw_weapon(),
            }
        }
        fn is_live(&self) -> bool {
            let threshold = 0.5;
            *self.hp_ex > threshold
        }
    }
    // simulationするアクション
    #[derive(Copy, Clone, Debug)]
    enum TryAction {
        Move(Direc),
        Throw((Direc, Weapon)),
        Stair,
    }
    impl TryAction {
        fn to_action(&self, agent: &FeudalAgent) -> Option<Action> {
            let res = match *self {
                TryAction::Move(d) => Action::Move(d),
                TryAction::Throw((d, w)) => Action::Throw((d, agent.get_weapon_id(w)?)),
                TryAction::Stair => Action::DownStair,
            };
            Some(res)
        }
    }
    #[derive(Clone, Debug)]
    struct SearchState {
        enemy_list: EnemyList,
        player: SearchPlayer,
        actions: Vec<TryAction>,
        end: bool, // Downstairした場合or死んだ場合 これをtrueにする
        val: ActionVal,
    }
    fn simulate_act(
        agent: &FeudalAgent,
        state: &SearchState,
        action: TryAction,
        turn: usize,
    ) -> Option<SearchState> {
        let cur_cd = state.player.cd;
        let mut next_state = state.clone();
        let mut caused_dam = ActionVal::default();
        let (mut gained_gold, mut gained_exp) = (0, 0);
        {
            let cur_hp = next_state.player.hp_ex;
            let mut cause_damage = |enem: &mut EnemyHist, dam: DamageVal| {
                caused_dam += ActionVal::from_my_dam(cur_hp, dam);
                enem.hp_ex -= dam;
                enem.running = true;
                if !enem.is_live() {
                    gained_gold = enem.typ.treasure();
                    gained_exp = enem.typ.exp();
                }
            };
            // 自分の行動
            match action {
                TryAction::Move(d) => {
                    if !agent.dangeon.can_move(cur_cd, d) {
                        return None;
                    }
                    let ncd = cur_cd + d.to_cd();
                    if let Some(enem_ref) = next_state.enemy_list.get_mut(ncd) {
                        let prob = hit_rate_attack(&agent.player_stat, enem_ref);
                        let dam = expect_dam_attack(&agent.player_stat, state.player.wield, false);
                        let dam = dam * DamageVal(*prob);
                        cause_damage(enem_ref, dam);
                    } else {
                        next_state.player.cd = ncd;
                    }
                }
                TryAction::Throw((d, throw_weap)) => {
                    let mut ok = false;
                    for cd in cur_cd.direc_iter(d)? {
                        let cell = agent.dangeon.get(cd)?;
                        match cell.surface() {
                            Surface::Wall | Surface::None | Surface::Door => return None,
                            _ => {}
                        }
                        if let Some(enem_ref) = next_state.enemy_list.get_mut(cd) {
                            let prob = hit_rate_attack(&agent.player_stat, enem_ref);
                            let dam = expect_dam_attack(&agent.player_stat, throw_weap, true);
                            let dam = dam * DamageVal(*prob);
                            cause_damage(enem_ref, dam);
                            ok = true;
                            break;
                        }
                    }
                    // 敵がいないなら投げない
                    if !ok {
                        return None;
                    }
                }
                _ => {}
            }
        }
        // 敵の行動
        next_state.enemy_list = EnemyList::from_vec(
            next_state
                .enemy_list
                .iter()
                .filter(|ene| ene.is_live())
                .cloned()
                .collect(),
        );
        let cur_cd = next_state.player.cd;
        let mut received_dam = ActionVal::default();
        let mut enem_coord = next_state.enemy_list.coord_list();
        'outer: for (i, enem_ref) in next_state.enemy_list.iter_mut().enumerate() {
            macro_rules! is_cd_used {
                ($cd:expr) => {(
                    enem_coord.iter().take(i).any(|&cd| cd == $cd)
                )}
            }
            if !enem_ref.running {
                continue;
            }
            // 殴れるかチェック
            if !is_cd_used!(enem_ref.cd) {
                for d in Direc::vars() {
                    let cd = enem_ref.cd + d.to_cd();
                    if cd == cur_cd {
                        let prob = hit_rate_deffence(&agent.player_stat, &enem_ref.typ);
                        let dam = expect_dam_deffence(enem_ref.typ);
                        let dam = dam * DamageVal(*prob);
                        next_state.player.hp_ex -= dam;
                        received_dam += ActionVal::from_enem_dam(enem_ref.hp_ex, dam);
                        continue 'outer;
                    }
                }
            }
            let (mut best_dist, mut best_cd) = (cur_cd.dist_euc(&enem_ref.cd), Coord::default());
            for &d in Direc::vars() {
                let cd = enem_ref.cd + d.to_cd();
                if !agent.dangeon.can_move_enemy(enem_ref.cd, d) || is_cd_used!(cd) {
                    continue;
                }
                let dist = cur_cd.dist_euc(&cd);
                if dist < best_dist {
                    best_dist = dist;
                    best_cd = cd;
                }
            }
            enem_ref.cd = best_cd;
            enem_coord[i] = best_cd;
        }
        let mut val = ActionVal::from_gold(gained_gold) + ActionVal::from_exp(gained_exp)
            + received_dam + caused_dam;
        if !next_state.player.is_live() {
            val += ActionVal::death();
            next_state.end = true;
        }
        val = val.comp_search_depth(turn);
        next_state.val += val;
        next_state.actions.push(action);
        Some(next_state)
    }
    pub fn select_throw(weapons: &[(Weapon, u32)]) -> Option<Weapon> {
        if weapons.is_empty() {
            return None;
        }
        let mut aim = (Weapon::None, DamageVal::default());
        for w in weapons {
            let dam = w.0.throw().expect_val();
            if w.1 > 0 && dam > aim.1 {
                aim = (w.0, dam);
            }
        }
        Some(aim.0)
    }
    pub fn exec(agent: &FeudalAgent) -> Option<(ActionVal, Action)> {
        if agent.enemy_list.is_empty() {
            return None;
        }
        let init_state = SearchState {
            enemy_list: agent.enemy_list.clone(),
            player: SearchPlayer::initial(agent),
            actions: Vec::with_capacity(SEARCH_DEPTH_MAX),
            end: false,
            val: ActionVal::default(),
        };
        let mut state_list = vec![init_state.clone()];
        let _thread_num = num_cpus::get();
        let mut ma = 0;
        let mut worst = ActionVal::default();
        for turn in 0..SEARCH_DEPTH_MAX {
            state_list.sort_unstable();
            let mut next_states = Vec::new();
            macro_rules! add_state {
                ($ns:ident) => {
                    if $ns.val.is_nan() {
                        warn!(LOGGER, "state value is NAN");
                    } else {
                        next_states.push($ns);
                    }
                }
            }
            ma = cmp::max(ma, state_list.len());
            if let Some(st) = state_list.iter().next() {
                worst = cmp::min(worst, st.val);
            }
            for mut cur_state in state_list.into_iter().rev().take(SEARCH_WIDTH_MAX) {
                if cur_state.end {
                    next_states.push(cur_state);
                    continue;
                }
                // just try to move or throw
                for &d in Direc::vars().take(8) {
                    if let Some(ns) = simulate_act(agent, &cur_state, TryAction::Move(d), turn) {
                        add_state!(ns);
                    }
                }
                if let Some(w) = select_throw(&cur_state.player.throw) {
                    if w == Weapon::None {
                        break;
                    }
                    for &d in Direc::vars().take(8) {
                        if let Some(mut ns) =
                            simulate_act(agent, &cur_state, TryAction::Throw((d, w)), turn)
                        {
                            for wep in &mut ns.player.throw {
                                if wep.0 == w {
                                    wep.1 -= 1;
                                }
                            }
                            add_state!(ns);
                        }
                    }
                }
                if let Some(cell) = agent.dangeon.get(cur_state.player.cd) {
                    if cell.surface() == Surface::Stair {
                        cur_state.actions.push(TryAction::Stair);
                        cur_state.end = true;
                        add_state!(cur_state);
                    }
                }
            }
            state_list = next_states;
        }
        let best_state = state_list.iter().max()?;
        trace!(
            LOGGER,
            "SerachResult best_state: {:?}, worst_score: {:?}",
            best_state,
            worst
        );
        Some((
            best_state.val - worst,
            best_state.actions.get(0)?.to_action(agent)?,
        ))
    }
    impl Ord for SearchState {
        fn cmp(&self, other: &SearchState) -> Ordering {
            self.val.cmp(&other.val)
        }
    }

    impl PartialOrd for SearchState {
        fn partial_cmp(&self, other: &SearchState) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Eq for SearchState {}

    impl PartialEq for SearchState {
        fn eq(&self, other: &SearchState) -> bool {
            self.val.eq(&other.val)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use testutils::*;
    #[test]
    fn test_comp_action() {
        let a = ActionVal(5.0);
        let b = ActionVal(6.0);
        let c = ActionVal(4.0);
        assert_eq!(comp_action!(a, b, c), 1);
    }
    const MAP1: &str = "
             -+--
            |   |
            |   |                 ------+---
            |   |                 |.%.*B..@|
            |   +#################+........|
            -----                 ----------
";
    const MAP2: &str = "
             -+--
            |   |
            |   |                 ------+---
            |   |                 |.%.*.B.@|
            |   +#################+.....)..|
            -----                 ----------
";
    const MAP3: &str = "
             -+--
            |   |
            |   |                 ------+---
            |   |                 |.%.*....|
            |   +#################+....B).@|
            -----                 ----------
";
    const MAP4: &str = "
             -+--
            |   |
            |   |                 ------+---
            |   |                 |.%.*....|
            |   +#################+.....B@.|
            -----                 ----------
";
    #[test]
    fn test_enemy_list() {
        let mut enemy_list = EnemyList::new();
        let maps = vec![MAP1, MAP2, MAP3, MAP4];
        let answers = vec![
            Coord { x: 39, y: 3 },
            Coord { x: 40, y: 3 },
            Coord { x: 39, y: 4 },
            Coord { x: 40, y: 4 },
        ];
        let mut dangeon = make_dangeon(MAP1);
        for (map, &ans) in maps.iter().zip(answers.iter()) {
            dangeon.merge(&str_to_buf(map));
            enemy_list.merge(&dangeon);
            assert_eq!(ans, enemy_list.0[0].cd);
        }
    }
}
