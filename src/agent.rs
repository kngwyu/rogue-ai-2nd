use consts::*;
use data::*;
use dangeon::*;
use damage::*;
use parse::{MsgParse, StatusParse};
use cgw::{ActionResult, Reactor};
use num_cpus;
use std::str;
use std::cmp;
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
                let mut merged = Direc::vars().any(|direc| {
                    let search_cd = cd + direc.as_cd();
                    let mut res = false;
                    exec_merge!(search_cd, res, *direc != Direc::Stay);
                    res
                });
                if !merged && enem.has_attr(EnemyAttr::FLYING) {
                    merged = Direc::vars().any(|direc| {
                        let mut res = false;
                        for &plus_cd in [direc.as_cd(), direc.rotate().as_cd()].iter() {
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
    fn iter<'a>(&'a self) -> SliceIter<'a, ItemPack> {
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
            .expect("NAN value is compared!: ActionVal")
    }
}

impl ActionVal {
    fn from_gold(i: i32) -> ActionVal {
        ActionVal(i as f64)
    }
    fn from_my_dam(d: DamageVal) -> ActionVal {
        -ActionVal(*d * 40.0)
    }
    fn from_enem_dam(d: DamageVal) -> ActionVal {
        ActionVal(*d * 10.0)
    }
    fn from_exp(i: i32) -> ActionVal {
        ActionVal((i * 4) as f64)
    }
    fn from_hung(hung: i8) -> ActionVal {
        match hung {
            1i8 => ActionVal(100.0),
            2i8 => ActionVal(500.0),
            _ => ActionVal::default(),
        }
    }
    fn death() -> ActionVal {
        -ActionVal(1000.0)
    }
}

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
    fn update(&self, tact: Tactics, act: Action, dest: Option<Coord>, val: ActionVal) -> PlayInfo {
        let mut res = self.clone();
        res.tact = tact;
        res.act = act;
        res.dest = dest;
        res.priority = val;
        res
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
    fn interupput(&self) -> Option<(PlayInfo)> {
        let hung = self.player_stat.hungry_level;
        let eat = ActionVal::from_hung(hung);
        let (fight, fight_act) = enemy_search::exec(self).unwrap_or_default();
        let prev = self.play_info.priority;
        let max_act = comp_action!(prev, eat, fight);
        match max_act {
            0 => None,
            1 => Some(self.play_info.update(
                Tactics::None,
                Action::EatFood(self.item_list.any_food()?),
                None,
                eat,
            )),
            _ => Some(
                self.play_info
                    .update(Tactics::Fight, fight_act, None, fight),
            ),
        }
    }
    fn move_to_dest_sub(&self) -> Option<Direc> {
        let dist = self.dangeon.make_dist_map(self.play_info.dest?)?;
        let cd = self.play_info.cd;
        let cur_dist = *dist.get(cd)?;
        for &d in Direc::vars().take(8) {
            let nxt = cd + d.as_cd();
            if self.dangeon.can_move(cd, d) == Some(true) && *dist.get(nxt)? < cur_dist {
                return Some(d);
            }
        }
        None
    }
    fn move_to_dest(&self) -> Option<PlayInfo> {
        let d = self.move_to_dest_sub()?;
        let mut res = self.play_info.clone();
        res.act = Action::Move(d);
        res.cd += d.as_cd();
        Some(res)
    }
    fn rethink(&mut self) -> Option<PlayInfo> {
        // 敵→もう書いた
        // Explore, PickItem, Recover, Tostair が必要
        let enemy_or_eat = self.interupput();
        let recov = ActionVal(if self.player_stat.have_enough_hp() {
            0.0
        } else {
            200.0
        });
        let (item, item_cd) = self.dangeon
            .find_nearest_item(self.play_info.cd)
            .unwrap_or_default();
        None
    }
    fn action_sub(&mut self, dangeon_msg: DangeonMsg) -> Option<Vec<u8>> {
        let action = match self.play_info.tact {
            Tactics::Explore => {
                // 割込み処理 終了判定は？
                if self.is_dest() {
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
                // 割込み処理のみ
                let inter = self.interupput();
                inter
            }
            Tactics::PickItem => {
                // 座標の確認 割込みまたはReThink
                if self.is_dest() {
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
            Tactics::None => self.rethink(),
        };
        None
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
                                let mut diter = self.play_info.cd.direc_iter(d);
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
                                self.play_info.cd.direc_iter(d).any(|cd| {
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
mod enemy_search {
    use super::*;
    const SEARCH_DEPTH_MAX: usize = 8;
    const SEARCH_WIDTH_MAX: usize = 100;
    // 探索用のPlayerState
    #[derive(Clone, Debug)]
    struct SearchPlayer {
        cd: Coord,
        hp_exp: DamageVal,
        wield: Weapon,
        throw: Vec<(Weapon, u32)>,
    }
    impl SearchPlayer {
        fn initial(agent: &FeudalAgent) -> SearchPlayer {
            SearchPlayer {
                cd: agent.play_info.cd,
                hp_exp: DamageVal(agent.player_stat.cur_hp as _),
                wield: agent.cur_weapon().unwrap_or_default(),
                throw: agent.throw_weapon(),
            }
        }
        fn is_live(&self) -> bool {
            let threshold = -0.5;
            *self.hp_exp > threshold
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
        let mut caused_dam = DamageVal::default();
        let (mut gained_gold, mut gained_exp) = (0, 0);
        {
            let mut cause_damage = |enem: &mut EnemyHist, dam| {
                caused_dam += dam;
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
                    let ncd = cur_cd + d.as_cd();
                    if let Some(enem_ref) = next_state.enemy_list.get_mut(ncd) {
                        let prob = hit_rate_attack(&agent.player_stat, &enem_ref);
                        let dam = expect_dam_attack(&agent.player_stat, state.player.wield, false);
                        let dam = dam * DamageVal(*prob);
                        cause_damage(enem_ref, dam);
                    } else {
                        next_state.player.cd = ncd;
                    }
                }
                TryAction::Throw((d, throw_weap)) => {
                    let diter = cur_cd.direc_iter(d);
                    let mut ok = false;
                    for cd in diter {
                        let cell = agent.dangeon.get(cd)?;
                        match cell.surface() {
                            Surface::Door | Surface::None => return None,
                            _ => {}
                        }
                        if let Some(enem_ref) = next_state.enemy_list.get_mut(cd) {
                            let prob = hit_rate_attack(&agent.player_stat, &enem_ref);
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
        let mut received_dam = DamageVal::default();
        'outer: for enem_ref in next_state.enemy_list.iter_mut() {
            if !enem_ref.running {
                continue;
            }
            // 殴れるかチェック
            for d in Direc::vars() {
                let cd = enem_ref.cd + d.as_cd();
                if cd == cur_cd {
                    let prob = hit_rate_deffence(&agent.player_stat, &enem_ref.typ);
                    let dam = expect_dam_deffence(enem_ref.typ);
                    let dam = dam * DamageVal(*prob);
                    next_state.player.hp_exp -= dam;
                    received_dam += dam;
                    break 'outer;
                }
            }
            let cur_dist = cur_cd.dist_euc(&enem_ref.cd);
            for d in Direc::vars() {
                let cd = enem_ref.cd + d.as_cd();
                let dist = cur_cd.dist_euc(&cd);
                if dist < cur_dist {
                    enem_ref.cd = cd;
                    break;
                }
            }
        }
        let mut val = ActionVal::from_gold(gained_gold) + ActionVal::from_exp(gained_exp)
            + ActionVal::from_my_dam(received_dam)
            + ActionVal::from_enem_dam(caused_dam);
        if !next_state.player.is_live() {
            val += ActionVal::death();
            next_state.end = true;
        }
        next_state.val += val;
        next_state.actions.push(action);
        Some(next_state)
    }
    pub fn select_throw(weapons: &Vec<(Weapon, u32)>) -> Option<Weapon> {
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
        for _ in 0..SEARCH_DEPTH_MAX {
            state_list.sort_unstable();
            let mut next_states = Vec::new();
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
                for &d in Direc::vars() {
                    if let Some(ns) = simulate_act(agent, cur_state, TryAction::Move(d)) {
                        next_states.push(ns);
                    }
                }
                if let Some(w) = select_throw(&cur_state.player.throw) {
                    if w == Weapon::None {
                        break;
                    }
                    for &d in Direc::vars() {
                        if let Some(mut ns) =
                            simulate_act(agent, cur_state, TryAction::Throw((d, w)))
                        {
                            for wep in ns.player.throw.iter_mut() {
                                if wep.0 == w {
                                    wep.1 -= 1;
                                }
                            }
                            next_states.push(ns);
                        }
                    }
                }
            }
            state_list = next_states;
        }
        debug!(LOGGER, "max state num: {}", ma);
        let best_state = state_list.iter().max()?;
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
    fn num_cpus() {
        use num_cpus;
        let num = num_cpus::get();
        println!("{}", num);
    }
    #[test]
    fn cmp_act() {
        let a = ActionVal(5.0);
        let b = ActionVal(6.0);
        let c = ActionVal(4.0);
        assert_eq!(comp_action!(a, b, c), 1);
    }
}
