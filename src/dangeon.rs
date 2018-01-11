use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::fmt::Debug;
use std::cmp::Ordering;
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

impl Cell {
    pub fn visit(&mut self) {
        self.hist.attr.insert(ExplAttr::VISITED);
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

pub struct Dangeon {
    inner: Vec<Vec<Cell>>,
    empty: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DangeonMsg {
    FindNew,
    Die,
    None,
}

default_none!(DangeonMsg);

impl Dangeon {
    pub fn is_empty(&self) -> bool {
        self.empty
    }
    pub fn merge(&mut self, orig: &[Vec<u8>]) -> DangeonMsg {
        let mut res = DangeonMsg::default();
        for (cell_mut, cd) in self.iter_mut() {
            let c = orig[cd.y as usize][cd.x as usize];
            if c == b'/' {
                return DangeonMsg::Die;
            }
            cell_mut.obj = FieldObject::from(c);
            if cell_mut.surface == Surface::None {
                cell_mut.surface = Surface::from(c);
                if cell_mut.surface != Surface::None {
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
            content: &self,
            cd: Coord::default(),
        }
    }
    pub fn iter_mut<'a>(&'a mut self) -> CoordIterMut<Dangeon> {
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
    fn can_move_sub(cur: Surface, nxt: Surface, d: Direc) -> Option<bool> {
        match cur {
            Surface::Floor => match nxt {
                Surface::Floor | Surface::Stair | Surface::Trap => Some(true),
                Surface::Door => Some(!d.is_diag()),
                Surface::Wall => Some(false),
                _ => None,
            },
            Surface::Road => match nxt {
                Surface::Road | Surface::Stair | Surface::Trap | Surface::Door => Some(d.is_diag()),
                Surface::Wall => Some(false),
                _ => None,
            },
            Surface::Door => match nxt {
                Surface::Road | Surface::Stair | Surface::Trap | Surface::Door | Surface::Floor => {
                    Some(d.is_diag())
                }
                Surface::Wall => Some(false),
                _ => None,
            },

            _ => None,
        }
    }
    pub fn can_move(&self, cd: Coord, d: Direc) -> Option<bool> {
        let cur_sur = self.get(cd)?.surface;
        let nxt_sur = self.get(cd + d.as_cd())?.surface;
        match cur_sur {
            Surface::Stair | Surface::Trap => {
                let (mut cnt_f, mut cnt_r) = (0, 0);
                for d in Direc::vars().take(8) {
                    let neib = cd + d.as_cd();
                    let nei_s = self.get(neib)?.surface;
                    match nei_s {
                        Surface::Floor => cnt_f += 1,
                        Surface::Road => cnt_r += 1,
                        _ => {}
                    }
                }
                if cnt_f >= cnt_r {
                    Dangeon::can_move_sub(Surface::Floor, nxt_sur, d)
                } else {
                    Dangeon::can_move_sub(Surface::Road, nxt_sur, d)
                }
            }
            _ => Dangeon::can_move_sub(cur_sur, nxt_sur, d),
        }
    }
    pub fn make_dist_map(&self, start: Coord) -> Option<SimpleMap<i32>> {
        const INF: i32 = (COLUMNS * LINES) as i32;
        let mut dist = SimpleMap::new(INF);
        *dist.get_mut(start)? = 0;
        let mut que = VecDeque::new();
        que.push_back(start);
        while let Some(cd) = que.pop_front() {
            for &d in Direc::vars().take(8) {
                let nxt = cd + d.as_cd();
                let ok = self.can_move(cd, d) == Some(true) && *dist.get(cd)? == INF;
                if ok {
                    que.push_back(nxt);
                    let cur_d = *dist.get(cd)?;
                    *dist.get_mut(cd)? = cur_d + 1;
                }
            }
        }
        Some(dist)
    }
    pub fn explore_rate(&self) -> ProbVal {
        let known = self.iter().fold(0, |acc, cell_cd| {
            if cell_cd.0.surface != Surface::None {
                acc + 1
            } else {
                acc
            }
        });
        let all = LINES * COLUMNS;
        ProbVal(known as f64 / all as f64)
    }
    pub fn explore(&self) -> Option<Coord> {
        None
    }
    pub fn find_stair(&self) -> Option<Coord> {
        None
    }
    pub fn find_nearest_item(&self, cd: Coord) -> Option<(ActionVal, Coord)> {
        let (cell, cd) = self.iter()
            .filter(|cell_cd| {
                if let FieldObject::Item(_) = cell_cd.0.obj {
                    true
                } else {
                    false
                }
            })
            .min_by_key(|cell_cd| cd.dist_euc(&cell_cd.1))?;
        let act_val = if let FieldObject::Item(item) = cell.obj {
            let val = match item {
                Item::Amulet => 500.0,
                Item::Gold => 50.0,
                _ => 0.0,
            };
            ActionVal(val)
        } else {
            ActionVal::default()
        };
        None
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

impl Default for Dangeon {
    fn default() -> Dangeon {
        Dangeon {
            inner: vec![vec![Cell::default(); COLUMNS]; LINES],
            empty: true,
        }
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
            content: &self,
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
            direc: d.as_cd(),
        }
    }
    fn range_ok(&self) -> bool {
        self.x >= 0 && self.y >= 0 && self.x < COLUMNS as _ && self.y < LINES as _
    }
    pub fn dist_euc(&self, other: &Coord) -> EucDist {
        let x = self.x - other.x;
        let y = self.y - other.y;
        EucDist(((x * x + y * y) as f64).sqrt())
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
    #[test]
    fn dangeon_test() {
        use dangeon::*;
        let mut d = Dangeon::default();
        println!(">_<");
        for (cell_ref, cd) in d.iter_mut() {
            cell_ref.obj = FieldObject::Player;
        }
        for cd in Coord::new(5, 5).direc_iter(Direc::Right) {
            println!("{:?}", cd);
        }
    }
}
