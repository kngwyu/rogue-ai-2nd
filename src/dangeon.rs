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
        match cur_sur {
            Surface::Stair | Surface::Trap | Surface::None => {
                Dangeon::can_move_sub(self.guess_floor(cd)?, nxt_sur, d)
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
                let nxt = cd + d.to_cd();
                let ok = self.can_move(cd, d) == Some(true) && *dist.get(nxt)? == INF;
                if ok {
                    que.push_back(nxt);
                    *dist.get_mut(nxt)? = *dist.get(cd)? + 1;
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
            let val = match item {
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
            };
            ActionVal(val)
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

int_alias!(BlockVal, u8);

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
        EucDist(((x * x + y * y) as f64).sqrt())
    }
    // 未探索区域を知るために9分割する
    //  0 | 1 | 2
    //  -   -   -
    //  3 | 4 | 5
    //  -   -   -
    //  6 | 7 | 8
    // pub fn block(&self) -> BlockVal {}
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
