use agent::ActionVal;
use consts::*;
use damage::ProbVal;
use data::*;
use std::cmp::{self, Ordering};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::mem;
use std::ops::{Add, AddAssign, Sub, SubAssign};
bitflags! {
    pub struct ExplAttr: u16 {
        const NONE = 0;
        const VISITED    = 0b000000001;
        const UP         = 0b000000010;
        const DOWN       = 0b000000100;
        const LEFT       = 0b000001000;
        const RIGHT      = 0b000010000;
        const LEFT_UP    = 0b000100000;
        const RIGHT_UP   = 0b001000000;
        const LEFT_DOWN  = 0b010000000;
        const RIGHT_DOWN = 0b100000000;
    }
}

macro_rules! or_empty_vec {
    ($option:expr) => {{
        match $option {
            Some(v) => v,
            None => return Vec::new(),
        }
    }}
}

impl Default for ExplAttr {
    fn default() -> ExplAttr {
        ExplAttr::NONE
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExplHist {
    attr: ExplAttr,
    searched: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Cell {
    obj: FieldObject,
    surface: Surface,
    hist: ExplHist,
}

const FIND_RATE_DOOR: f64 = 0.19;
const FIND_RATE_ROAD: f64 = 0.49;

impl Cell {
    pub fn is_visited(&self) -> bool {
        self.hist.attr.contains(ExplAttr::VISITED)
    }

    pub fn visit(&mut self) {
        self.hist.attr.insert(ExplAttr::VISITED);
    }

    // 隠し通路があった場合に「それが見つかっていない」確率
    pub fn search_suc_rate(&self) -> f64 {
        let searched_i = self.hist.searched as i32;
        let val = if self.surface == Surface::Road {
            (1.0 - FIND_RATE_ROAD).powi(searched_i)
        } else {
            (1.0 - FIND_RATE_DOOR).powi(searched_i)
        };
        val
    }

    pub fn go(&mut self, d: Direc) {
        let ins = match d {
            Direc::Up => ExplAttr::UP,
            Direc::Down => ExplAttr::DOWN,
            Direc::Left => ExplAttr::LEFT,
            Direc::Right => ExplAttr::RIGHT,
            Direc::LeftUp => ExplAttr::LEFT_UP,
            Direc::RightUp => ExplAttr::RIGHT_UP,
            Direc::LeftDown => ExplAttr::LEFT_DOWN,
            Direc::RightDown => ExplAttr::RIGHT_DOWN,
            Direc::Stay => return,
        };
        self.hist.attr.insert(ins);
    }

    pub fn enemy(&self) -> Option<Enemy> {
        if let FieldObject::Enemy(enem) = self.obj {
            Some(enem)
        } else {
            None
        }
    }

    pub fn surface(&self) -> Surface {
        self.surface
    }
    pub fn need_guess(&self) -> bool {
        self.surface.need_guess() && self.obj != FieldObject::None
    }
}

pub trait CoordGet {
    type Item;
    fn get(&self, c: Coord) -> Option<&Self::Item>;
}

pub trait CoordGetMut {
    type Item;
    fn get_mut(&mut self, c: Coord) -> Option<&mut Self::Item>;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DangeonMsg {
    FindNew,
    Die,
    None,
}

default_none!(DangeonMsg);

// ダンジョンの内部表現
#[derive(Debug, Clone)]
pub struct Dangeon {
    inner: Vec<Vec<Cell>>,
    empty: bool,
}

impl Default for Dangeon {
    fn default() -> Dangeon {
        Dangeon {
            inner: vec![vec![Cell::default(); COLUMNS]; LINES],
            empty: true,
        }
    }
}

impl Dangeon {
    pub fn is_empty(&self) -> bool {
        self.empty
    }

    pub fn visit(&mut self, cd: Coord) {
        if let Some(cell) = self.get_mut(cd) {
            cell.visit();
        }
    }

    pub fn merge(&mut self, orig: &[Vec<u8>]) -> DangeonMsg {
        let mut res = DangeonMsg::default();
        let mut new_floor = None;
        for (cell_mut, cd) in self.iter_mut() {
            let c = orig[cd.y as usize][cd.x as usize];
            if c == b'\\' {
                return DangeonMsg::Die;
            }
            cell_mut.obj = FieldObject::from(c);
            if cell_mut.surface == Surface::None {
                let cur_surface = Surface::from(c);
                cell_mut.surface = cur_surface;
                if cur_surface != Surface::None {
                    res = DangeonMsg::FindNew;
                    new_floor = Some(cd);
                }
            }
        }
        self.empty = false;
        if let Some(floor_cd) = new_floor {
            self.extend_floor(floor_cd);
        }
        res
    }

    pub fn init(&mut self) {
        for (cell_mut, _) in self.iter_mut() {
            *cell_mut = Cell::default();
        }
        self.empty = true;
    }

    pub fn iter(&self) -> CoordIter<Dangeon> {
        CoordIter {
            content: self,
            cd: Coord::default(),
        }
    }

    pub fn iter_mut(&mut self) -> CoordIterMut<Dangeon> {
        CoordIterMut {
            content: self,
            cd: Coord::default(),
        }
    }

    pub fn rect_iter(&self, rect: Rect) -> RectIter<Dangeon> {
        RectIter {
            content: self,
            cd: rect.l,
            rect: rect,
        }
    }

    pub fn rect_iter_mut(&mut self, rect: Rect) -> RectIterMut<Dangeon> {
        RectIterMut {
            content: self,
            cd: rect.l,
            rect: rect,
        }
    }

    pub fn player_cd(&self) -> Option<Coord> {
        Some(
            self.iter()
                .find(|&(cell_ref, _)| cell_ref.obj == FieldObject::Player)?
                .1,
        )
    }

    pub fn extend_floor(&mut self, start: Coord) {
        if let Some(rect) = self.extend_floor_sub(start) {
            for (cell, _cd) in self.rect_iter_mut(rect) {
                cell.surface = Surface::Floor;
            }
        }
    }

    pub fn extend_floor_sub(&self, start: Coord) -> Option<Rect> {
        let mut rect = Rect::default();
        for &d in Direc::vars().take(4) {
            let mut bound = None;
            for cd in start.direc_iter(d).unwrap() {
                let cell = self.get(cd)?;
                match cell.surface {
                    Surface::Wall | Surface::Door => {
                        bound = Some(cd);
                        break;
                    }
                    Surface::Road => break,
                    _ => {}
                }
            }
            let b = bound?;
            match d {
                Direc::Up => rect.l.y = b.y + 1,
                Direc::Right => rect.r.x = b.x - 1,
                Direc::Down => rect.r.y = b.y - 1,
                Direc::Left => rect.l.x = b.x + 1,
                _ => {}
            }
        }
        Some(Rect::new(rect.l, rect.r)?)
    }

    // check need_guess before call this fn
    fn guess_floor(&self, cd: Coord) -> Option<Surface> {
        let (mut cnt_f, mut cnt_r) = (0, 0);
        for d in Direc::vars().take(8) {
            if let Some(cell) = self.get(cd + d.to_cd()) {
                match cell.surface {
                    Surface::Road => cnt_r += 1,
                    Surface::Floor => cnt_f += 1,
                    _ => {}
                }
            }
        }
        // 雑すぎかも？
        if cnt_f >= cnt_r {
            Some(Surface::Floor)
        } else {
            Some(Surface::Road)
        }
    }

    fn can_move_sub(cur: Surface, nxt: Surface, d: Direc) -> Option<bool> {
        match cur {
            Surface::Floor => match nxt {
                Surface::Floor | Surface::Stair | Surface::Trap => Some(true),
                Surface::Door => Some(!d.is_diag()),
                Surface::Wall => Some(false),
                _ => None,
            },
            Surface::Road => match nxt {
                Surface::Road | Surface::Stair | Surface::Trap | Surface::Door => {
                    Some(!d.is_diag())
                }
                Surface::Wall => Some(false),
                _ => None,
            },
            Surface::Door => match nxt {
                Surface::Road | Surface::Stair | Surface::Trap | Surface::Door | Surface::Floor => {
                    Some(!d.is_diag())
                }
                Surface::Wall => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn can_move(&self, cd: Coord, d: Direc) -> Option<bool> {
        if d == Direc::Stay {
            return Some(true);
        }
        let cur_cell = self.get(cd)?;
        let nxt_cell = self.get(cd + d.to_cd())?;
        let cur_sur = if cur_cell.need_guess() {
            self.guess_floor(cd)?
        } else {
            cur_cell.surface
        };
        let nxt_sur = if nxt_cell.need_guess() {
            self.guess_floor(cd + d.to_cd())?
        } else {
            nxt_cell.surface
        };
        Dangeon::can_move_sub(cur_sur, nxt_sur, d)
    }

    pub fn make_dist_map(&self, start: Coord) -> Option<SimpleMap<i32>> {
        let mut dist = SimpleMap::new(INF_DIST);
        *dist.get_mut(start)? = 0;
        let mut que = VecDeque::new();
        que.push_back(start);
        while let Some(cd) = que.pop_front() {
            for &d in Direc::vars().take(8) {
                let nxt_cd = cd + d.to_cd();
                let cur_dist = *dist.get(cd)?; // ここは?でいい
                if let Some(nxt_dist_ref) = dist.get_mut(nxt_cd) {
                    let ok = self.can_move(cd, d) == Some(true) && *nxt_dist_ref == INF_DIST;
                    if ok {
                        que.push_back(nxt_cd);
                        *nxt_dist_ref = cur_dist + 1;
                    }
                }
            }
        }
        Some(dist)
    }

    pub fn recover(&self, cd: Coord) -> Option<Direc> {
        for &d in Direc::vars() {
            if self.can_move(cd, d) == Some(true) {
                return Some(d);
            }
        }
        None
    }

    pub fn count_around_none(&self, cd: Coord) -> u8 {
        Direc::vars().fold(0, |acc, d| {
            let nxt_cd = cd + d.to_cd();
            if let Some(cell) = self.get(nxt_cd) {
                if cell.surface == Surface::None {
                    return acc + 1;
                }
            }
            acc
        })
    }

    // explore
    pub fn explore_rate(&self) -> ProbVal {
        let known = self.iter().fold(0f64, |acc, cell_cd| {
            if cell_cd.0.surface != Surface::None {
                acc + 1.0
            } else {
                acc
            }
        });
        let all = LINES * COLUMNS;
        ProbVal(known / all as f64)
    }

    // BFSして葉がdead_endかどうか判断する
    // dead_endと判断されたCoordとそのCellに入るための進行方向を返す
    fn find_dead_end(&self, start: Coord) -> Vec<(Coord, Direc)> {
        let mut used = SimpleMap::new(false);
        *or_empty_vec!(used.get_mut(start)) = true;
        let mut que = VecDeque::new();
        que.push_back(start);
        let mut res = Vec::new();
        'outer: while let Some(cd) = que.pop_front() {
            let mut is_leaf = true;
            for &d in Direc::vars().take(8) {
                let nxt_cd = cd + d.to_cd();
                if let Some(nxt_used_ref) = used.get_mut(nxt_cd) {
                    let ok = self.can_move(cd, d) == Some(true) && !*nxt_used_ref;
                    if ok {
                        que.push_back(nxt_cd);
                        *nxt_used_ref = true;
                        is_leaf = false;
                    }
                }
            }
            if !is_leaf {
                continue;
            }
            let cell = or_empty_vec!(self.get(cd));
            if cell.surface != Surface::Road || !cell.is_visited() {
                continue;
            }
            let mut adj = None;
            for &d in Direc::vars().take(4) {
                let nxt_cd = cd + d.to_cd();
                *or_empty_vec!(used.get_mut(nxt_cd)) = true;
                if let Some(nxt_cell_ref) = self.get(nxt_cd) {
                    if nxt_cell_ref.surface == Surface::Road {
                        if let Some(_) = adj {
                            continue 'outer;
                        } else {
                            adj = Some(d.rotate_n(4));
                        }
                    }
                }
            }
            if let Some(d) = adj {
                res.push((cd, d));
            }
        }
        res
    }

    fn find_not_visited(&self) -> Vec<Coord> {
        self.iter()
            .filter_map(|(cell, cd)| {
                if !cell.is_visited() && cell.surface != Surface::None {
                    Some(cd)
                } else {
                    None
                }
            })
            .collect()
    }

    // 各壁について最も探索回数の少ないマスを適当に集めて返す
    fn find_walls(&self, start: Coord) -> Vec<(Coord, Direc)> {
        let mut res = Vec::new();
        let mut used = SimpleMap::new(false);
        *or_empty_vec!(used.get_mut(start)) = true;
        let mut que = VecDeque::new();
        que.push_back(start);
        while let Some(cd) = que.pop_front() {
            if *or_empty_vec!(used.get(cd)) {
                continue;
            }
            for (i, &d) in Direc::vars().take(8).enumerate() {
                let nxt_cd = cd + d.to_cd();
                let can_move = self.can_move(cd, d);
                if can_move == Some(true) {
                    if let Some(nxt_used_ref) = used.get_mut(nxt_cd) {
                        if !*nxt_used_ref {
                            que.push_back(nxt_cd);
                            *nxt_used_ref = true;
                        }
                    }
                } else if i < 4 {
                    if let Some(nxt_cell) = self.get(nxt_cd) {
                        let cur_cell = or_empty_vec!(self.get(cd));
                        if nxt_cell.surface != Surface::Wall || !cur_cell.surface.can_be_floor() {
                            continue;
                        }
                        let move_dir = if i < 2 {
                            [Direc::Left, Direc::Right]
                        } else {
                            [Direc::Up, Direc::Down]
                        };
                        let mut min_vis = (cd, cur_cell.hist.searched);
                        for &wall_d in &move_dir {
                            for wall_cd in cd.direc_iter(wall_d).expect("Error in find walls") {
                                let wall_cell = or_empty_vec!(self.get(wall_cd));
                                if !wall_cell.surface.can_be_floor() {
                                    break;
                                }
                                if min_vis.1 > wall_cell.hist.searched {
                                    min_vis = (wall_cd, wall_cell.hist.searched);
                                }
                            }
                        }
                        res.push((min_vis.0, d));
                    }
                }
            }
        }
        res
    }
    //  0 | 1 | 2
    //  -   -   -
    //  3 | 4 | 5
    //  -   -   -
    //  6 | 7 | 8
    pub fn explore(&self, player_cd: Coord) -> Option<(Coord, ActionVal)> {
        let dist = self.make_dist_map(player_cd)?;

        // calc not visited
        let not_visited = self.find_not_visited();
        let mut non_visited_val = ActionVal::default();
        let non_visited_cd = not_visited.iter().max_by_key(|&&cd| {
            let dis = *dist.get(cd).unwrap_or(&INF_DIST);
            let val = ActionVal::not_visited(self.count_around_none(cd)).comp_dist(dis);
            non_visited_val = cmp::max(non_visited_val, val);
            val
        });

        let mut has_room = [false; 9];
        self.iter().for_each(|(cell, cd)| {
            let block = cd.block();
            has_room[*block as usize] = cell.surface == Surface::Floor;
        });
        let calc_nonroom_area = |cd: Coord, dir: Direc| -> u8 {
            let mut block = cd.block();
            let mut cnt = 0;
            while let Some(nxt_blk) = block.iterate(dir) {
                if !has_room[*nxt_blk as usize] {
                    cnt += 1;
                }
                block = nxt_blk;
            }
            cnt
        };

        // calc dead end
        let dead_end = self.find_dead_end(player_cd);
        let mut dead_end_val = ActionVal::default();

        let dead_end_cd = dead_end.iter().max_by_key(|cd_and_dir| {
            let nr_areas = calc_nonroom_area(cd_and_dir.0, cd_and_dir.1);
            let dis = *dist.get(cd_and_dir.0).unwrap_or(&INF_DIST);
            let val = ActionVal::explore(nr_areas).comp_dist(dis);
            // 探索回数によるペナルティ
            let pena = if let Some(cell) = self.get(cd_and_dir.0) {
                cell.search_suc_rate()
            } else {
                1.0
            };
            let val = val.comp_suc_rate(pena);
            dead_end_val = cmp::max(dead_end_val, val);
            val
        });

        // calc suspicious wall
        let mut wall_val = ActionVal::default();
        let walls = if non_visited_cd.is_some() {
            Vec::new()
        } else {
            self.find_walls(player_cd)
        };
        let wall_cd = walls.iter().max_by_key(|cd_and_dir| {
            let nr_areas = calc_nonroom_area(cd_and_dir.0, cd_and_dir.1);
            let dis = *dist.get(cd_and_dir.0).unwrap_or(&INF_DIST);
            let val = ActionVal::explore(nr_areas).comp_dist(dis);
            // 探索回数によるペナルティ
            let pena = if let Some(cell) = self.get(cd_and_dir.0) {
                cell.search_suc_rate()
            } else {
                1.0
            };
            let val = val.comp_suc_rate(pena);
            wall_val = cmp::max(wall_val, val);
            val
        });

        let max_act = comp_action!(non_visited_val, dead_end_val, wall_val);
        trace!(
            LOGGER,
            "non visited: {:?}, dead_end: {:?}, wall: {:?}",
            non_visited_val,
            dead_end_val,
            wall_val
        );
        match max_act {
            0 => {
                let cd = non_visited_cd?;
                Some((*cd, non_visited_val))
            }
            1 => {
                let cd = dead_end_cd?.0;
                Some((cd, dead_end_val))
            }
            2 => {
                let cd = wall_cd?.0;
                Some((cd, wall_val))
            }
            _ => None,
        }
    }

    pub fn find_stair(&self) -> Option<Coord> {
        let cd = self.iter()
            .find(|&cell_cd| cell_cd.0.surface == Surface::Stair)?;
        Some(cd.1)
    }

    pub fn find_nearest_item(&self, cd: Coord) -> Option<(Coord, ActionVal)> {
        let dist = self.make_dist_map(cd)?;
        let (cell, cd) = self.iter()
            .filter(|cell_cd| cell_cd.0.obj.is_item())
            .min_by_key(|cell_cd| *dist.get(cell_cd.1).unwrap_or(&0))?;
        let act_val = if let FieldObject::Item(item) = cell.obj {
            ActionVal::from_item(item)
        } else {
            ActionVal::default()
        };
        Some((cd, act_val))
    }
}

// innerへのアクセスは
// let d = Dangeon::default();
// let c = d.get(Coord(0, 0));
// let mut c_ref = d.get_mut(Coord(0, 0));

impl CoordGet for Dangeon {
    type Item = Cell;
    fn get(&self, c: Coord) -> Option<&Cell> {
        if !c.range_ok() {
            return None;
        }
        Some(&self.inner[c.y as usize][c.x as usize])
    }
}

impl CoordGetMut for Dangeon {
    type Item = Cell;
    fn get_mut(&mut self, c: Coord) -> Option<&mut Cell> {
        if !c.range_ok() {
            return None;
        }
        Some(&mut self.inner[c.y as usize][c.x as usize])
    }
}

#[derive(Clone, Debug)]
pub struct SimpleMap<T: Copy + Debug> {
    inner: Vec<Vec<T>>,
}

impl<T: Copy + Debug> SimpleMap<T> {
    pub fn new(init_val: T) -> SimpleMap<T> {
        SimpleMap {
            inner: vec![vec![init_val; COLUMNS]; LINES],
        }
    }
    pub fn iter(&self) -> CoordIter<SimpleMap<T>> {
        CoordIter {
            content: self,
            cd: Coord::default(),
        }
    }
    pub fn iter_mut(&mut self) -> CoordIterMut<SimpleMap<T>> {
        CoordIterMut {
            content: self,
            cd: Coord::default(),
        }
    }
}

impl<T: Copy + Debug> CoordGet for SimpleMap<T> {
    type Item = T;
    fn get(&self, c: Coord) -> Option<&T> {
        if !c.range_ok() {
            return None;
        }
        Some(&self.inner[c.y as usize][c.x as usize])
    }
}

impl<T: Copy + Debug> CoordGetMut for SimpleMap<T> {
    type Item = T;
    fn get_mut(&mut self, c: Coord) -> Option<&mut T> {
        if !c.range_ok() {
            return None;
        }
        Some(&mut self.inner[c.y as usize][c.x as usize])
    }
}

pub struct CoordIter<'a, T>
where
    T: CoordGet + 'a,
{
    content: &'a T,
    cd: Coord,
}

impl<'a, T> Iterator for CoordIter<'a, T>
where
    T: CoordGet + 'a,
{
    type Item = (&'a T::Item, Coord);
    fn next(&mut self) -> Option<(&'a T::Item, Coord)> {
        let before = self.cd;
        self.cd.x += 1;
        if self.cd.x >= COLUMNS as _ {
            self.cd.x = 0;
            self.cd.y += 1;
        }
        Some((self.content.get(before)?, before))
    }
}

pub struct CoordIterMut<'a, T>
where
    T: CoordGetMut + 'a,
{
    content: &'a mut T,
    cd: Coord,
}

impl<'a, T> Iterator for CoordIterMut<'a, T>
where
    T: CoordGetMut + 'a,
{
    type Item = (&'a mut T::Item, Coord);
    fn next(&mut self) -> Option<(&'a mut T::Item, Coord)> {
        let before = self.cd;
        self.cd.x += 1;
        if self.cd.x >= COLUMNS as _ {
            self.cd.x = 0;
            self.cd.y += 1;
        }
        let cell = self.content.get_mut(before)?;
        unsafe { Some((mem::transmute(cell), before)) }
    }
}

int_alias!(BlockVal, i8);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

float_alias!(EucDist, f64);

impl Eq for EucDist {}

impl Ord for EucDist {
    fn cmp(&self, other: &EucDist) -> Ordering {
        self.partial_cmp(other)
            .expect("EucDist: NAN value is compared!")
    }
}
impl Coord {
    pub fn new<T: Into<i32> + Copy>(x: T, y: T) -> Coord {
        Coord {
            x: x.into(),
            y: y.into(),
        }
    }
    pub fn direc_iter(&self, d: Direc) -> Option<DirecIterator> {
        // 無限ループの原因になる
        if d == Direc::Stay {
            None
        } else {
            Some(DirecIterator {
                cur: *self,
                direc: d.to_cd(),
            })
        }
    }
    fn range_ok(&self) -> bool {
        self.x >= 0 && self.y >= 0 && self.x < COLUMNS as _ && self.y < LINES as _
    }
    pub fn dist_euc(&self, other: &Coord) -> EucDist {
        let x = self.x - other.x;
        let y = self.y - other.y;
        EucDist(f64::from(x * x + y * y).sqrt())
    }
    // 未探索区域を知るために9分割する
    //  0 | 1 | 2
    //  -   -   -
    //  3 | 4 | 5
    //  -   -   -
    //  6 | 7 | 8
    pub fn block(&self) -> BlockVal {
        // TODO: remove hard coding
        let row = match self.y {
            val if val < 7 => 0,
            val if val < 14 => 1,
            _ => 2,
        };
        let col = match self.x {
            val if val < 26 => 0,
            val if val < 53 => 1,
            _ => 2,
        };
        BlockVal(row * 3 + col)
    }
}

impl BlockVal {
    fn iterate(&self, dir: Direc) -> Option<Self> {
        match dir {
            Direc::Up => {
                let res = *self + BlockVal(3);
                if *res > 8 {
                    None
                } else {
                    Some(res)
                }
            }
            Direc::Down => {
                let res = *self - BlockVal(3);
                if *res < 0 {
                    None
                } else {
                    Some(res)
                }
            }
            Direc::Right => {
                let res = *self + BlockVal(1);
                if *res % 3 != **self % 3 {
                    None
                } else {
                    Some(res)
                }
            }
            Direc::Left => {
                let res = *self - BlockVal(1);
                if *res % 3 != **self % 3 {
                    None
                } else {
                    Some(res)
                }
            }
            _ => None,
        }
    }
}

impl Add for Coord {
    type Output = Coord; // Coord * Coord -> Coord
    fn add(self, other: Coord) -> Coord {
        Coord::new(self.x + other.x, self.y + other.y)
    }
}

impl AddAssign for Coord {
    fn add_assign(&mut self, other: Coord) {
        *self = *self + other;
    }
}

impl Sub for Coord {
    type Output = Coord;
    fn sub(self, other: Coord) -> Coord {
        Coord::new(self.x - other.x, self.y - other.y)
    }
}

impl SubAssign for Coord {
    fn sub_assign(&mut self, other: Coord) {
        *self = *self - other;
    }
}

impl PartialOrd for Coord {
    fn partial_cmp(&self, other: &Coord) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Coord {
    fn cmp(&self, other: &Coord) -> Ordering {
        let xcmp = self.x.cmp(&other.x);
        match xcmp {
            Ordering::Equal => self.y.cmp(&other.y),
            _ => xcmp,
        }
    }
}

pub struct DirecIterator {
    cur: Coord,
    direc: Coord,
}

impl Iterator for DirecIterator {
    type Item = Coord;
    fn next(&mut self) -> Option<Coord> {
        if !self.cur.range_ok() {
            return None;
        }
        let res = self.cur;
        self.cur += self.direc;
        Some(res)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rect {
    l: Coord,
    r: Coord,
}

impl Rect {
    fn new(l: Coord, r: Coord) -> Option<Rect> {
        let ok = l.x <= r.x && l.y <= r.y;
        if !ok {
            None
        } else {
            Some(Rect { l: l, r: r })
        }
    }
    fn range_ok(&self, cd: Coord) -> bool {
        self.l.x <= cd.x && cd.x <= self.r.y && self.l.y <= cd.y && cd.y <= self.r.y
    }
}

pub struct RectIter<'a, T>
where
    T: CoordGet + 'a,
{
    content: &'a T,
    cd: Coord,
    rect: Rect,
}

impl<'a, T> Iterator for RectIter<'a, T>
where
    T: CoordGet + 'a,
{
    type Item = (&'a T::Item, Coord);
    fn next(&mut self) -> Option<(&'a T::Item, Coord)> {
        let before = self.cd;
        self.cd.x += 1;
        if self.cd.x > self.rect.r.x {
            self.cd.x = self.rect.l.x;
            self.cd.y += 1;
        }
        if self.rect.range_ok(before) {
            Some((self.content.get(before)?, before))
        } else {
            None
        }
    }
}

pub struct RectIterMut<'a, T>
where
    T: CoordGetMut + 'a,
{
    content: &'a mut T,
    cd: Coord,
    rect: Rect,
}

impl<'a, T> Iterator for RectIterMut<'a, T>
where
    T: CoordGetMut + 'a,
{
    type Item = (&'a mut T::Item, Coord);
    fn next(&mut self) -> Option<(&'a mut T::Item, Coord)> {
        let before = self.cd;
        self.cd.x += 1;
        if self.cd.x > self.rect.r.x {
            self.cd.x = self.rect.l.x;
            self.cd.y += 1;
        }
        if self.rect.range_ok(before) {
            let cell = self.content.get_mut(before)?;
            unsafe { Some((mem::transmute(cell), before)) }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // complete map
    const MAP1: &str = "
                            ----------
                            |........|                    ------
 ------------------------- #+........|                    |....|
 |.......................| #|.?......+####################+....|
 |...%...................+##----------                    |....|
 ---------+---------------                                ----+-
          #                                                ####
          #########                                    ----+-----
 -----------------+------- ##########################  |........|
 |......@................+###                       #  |........|
 |.......................|  #                       #  |.....*..|
 |.......................|  #                       #  |........|
 |.......................|  #                       ###+........|
 ----------------+--------  #                          ---+------
                ##          ###########                   #######
   -------------+--        -----------+----                  ---+-
   |..............| #######+..............+############      |...|
   |..............| #      |.!............|           #      |...|
   |..............+##      |..............|           #      |...|
   |..............|        ----------------           #      |...|
   ----------------                                   #######+...|
                                                             -----
";
    #[test]
    fn test_distmap() {
        let d = make_dangeon(&MAP1);
        let cur = d.player_cd().unwrap();
        assert_eq!(cur, Coord::new(8, 9));
        let item = d.find_nearest_item(cur).unwrap();
        assert_approx_eq!(*item.1, 10.0);
        let stair = d.find_stair().unwrap();
        let dist = d.make_dist_map(cur).unwrap();
        assert_eq!(d.can_move(stair, Direc::RightUp), Some(true));
        assert_eq!(d.can_move(stair, Direc::Down), Some(false));
        assert_eq!(28, *dist.get(stair).unwrap());
        println!("{:?}", d.explore_rate());
    }

    #[test]
    fn test_rect_iter() {
        let d = make_dangeon(&MAP1);
        let cur = d.player_cd().unwrap();
        let rect = d.extend_floor_sub(cur).unwrap();
        assert_eq!(
            rect,
            Rect {
                l: Coord { x: 2, y: 9 },
                r: Coord { x: 24, y: 12 },
            }
        );
        for (cell, cd) in d.rect_iter(rect) {
            if cell.surface == Surface::None {
                assert_eq!(cd, cur);
            } else {
                assert_eq!(cell.surface, Surface::Floor);
            }
        }
    }
    use std::io::{BufRead, BufReader};
    use std::str;
    fn make_dangeon(s: &str) -> Dangeon {
        let mut res = Dangeon::default();
        {
            let mut orig = Vec::new();
            let mut buf = String::new();
            let mut reader = BufReader::new(s.as_bytes());
            while let Ok(n) = reader.read_line(&mut buf) {
                if n == 0 || buf.pop() != Some('\n') {
                    break;
                }
                if buf.is_empty() {
                    continue;
                }
                let mut v = buf.as_bytes().to_owned();
                while v.len() < COLUMNS {
                    v.push(b' ');
                }
                orig.push(v);
                buf.clear();
            }
            res.merge(&orig);
        }
        res
    }
}
