use consts::*;
use data::*;

bitflags! {
    pub struct ExplAttr: u8 {
        const VISITED  = 0b000001;
        const GO_UP    = 0b000010;
        const GO_RIGHT = 0b000100;
        const GO_LEFT  = 0b001000;
        const GO_DOWN  = 0b010000;
    }
}
pub struct ExplHist {
    attr: ExplAttr,
    searched: u32,
}
pub struct Cell {
    obj: FieldObject,
    surface: Surface,
}

pub struct Dangeon {
    inner: Vec<Vec<Cell>>,
}

