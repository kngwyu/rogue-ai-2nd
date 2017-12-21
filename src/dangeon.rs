use consts::*;
use std::ops::{Index, IndexMut};
use data::*;

bitflags! {
    pub struct ExplAttr: u8 {
        const NONE = 0;
        const VISITED  = 0b000001;
        const GO_UP    = 0b000010;
        const GO_RIGHT = 0b000100;
        const GO_LEFT  = 0b001000;
        const GO_DOWN  = 0b010000;
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

#[derive(Debug, Clone, Default)]
pub struct Cell {
    obj: FieldObject,
    surface: Surface,
    hist: ExplHist,
}

pub struct Dangeon {
    origin: Vec<Vec<u8>>,
    inner: Vec<Vec<Cell>>,
}

impl Dangeon {
    fn from_buf(buf: Vec<Vec<u8>>) {}
    fn fetch(&self) {}
}

impl Index<usize> for Dangeon {
    type Output = Vec<Cell>;

    fn index(&self, y: usize) -> &Vec<Cell> {
        if y >= LINES {
            &self.inner[LINES - 1]
        } else {
            &self.inner[y]
        }
    }
}

impl IndexMut<usize> for Dangeon {
    fn index_mut<'a>(&'a mut self, y: usize) -> &'a mut Vec<Cell> {
        if y >= LINES {
            &mut self.inner[LINES - 1]
        } else {
            &mut self.inner[y]
        }
    }
}
