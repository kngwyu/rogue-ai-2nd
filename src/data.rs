// domain knowleadge
// enemy data is from https://nethackwiki.com/wiki/Rogue_(game), thanks

use std::ops::{Index, IndexMut, Sub};
use consts::*;

macro_rules! default_none {
    ($enum:ident) => {
        impl Default for $enum {
            fn default()-> $enum {
                $enum::None
            }
        }
    };
}

#[derive(Copy, Clone, Debug)]
pub struct Dice(i32, i32);

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
impl From<u8> for Dist {
    fn from(d: u8) -> Self {
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
}

impl Into<u8> for Dist {
    fn into(self) -> u8 {
        match self {
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

#[allow(dead_code)]
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

impl Into<Vec<u8>> for Action {
    fn into(self) -> Vec<u8> {
        match self {
            Action::Move(d) => vec![d.into()],
            Action::Fight(d) => vec![b'f', d.into()],
            Action::Throw((d, b)) => vec![b't', d.into(), b],
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

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerStatus {
    pub stage_level: i8,
    pub gold: i32,
    pub cur_hp: i32,
    pub max_hp: i32,
    pub cur_str: i32,
    pub max_str: i32,
    pub arm: i8,
    pub exp_level: i32,
    pub exp: i32,
    pub hungry_level: i8,
}

impl Sub for PlayerStatus {
    type Output = PlayerStatus;
    fn sub(self, other: PlayerStatus) -> PlayerStatus {
        let mut res = PlayerStatus::default();
        macro_rules! diff {
            ($name:ident) => {
                res.$name = other.$name - self.$name;
            }
        }
        diff!(stage_level);
        diff!(gold);
        diff!(cur_hp);
        diff!(max_hp);
        diff!(cur_str);
        diff!(max_str);
        diff!(arm);
        diff!(exp_level);
        diff!(exp);
        diff!(hungry_level);
        res
    }
}

impl PlayerStatus {
    pub fn new() -> PlayerStatus {
        PlayerStatus {
            stage_level: 1,
            gold: 0,
            cur_hp: 12,
            max_hp: 12,
            cur_str: 16,
            max_str: 16,
            arm: 4,
            exp_level: 1,
            exp: 0,
            hungry_level: 0,
        }
    }
    pub fn fetch(&mut self, new_stat: PlayerStatus) -> PlayerStatus {
        let res = new_stat - *self;
        *self = new_stat;
        res
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    NotInjured(Enemy),
    Injured(Enemy),
    Direction,
    Scored(Enemy),
    Defeated(Enemy),
    Missed(Enemy),
    Item(ItemWithId),
    ArmorW,
    ArmorT,
    WhichObj,
    LevelUp(u8),
    Ate,
    PackFull,
    MovedOnto(Item),
    Dropped,
    CallIt,
    None,
}

default_none!(Msg);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enemy {
    Aquator,
    Bat,
    Centaur,
    Dragon,
    Emu,
    Flytrap,
    Griffin,
    Hobgoblin,
    IceMonster,
    Jabberwock,
    Kestrel,
    Leprechaun,
    Medusa,
    Nymph,
    Orc,
    Phantom,
    Quagga,
    Rattlesnake,
    Snake,
    Troll,
    UrVile,
    Vampire,
    Wraith,
    Xeroc,
    Yeti,
    Zombie,
    None,
}

default_none!(Enemy);

impl From<u8> for Enemy {
    fn from(u: u8) -> Self {
        match u {
            b'a' | b'A' => Enemy::Aquator,
            b'b' | b'B' => Enemy::Bat,
            b'c' | b'C' => Enemy::Centaur,
            b'd' | b'D' => Enemy::Dragon,
            b'e' | b'E' => Enemy::Emu,
            b'f' | b'F' => Enemy::Flytrap,
            b'g' | b'G' => Enemy::Griffin,
            b'h' | b'H' => Enemy::Hobgoblin,
            b'i' | b'I' => Enemy::IceMonster,
            b'j' | b'J' => Enemy::Jabberwock,
            b'k' | b'K' => Enemy::Kestrel,
            b'l' | b'L' => Enemy::Leprechaun,
            b'm' | b'M' => Enemy::Medusa,
            b'n' | b'N' => Enemy::Nymph,
            b'o' | b'O' => Enemy::Orc,
            b'p' | b'P' => Enemy::Phantom,
            b'q' | b'Q' => Enemy::Quagga,
            b'r' | b'R' => Enemy::Rattlesnake,
            b's' | b'S' => Enemy::Snake,
            b't' | b'T' => Enemy::Troll,
            b'u' | b'U' => Enemy::UrVile,
            b'v' | b'V' => Enemy::Vampire,
            b'w' | b'W' => Enemy::Wraith,
            b'x' | b'X' => Enemy::Xeroc,
            b'y' | b'Y' => Enemy::Yeti,
            b'z' | b'Z' => Enemy::Zombie,
            _ => Enemy::None,
        }
    }
}

impl Into<u8> for Enemy {
    fn into(self) -> u8 {
        match self {
            Enemy::Aquator => b'A',
            Enemy::Bat => b'B',
            Enemy::Centaur => b'C',
            Enemy::Dragon => b'D',
            Enemy::Emu => b'E',
            Enemy::Flytrap => b'F',
            Enemy::Griffin => b'G',
            Enemy::Hobgoblin => b'H',
            Enemy::IceMonster => b'I',
            Enemy::Jabberwock => b'J',
            Enemy::Kestrel => b'K',
            Enemy::Leprechaun => b'L',
            Enemy::Medusa => b'M',
            Enemy::Nymph => b'N',
            Enemy::Orc => b'O',
            Enemy::Phantom => b'P',
            Enemy::Quagga => b'Q',
            Enemy::Rattlesnake => b'R',
            Enemy::Snake => b'S',
            Enemy::Troll => b'T',
            Enemy::UrVile => b'U',
            Enemy::Vampire => b'V',
            Enemy::Wraith => b'W',
            Enemy::Xeroc => b'X',
            Enemy::Yeti => b'Y',
            Enemy::Zombie => b'Z',
            Enemy::None => b' ',
        }
    }
}

impl Enemy {
    fn status(self) -> &'static EnemyStatus {
        let id: u8 = self.into();
        &ENEMIES[(id - b'A') as usize]
    }
}

#[derive(Debug)]
pub struct EnemyStatus {
    pub treasure: i32,
    pub attr: EnemyAttr,
    pub exp: i32,
    pub hp: Dice,
    pub defence: i32,
    pub attack: Vec<Dice>,
}

bitflags! {
    pub struct EnemyAttr: u16 {
        const MEAN        = 0b0000000001;
        const FLYING      = 0b0000000010;
        const REGENERATE  = 0b0000000100;
        const GREEDY      = 0b0000001000;
        const INVISIBLE   = 0b0000010000;
        const RUSTS_ARMOR = 0b0000100000;
        const STEAL_GOLD  = 0b0001000000;
        const REDUCE_STR  = 0b0010000000;
        const FREEZES     = 0b0100000000;
        const RANDOM      = 0b1000000000;
        const NONE        = 0;
    }
}

impl From<Vec<EnemyAttr>> for EnemyAttr {
    fn from(ar: Vec<EnemyAttr>) -> Self {
        let mut res = EnemyAttr::NONE;
        for x in ar {
            res.insert(x);
        }
        res
    }
}

lazy_static!{
    static ref ENEMIES: [EnemyStatus; 26] =[
        EnemyStatus { // Aquator
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN, EnemyAttr::RUSTS_ARMOR]),
            exp: 20,
            hp: Dice(5, 8),
            defence: 2,
            attack: vec![Dice(0, 0)],
        },
        EnemyStatus { // Bat
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::FLYING, EnemyAttr::RANDOM]),
            exp: 1,
            hp: Dice(1, 8),
            defence: 3,
            attack: vec![Dice(1, 2)],
        },
        EnemyStatus { // Centaur
            treasure: 15,
            attr: EnemyAttr::from(vec![]),
            exp: 17,
            hp: Dice(4, 8),
            defence: 4,
            attack: vec![Dice(1, 2), Dice(1, 5), Dice(1, 5)],
        },
        EnemyStatus { // Dragon
            treasure: 100,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN]),
            exp: 5000,
            hp: Dice(10, 8),
            defence: 3,
            attack: vec![Dice(1, 8), Dice(1, 8), Dice(3, 10)],
        },
        EnemyStatus { // Emu
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN]),
            exp: 2,
            hp: Dice(1, 8),
            defence: 7,
            attack: vec![Dice(1, 2)],
        },
        EnemyStatus { // Venus Flytrap
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN]),
            exp: 80,
            hp: Dice(8, 8),
            defence: 3,
            attack: vec![], // special
        },
        EnemyStatus { // Griffin
            treasure: 20,
            attr: EnemyAttr::from(vec![EnemyAttr::FLYING, EnemyAttr::MEAN, EnemyAttr::REGENERATE]),
            exp: 2000,
            hp: Dice(13, 8),
            defence: 2,
            attack: vec![Dice(4, 3), Dice(3, 5)],
        },
        EnemyStatus { // Hobgoblin
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN]),
            exp: 3,
            hp: Dice(1, 8),
            defence: 5,
            attack: vec![Dice(1, 8)],
        },
        EnemyStatus { // Icemonster
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::FREEZES]),
            exp: 5,
            hp: Dice(1, 8),
            defence: 9,
            attack: vec![Dice(0, 0)],
        },
        EnemyStatus { // Jabberwock
            treasure: 70,
            attr: EnemyAttr::from(vec![]),
            exp: 3000,
            hp: Dice(15, 8),
            defence: 6,
            attack: vec![Dice(2, 12), Dice(2, 4)],
        },
        EnemyStatus { // Kestrel
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::FLYING, EnemyAttr::MEAN]),
            exp: 1,
            hp: Dice(1, 8),
            defence: 7,
            attack: vec![Dice(1, 4)],
        },
        EnemyStatus { // Leperachaun
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::STEAL_GOLD]),
            exp: 10,
            hp: Dice(3, 8),
            defence: 8,
            attack: vec![Dice(1, 1)],
        },
        EnemyStatus { // Medusa
            treasure: 40,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN]),
            exp: 200,
            hp: Dice(8, 8),
            defence: 2,
            attack: vec![Dice(3, 4), Dice(3, 4), Dice(2, 5)],
        },
        EnemyStatus { // Nymph
            treasure: 100,
            attr: EnemyAttr::from(vec![]),
            exp: 200,
            hp: Dice(3, 8),
            defence: 9,
            attack: vec![Dice(0, 0)],
        },
        EnemyStatus { // Orc
            treasure: 15,
            attr: EnemyAttr::from(vec![EnemyAttr::GREEDY]),
            exp: 5,
            hp: Dice(1, 8),
            defence: 6,
            attack: vec![Dice(1, 8)],
        },
        EnemyStatus { // Phantom
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::INVISIBLE]),
            exp: 120,
            hp: Dice(8, 8),
            defence: 3,
            attack: vec![Dice(4, 4)],
        },
        EnemyStatus { // Quagga
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN]),
            exp: 15,
            hp: Dice(3, 8),
            defence: 3,
            attack: vec![Dice(1, 5), Dice(1, 5)],
        },
        EnemyStatus { // Rattlesnake
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN, EnemyAttr::REDUCE_STR]),
            exp: 9,
            hp: Dice(2, 8),
            defence: 3,
            attack: vec![Dice(1, 6)],
        },
        EnemyStatus { // Snake
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN]),
            exp: 2,
            hp: Dice(1, 8),
            defence: 5,
            attack: vec![Dice(1, 3)],
        },
        EnemyStatus { // Troll
            treasure: 50,
            attr: EnemyAttr::from(vec![EnemyAttr::REGENERATE, EnemyAttr::MEAN]),
            exp: 120,
            hp: Dice(6, 8),
            defence: 4,
            attack: vec![Dice(1, 8), Dice(1, 8), Dice(2, 6)],
        },
        EnemyStatus { // Urvile
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN]),
            exp: 190,
            hp: Dice(7, 8),
            defence: -2,
            attack: vec![Dice(1, 9), Dice(1, 9), Dice(2, 9)],
        },
        EnemyStatus { // Vampire
            treasure: 20,
            attr: EnemyAttr::from(vec![EnemyAttr::REGENERATE, EnemyAttr::MEAN]),
            exp: 350,
            hp: Dice(8, 8),
            defence: 1,
            attack: vec![Dice(1, 19)],
        },
        EnemyStatus { // Wraith
            treasure: 0,
            attr: EnemyAttr::from(vec![]),
            exp: 55,
            hp: Dice(5, 8),
            defence: 4,
            attack: vec![Dice(1, 6)],
        },
        EnemyStatus { // Xeroc
            treasure: 30,
            attr: EnemyAttr::from(vec![]),
            exp: 100,
            hp: Dice(7, 8),
            defence: 7,
            attack: vec![Dice(4, 4)],
        },
        EnemyStatus { // Yeti
            treasure: 30,
            attr: EnemyAttr::from(vec![]),
            exp: 50,
            hp: Dice(4, 8),
            defence: 6,
            attack: vec![Dice(1, 6), Dice(1, 6)],
        },
        EnemyStatus { // Zombie
            treasure: 0,
            attr: EnemyAttr::from(vec![EnemyAttr::MEAN]),
            exp: 6,
            hp: Dice(2, 8),
            defence: 8,
            attack: vec![Dice(1, 8)],
        }
    ];
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Potion,
    Scroll,
    Armor(Armor),
    Weapon(Weapon),
    Wand,
    Food(Food),
    Gold,
    Ring,
    Amulet,
    None,
}

