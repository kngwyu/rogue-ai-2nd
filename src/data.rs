// domain knowleadge
// enemy data is from https://nethackwiki.com/wiki/Rogue_(game), thanks

macro_rules! default_none {
    ($enum:ident) => {
        impl Default for $enum {
            fn default()-> $enum {
                $enum::None
            }
        }
    };
}

pub trait Fetch {
    type Message: ?Sized;
    fn fetch(&mut self) -> Self::Message;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerStatus {
    pub stage_level: u8,
    pub gold: u32,
    pub cur_hp: u32,
    pub max_hp: u32,
    pub cur_str: u32,
    pub max_str: u32,
    pub arm: u8,
    pub exp_level: u32,
    pub exp: u32,
    pub hungry: bool,
}

impl Default for PlayerStatus {
    fn default() -> Self {
        PlayerStatus { stage_level: 1,
                       gold: 0,
                       cur_hp: 12,
                       max_hp: 12,
                       cur_str: 16,
                       max_str: 16,
                       arm: 4,
                       exp_level: 1,
                       exp: 0,
                       hungry: false, }
    }
}

pub enum PlayerFetchResult {

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
    Kestral,
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
            b'k' | b'K' => Enemy::Kestral,
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

bitflags! {
    pub struct EnemyAttr: u8 {
        const MEAN       = 0b00001;
        const FLYING     = 0b00010;
        const REGENERATE = 0b00100;
        const GREEDY     = 0b01000;
        const INVISIBLE  = 0b10000;
    }
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
            _ => Surface::None,
        }
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
