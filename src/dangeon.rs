use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::fmt::Debug;
use std::cmp::Ordering;
use std::mem;
use consts::*;
use data::*;

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
    pub fn go(&mut self, d: Dist) {
        let ins = match d {
            Dist::Up => ExplAttr::UP,
            Dist::Down => ExplAttr::DOWN,
            Dist::Left => ExplAttr::LEFT,
            Dist::Right => ExplAttr::RIGHT,
            Dist::LeftUp => ExplAttr::LEFT_UP,
            Dist::RightUp => ExplAttr::RIGHT_UP,
            Dist::LeftDown => ExplAttr::LEFT_DOWN,
            Dist::RightDown => ExplAttr::RIGHT_DOWN,
            Dist::Stay => return,
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

impl Coord {
    pub fn new<T: Into<i32> + Copy>(x: T, y: T) -> Coord {
        Coord {
            x: x.into(),
            y: y.into(),
        }
    }
    pub fn dist_iter(&self, d: Dist) -> DistIterator {
        DistIterator {
            cur: *self,
            dist: d.as_cd(),
        }
    }

    fn range_ok(&self) -> bool {
        self.x >= 0 && self.y >= 0 && self.x < COLUMNS as _ && self.y < LINES as _
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

pub struct DistIterator {
    cur: Coord,
    dist: Coord,
}

impl Iterator for DistIterator {
    type Item = Coord;
    fn next(&mut self) -> Option<Coord> {
        if !self.cur.range_ok() {
            return None;
        }
        let res = self.cur;
        self.cur += self.dist;
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
        for cd in Coord::new(5, 5).dist_iter(Dist::Right) {
            println!("{:?}", cd);
        }
    }
}