default_none!(Item);

impl From<u8> for Item {
    fn from(u: u8) -> Item {
        match u {
            b'!' => Item::Potion,
            b'?' => Item::Scroll,
            b']' => Item::Armor(Armor::None),
            b')' => Item::Weapon(Weapon::None),
            b'/' => Item::Wand,
            b'*' => Item::Gold,
            b':' => Item::Food(Food::None),
            b'=' => Item::Ring,
            b',' => Item::Amulet,
            _ => Item::None,
        }
    }
}
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Weapon {
    Mace,
    LongSword,
    Bow,
    Dagger,
    TwoHandedSword,
    Dart,
    Shuriken,
    Spear,
    None,
}

default_none!(Weapon);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Armor {
    Leather,
    Studded,
    Ring,
    Scale,
    Chain,
    Splint,
    Banded,
    Plate,
    None,
}

default_none!(Armor);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemWithId(pub Item, pub String, pub u8, pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Food {
    Ration,
    SlimeMold,
    None,
}

default_none!(Food);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Surface {
    Road,
    Floor,
    Wall,
    Stair,
    Door,
    Trap,
    None,
}

default_none!(Surface);

impl From<u8> for Surface {
    fn from(u: u8) -> Self {
        match u {
            b'#' => Surface::Road,
            b'.' => Surface::Floor,
            b'-' | b'|' => Surface::Wall,
            b'%' => Surface::Stair,
            b'+' => Surface::Door,
            b'^' => Surface::Trap,
            _ => Surface::None,
        }
    }
}

impl Surface {
    pub fn blank() -> u8 {
        b' '
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldObject {
    Enemy(Enemy),
    Item(Item),
    Player,
    None,
}

default_none!(FieldObject);

impl From<u8> for FieldObject {
    fn from(u: u8) -> Self {
        match u {
            b'@' => FieldObject::Player,
            val if b'A' <= val && val <= b'Z' => FieldObject::Enemy(Enemy::from(u)),
            _ => {
                let item = Item::from(u);
                if item == Item::None {
                    FieldObject::None
                } else {
                    FieldObject::Item(item)
                }
            }
        }
    }
}

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

impl Default for Dangeon {
    fn default() -> Dangeon {
        Dangeon {
            origin: vec![vec![Surface::blank(); COLUMNS]; LINES],
            inner: vec![vec![Cell::default(); COLUMNS]; LINES],
        }
    }
}

impl Dangeon {
    fn fetch(&mut self, orig: Vec<Vec<u8>>) {
        for i in 0..LINES {
            for j in 0..COLUMNS {
                self.inner[i][j].obj = FieldObject::from(orig[i][j]);
                if self.inner[i][j].surface == Surface::None {
                    self.inner[i][j].surface = Surface::from(orig[i][j]);
                }
            }
        }
    }
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
