use std::ops::{Add, Sub};
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

// innerへのアクセスは
// let d = Dangeon::default();
// let c = d.get(Coord(0, 0));
// let mut c_ref = d.get_mut(Coord(0, 0));

impl Default for Dangeon {
    fn default() -> Dangeon {
        Dangeon {
            inner: vec![vec![Cell::default(); COLUMNS]; LINES],
            empty: true,
        }
    }
}

impl CoordGet for Dangeon {
    type Item = Cell;
    fn get(&self, c: Coord) -> Option<&Cell> {
        if c.range_ok() {
            return None;
        }
        Some(&self.inner[c.y as usize][c.x as usize])
    }
}

impl CoordGetMut for Dangeon {
    type Item = Cell;
    fn get_mut(&mut self, c: Coord) -> Option<&mut Cell> {
        if c.range_ok() {
            return None;
        }
        Some(&mut self.inner[c.y as usize][c.x as usize])
    }
}

pub enum DangeonMsg {
    Die,
    None,
}

impl Dangeon {
    pub fn is_empty(&self) -> bool {
        self.empty
    }
    pub fn fetch(&mut self, orig: &[Vec<u8>]) -> DangeonMsg {
        for (cell_mut, cd) in self.iter_mut() {
            let c = orig[cd.y as usize][cd.x as usize];
            if c == b'/' {
                return DangeonMsg::Die;
            }
            cell_mut.obj = FieldObject::from(c);
            if cell_mut.surface == Surface::None {
                cell_mut.surface = Surface::from(c);
            }
        }
        self.empty = false;
        DangeonMsg::None
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
    fn range_ok(c: &Coord) -> bool {
        c.x >= 0 && c.y >= 0 && c.x < COLUMNS as i32 && c.y < LINES as i32
    }
}

impl<T: Copy + Debug> CoordGet for SimpleMap<T> {
    type Item = T;
    fn get(&self, c: Coord) -> Option<&T> {
        if c.range_ok() {
            return None;
        }
        Some(&self.inner[c.y as usize][c.x as usize])
    }
}

impl<T: Copy + Debug> CoordGetMut for SimpleMap<T> {
    type Item = T;
    fn get_mut(&mut self, c: Coord) -> Option<&mut T> {
        if c.range_ok() {
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
        self.cd.x += 1;
        if self.cd.x >= COLUMNS as _ {
            self.cd.x = 0;
            self.cd.y += 1;
        }
        Some((self.content.get(self.cd)?, self.cd))
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
        self.cd.x += 1;
        if self.cd.x >= COLUMNS as _ {
            self.cd.x = 0;
            self.cd.y += 1;
        }
        let mut cell = self.content.get_mut(self.cd)?;
        // Some(&mut cell)
        unsafe { Some((mem::transmute(&mut cell), self.cd)) }
    }
}

pub trait MutIterator<'a, T> {
    type Item;
    fn next(&'a mut self) -> Option<&'a mut Self::Item>;
}

impl<'a, T> MutIterator<'a, T> for CoordIterMut<'a, T>
where
    T: CoordGetMut + 'a,
{
    type Item = T::Item;
    fn next(&'a mut self) -> Option<&'a mut T::Item> {
        self.cd.x += 1;
        if self.cd.x >= COLUMNS as _ {
            self.cd.x = 0;
            self.cd.y += 1;
        }
        self.content.get_mut(self.cd)
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
    fn range_ok(&self) -> bool {
        self.x >= 0 && self.y >= 0 && self.x < COLUMNS as i32 && self.y < LINES as i32
    }
}

impl Add for Coord {
    type Output = Coord; // Coord * Coord -> Coord
    fn add(self, other: Coord) -> Coord {
        Coord::new(self.x + other.x, self.y + other.y)
    }
}
impl Sub for Coord {
    type Output = Coord;
    fn sub(self, other: Coord) -> Coord {
        Coord::new(self.x - other.x, self.y - other.y)
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

mod test {}
