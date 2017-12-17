// domain knowleadge
// mainly from https://nethackwiki.com/wiki/Rogue_(game)
bitflags! {
    pub struct Attribute: u32 {
        const MEAN       = 0b00001;
        const FLYING     = 0b00010;
        const REGENERATE = 0b00100;
        const GREEDY     = 0b01000;
        const INVISIBLE  = 0b10000;
    }
}

mod fld {
    pub const ROAD: u8 = b'#';
    pub const FLOOR: u8 = b'.';
    pub const WALL_H: u8 = b'-';
    pub const WALL_V: u8 = b'|';
    pub const STAIR: u8 = b'%';
    pub const DOOR: u8 = b'+';
}

mod item {
    pub const PORTION: u8 = b'!';
    pub const SCROLL: u8 = b'?';
    pub const ARM: u8 = b')';
    pub const WAND: u8 = b'/';
    pub const GOLD: u8 = b'*';
    pub const FOOD: u8 = b':';
}

#[derive(Debug, Copy, Clone)]
pub enum Dist {
    Up,
    Down,
    Left,
    Right,
    LeftUp,
    RightUp,
    LeftDown,
    RightDown,
}

// 時計まわり
impl Dist {
    pub fn from_int(d: u8) -> Dist {
        match d {
            0u8 => Dist::Up,
            1u8 => Dist::RightUp,
            2u8 => Dist::Right,
            3u8 => Dist::RightDown,
            4u8 => Dist::Down,
            5u8 => Dist::LeftDown,
            6u8 => Dist::Left,
            _ => Dist::LeftUp,
        }
    }
    pub fn to_byte(&self) -> u8 {
        match *self {
            Dist::Up => 0,
            Dist::RightUp => 1,
            Dist::Right => 2,
            Dist::RightDown => 3,
            Dist::Down => 4,
            Dist::LeftDown => 5,
            Dist::Left => 6,
            Dist::LeftUp => 7,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Action {
    Move(Dist),
    Fight(Dist),
    Throw((Dist, u8)),
    UpStair,
    DownStair,
    Rest,
    QuaffPotion(u8),
    ReadScroll(u8),
    EatFood(u8),
    WieldWeapon(u8),
    WearArmor(u8),
    TakeArmorOff,
    PutOnRing(u8),
    RemoveRing,
    DropObject(u8),
    SaveGame,
    Quit,
}

impl Action {
    pub fn to_byte(&self) -> Vec<u8> {
        match *self {
            Action::Move(d) => vec![d.to_byte()],
            Action::Fight(d) => vec![b'f', d.to_byte()],
            Action::Throw((d, b)) => vec![b't', d.to_byte(), b],
            Action::UpStair => vec![b'<'],
            Action::DownStair => vec![b'>'],
            Action::Rest => vec![b'.'],
            Action::QuaffPotion(b) => vec![b'q', b],
            Action::ReadScroll(b) => vec![b'r', b],
            Action::EatFood(b) => vec![b'e', b],
            Action::WieldWeapon(b) => vec![b'w', b],
            Action::WearArmor(b) => vec![b'W', b],
            Action::TakeArmorOff => vec![b'T'],
            Action::PutOnRing(b) => vec![b'P', b],
            Action::RemoveRing => vec![b'R'],
            Action::DropObject(b) => vec![b'd', b],
            Action::SaveGame => vec![b'S'],
            Action::Quit => vec![b'Q'],
        }
    }
}
