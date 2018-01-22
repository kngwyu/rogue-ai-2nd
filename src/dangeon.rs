use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::fmt::Debug;
use std::cmp::{self, Ordering};
use std::collections::VecDeque;
use std::mem;
use consts::*;
use data::*;
use agent::ActionVal;
use damage::ProbVal;
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

    pub fn search_suc_rate(&self) -> ActionVal {
        let searched_i = self.hist.searched as i32;
        let val = if self.surface == Surface::Road {
            (1.0 - FIND_RATE_ROAD).powi(searched_i)
        } else {
            (1.0 - FIND_RATE_DOOR).powi(searched_i)
        };
        ActionVal(val)
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

    pub fn merge(&mut self, orig: &[Vec<u8>]) -> DangeonMsg {
        let mut res = DangeonMsg::default();
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
                }
            }
        }
        self.empty = false;
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

    pub fn player_cd(&self) -> Option<Coord> {
        Some(
            self.iter()
                .find(|&(cell_ref, _)| cell_ref.obj == FieldObject::Player)?
                .1,
        )
    }

    // check need_guess before call this fn
    fn guess_floor(&self, cd: Coord) -> Option<Surface> {
        for d in &[Direc::Up, Direc::Right, Direc::RightUp, Direc::RightDown] {
            let s1 = self.get(cd + d.to_cd())?.surface;
            let s2 = self.get(cd + d.rotate_n(4).to_cd())?.surface;
            if s1 == s2 {
                return Some(s1);
            }
        }
        None
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
        let cur_sur = self.get(cd)?.surface;
        let nxt_sur = self.get(cd + d.to_cd())?.surface;
        if cur_sur.need_guess() {
            Dangeon::can_move_sub(self.guess_floor(cd)?, nxt_sur, d)
        } else {
            Dangeon::can_move_sub(cur_sur, nxt_sur, d)
        }
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
            .filter_map(|(cell, cd)| if cell.is_visited() { Some(cd) } else { None })
            .collect()
    }

    // 各壁について最も探索回数の少ないマスを適当に集めて返す
    fn find_walls(&self, start: Coord) -> Vec<(Coord, u32)> {
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
                            for wall_cd in cd.direc_iter(wall_d) {
                                let wall_cell = or_empty_vec!(self.get(wall_cd));
                                if !wall_cell.surface.can_be_floor() {
                                    break;
                                }
                                if min_vis.1 > wall_cell.hist.searched {
                                    min_vis = (wall_cd, wall_cell.hist.searched);
                                }
                            }
                        }
                        res.push(min_vis);
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
    pub fn explore(&self, player_cd: Coord) -> Option<Coord> {
        let action_base = ActionVal(f64::from(INF_DIST));
        let dist = self.make_dist_map(player_cd)?;

        // calc non visited
        let not_visited = self.find_not_visited();
        let mut non_visited_val = ActionVal::default();
        let non_visited_cd = not_visited.iter().max_by_key(|cd| {
            let dis = *dist.get(**cd).unwrap_or(&INF_DIST);
            let val = action_base / ActionVal(f64::from(dis));
            non_visited_val = cmp::max(non_visited_val, val);
            val
        });

        let mut has_room = [false; 9];
        self.iter().for_each(|(cell, cd)| {
            let block = cd.block();
            has_room[*block as usize] = cell.surface == Surface::Floor;
        });
        let calc_nonroom_area = |cd: Coord, dir: Direc| -> f64 {
            let mut block = cd.block();
            let mut cnt = 0;
            while let Some(nxt_blk) = block.iterate(dir) {
                if !has_room[*nxt_blk as usize] {
                    cnt += 1;
                }
                block = nxt_blk;
            }
            match cnt {
                1 => 0.6,
                2 => 1.0,
                _ => 0.1,
            }
        };

        // calc dead end
        let dead_end = self.find_dead_end(player_cd);
        let mut dead_end_val = ActionVal::default();
        let dead_end_cd = dead_end.iter().max_by_key(|cd_and_dir| {
            let nonroom_point = calc_nonroom_area(cd_and_dir.0, cd_and_dir.1);
            let dis = *dist.get(cd_and_dir.0).unwrap_or(&INF_DIST);
            let val = ActionVal(nonroom_point) * action_base / ActionVal(f64::from(dis));
            // 探索回数によるペナルティ
            let pena = if let Some(cell) = self.get(cd_and_dir.0) {
                cell.search_suc_rate()
            } else {
                ActionVal(1.0)
            };
            let val = val * ActionVal(0.4) + val * ActionVal(0.6) * pena;
            dead_end_val = cmp::max(dead_end_val, val);
            val
        });

        // calc suspicious wall
        let suspicious_wall = {
            if non_visited_cd.is_some() {
                None
            } else {
                let walls = self.find_walls(player_cd);
                Some(walls[0])
            }
        };
        None
    }

    pub fn find_stair(&self) -> Option<Coord> {
        let cd = self.iter()
            .find(|&cell_cd| cell_cd.0.surface == Surface::Stair)?;
        Some(cd.1)
    }

    pub fn find_nearest_item(&self, cd: Coord) -> Option<(ActionVal, Coord)> {
        let dist = self.make_dist_map(cd)?;
        let (cell, cd) = self.iter()
            .filter(|cell_cd| {
                if let FieldObject::Item(_) = cell_cd.0.obj {
                    true
                } else {
                    false
                }
            })
            .min_by_key(|cell_cd| *dist.get(cell_cd.1).unwrap_or(&0))?;
        let act_val = if let FieldObject::Item(item) = cell.obj {
            ActionVal::from_item(item)
        } else {
            ActionVal::default()
        };
        Some((act_val, cd))
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
    pub fn direc_iter(&self, d: Direc) -> DirecIterator {
        DirecIterator {
            cur: *self,
            direc: d.to_cd(),
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
        assert_approx_eq!(*item.0, 10.0);
        let stair = d.find_stair().unwrap();
        let dist = d.make_dist_map(cur).unwrap();
        assert_eq!(d.can_move(stair, Direc::RightUp), Some(true));
        assert_eq!(d.can_move(stair, Direc::Down), Some(false));
        assert_eq!(28, *dist.get(stair).unwrap());
        println!("{:?}", d.explore_rate());
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
