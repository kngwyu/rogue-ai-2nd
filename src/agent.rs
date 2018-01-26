use cgw::{ActionResult, Reactor};
use consts::*;
use damage::*;
use dangeon::*;
use data::*;
use num_cpus;
use parse::{MsgParse, StatusParse};
use std::cmp;
use std::cmp::Ordering;
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
        ActionVal(f64::from(i * 4))
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
            Item::Gold => 30.0,
            Item::Ring => 10.0,
            Item::Amulet => 100.0,
            Item::None => 0.0,
        })
    }
    pub fn not_visited(around_none: u8) -> ActionVal {
        ActionVal(f64::from(around_none) + 5.0)
    }
    // Exploreに対する評価値
    pub fn explore(nr_areas: u8) -> ActionVal {
        let val = match nr_areas {
            1 => 3.0,
            2 => 10.0,
            _ => 1.0,
        };
        ActionVal(val)
    }
    // ActionValに対し探索成功確率で補正をかける
    pub fn comp_suc_rate(self, rate: f64) -> ActionVal {
        const PARTIRION_RATE: f64 = 0.3;
        let val = *self * PARTIRION_RATE + *self * (1.0 - PARTIRION_RATE) * rate;
        ActionVal(val)
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
        ActionVal(100.0 * comp)
    }
    fn recover(enough_hp: bool) -> ActionVal {
        if enough_hp {
            return ActionVal::default();
        }
        ActionVal(100.0)
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
            player_stat: PlayerStatus::initial(),
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
            if self.dangeon.can_move(cd, d) == Some(true) {
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
        let enemy_or_eat = self.interupput().unwrap_or_default();
        let recover_val = ActionVal::recover(self.player_stat.have_enough_hp());

        let (item_cd, item_val) = self.dangeon
            .find_nearest_item(self.play_info.cd)
            .unwrap_or_default();
        let cur_cd = self.play_info.cd;
        let (explore_cd, explore_val) = self.dangeon.explore(cur_cd).unwrap_or_default();
        let (stair_cd, stair_val) = if let Some(stair_cd) = self.dangeon.find_stair() {
            let exp_rate = self.dangeon.explore_rate();
            (stair_cd, ActionVal::stair(*exp_rate))
        } else {
            (Coord::default(), ActionVal::default())
        };

        let max_act = comp_action!(
            enemy_or_eat.priority,
            recover_val,
            item_val,
            explore_val,
            stair_val
        );
        trace!(
            LOGGER,
            "rethink\n enem: {:?} \n recv: {:?}\n item: {:?} {:?}\n explore: {:?} {:?}\n stair: {:?} {:?}",
            enemy_or_eat.priority,
            recover_val,
            item_val,
            item_cd,
            explore_val,
            explore_cd,
            stair_val,
            stair_cd,
        );
        let ret = match max_act {
            0 => Some(enemy_or_eat),
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
            _ => None,
        };
        ret
    }
    // 成功判定と同時に失敗判定をする
    fn action_sub(&mut self) -> Option<Vec<u8>> {
        trace!(LOGGER, "action_sub: {:?}", self.play_info);
        let mut rethinked = false;
        if self.msg_flags.need_to_reset() {
            self.play_info.init_tact();
        }
        if let Some(cd) = self.msg_flags.new_cd {
            self.set_cur_cd(cd);
        } else {
            self.play_info.init_tact();
        }
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
            Tactics::Recover => {
                // HPの確認 割込み処理
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
        self.play_info = nxt_playinfo?;
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
    const SEARCH_DEPTH_MAX: usize = 8;
    const SEARCH_WIDTH_MAX: usize = 100;
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
            let threshold = -0.5;
            *self.hp_ex > threshold
        }
    }
    // simulationするアクション
    #[derive(Copy, Clone, Debug)]
    enum TryAction {
        Move(Direc),
        Throw((Direc, Weapon)),
    }
    impl TryAction {
        fn to_action(&self, agent: &FeudalAgent) -> Option<Action> {
            let res = match *self {
                TryAction::Move(d) => Action::Move(d),
                TryAction::Throw((d, w)) => Action::Throw((d, agent.get_weapon_id(w)?)),
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
                    let can_move = agent.dangeon.can_move(cur_cd, d)?;
                    if !can_move {
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
                            Surface::Wall | Surface::None => return None,
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
        let cur_cd = state.player.cd;
        let mut received_dam = ActionVal::default();
        'outer: for enem_ref in next_state.enemy_list.iter_mut() {
            if !enem_ref.running {
                continue;
            }
            // 殴れるかチェック
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
            let cur_dist = cur_cd.dist_euc(&enem_ref.cd);
            for d in Direc::vars() {
                let cd = enem_ref.cd + d.to_cd();
                let dist = cur_cd.dist_euc(&cd);
                if dist < cur_dist {
                    enem_ref.cd = cd;
                    break;
                }
            }
        }
        let mut val = ActionVal::from_gold(gained_gold) + ActionVal::from_exp(gained_exp)
            + received_dam + caused_dam;
        if !next_state.player.is_live() {
            val += ActionVal::death();
            next_state.end = true;
        }
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
        for _turn in 0..SEARCH_DEPTH_MAX {
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
            for cur_state in state_list.iter().rev().take(SEARCH_WIDTH_MAX) {
                if cur_state.end {
                    next_states.push(cur_state.clone());
                    continue;
                }
                // just try to move or throw
                for &d in Direc::vars().take(8) {
                    if let Some(ns) = simulate_act(agent, cur_state, TryAction::Move(d)) {
                        add_state!(ns);
                    }
                }
                if let Some(w) = select_throw(&cur_state.player.throw) {
                    if w == Weapon::None {
                        break;
                    }
                    for &d in Direc::vars().take(8) {
                        if let Some(mut ns) =
                            simulate_act(agent, cur_state, TryAction::Throw((d, w)))
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
            }
            state_list = next_states;
        }
        let best_state = state_list.iter().max()?;
        trace!(
            LOGGER,
            "best_score: {:?}, worst: {:?}",
            best_state.val,
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
    #[test]
    fn cmp_act() {
        let a = ActionVal(5.0);
        let b = ActionVal(6.0);
        let c = ActionVal(4.0);
        assert_eq!(comp_action!(a, b, c), 1);
    }
}
