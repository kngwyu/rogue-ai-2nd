// domain knowleadge
// enemy data is from https://nethackwiki.com/wiki/Rogue_(game), thanks

use std::slice;
use std::cmp;
use dangeon::Coord;
use cgw::AsciiChar;
use damage::*;
#[macro_export]
macro_rules! default_none {
    ($enum:ident) => {
        impl Default for $enum {
            fn default()-> $enum {
                $enum::None
            }
        }
    };
}

macro_rules! enum_with_iter {
    ($name: ident { $($var: ident),*$(,)*}) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub enum $name {
            $($var),*,
        }
        impl $name {
            pub fn vars() -> slice::Iter<'static, $name> {
                static VARS: &'static [$name] = &[$($name::$var),*];
                VARS.into_iter()
            }
        }
    }
}

enum_with_iter!(Direc {
    Up,
    Down,
    Left,
    Right,
    LeftUp,
    RightUp,
    LeftDown,
    RightDown,
    Stay,
});

impl Default for Direc {
    fn default() -> Self {
        Direc::Stay
    }
}

impl Into<u8> for Direc {
    fn into(self) -> u8 {
        match self {
            Direc::Stay => b'.',
            Direc::Up => b'k',
            Direc::Down => b'j',
            Direc::Left => b'h',
            Direc::Right => b'l',
            Direc::LeftUp => b'y',
            Direc::RightUp => b'u',
            Direc::LeftDown => b'b',
            Direc::RightDown => b'n',
        }
    }
}

impl Direc {
    pub fn to_cd(&self) -> Coord {
        macro_rules! cd {
            ($x: expr, $y: expr) => (Coord {x: $x, y: $y})
        }
        use Direc::*;
        match *self {
            Stay => cd!(0, 0),
            Up => cd!(0, -1),
            RightUp => cd!(1, -1),
            Right => cd!(1, 0),
            RightDown => cd!(1, 1),
            Down => cd!(0, 1),
            LeftDown => cd!(-1, 1),
            Left => cd!(-1, 0),
            LeftUp => cd!(-1, -1),
        }
    }
    pub fn rotate(&self) -> Direc {
        use Direc::*;
        match *self {
            Up => RightUp,
            RightUp => Right,
            Right => RightDown,
            RightDown => Down,
            Down => LeftDown,
            LeftDown => Left,
            Left => LeftUp,
            LeftUp => Up,
            Stay => Stay,
        }
    }
    pub fn rotate_n(&self, n: usize) -> Direc {
        let n = n % 8;
        (0..n).fold(*self, |acc, _| acc.rotate())
    }
    pub fn is_diag(&self) -> bool {
        use Direc::*;
        match *self {
            RightUp | RightDown | LeftUp | LeftDown => true,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Action {
    Move(Direc),
    Fight(Direc),
    Throw((Direc, u8)),
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
    Die,
    Space,
    None,
}

default_none!(Action);

lazy_static! {
    static ref ENTER: u8 = AsciiChar::CarriageReturn.as_byte();
    static ref SPACE: u8 = AsciiChar::Space.as_byte();
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
            Action::Quit => vec![b'Q', b'y'],
            Action::Die => vec![*ENTER, *ENTER],
            Action::Space => vec![*SPACE],
            Action::None => vec![],
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
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

impl PlayerStatus {
    pub fn initial() -> PlayerStatus {
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
    fn diff(&self, other: &PlayerStatus) -> PlayerStatus {
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
    pub fn merge(&mut self, new_stat: PlayerStatus) -> PlayerStatus {
        let res = self.diff(&new_stat);
        *self = new_stat;
        res
    }
    pub fn have_enough_hp(&self) -> bool {
        let threshold = cmp::max(8, self.max_hp / 2 + 1);
        self.cur_hp >= threshold
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameMsg {
    NotInjured(Enemy),
    Injured(Enemy),
    Direction,
    Scored(Enemy),
    Defeated(Enemy),
    Missed(Enemy),
    Item(ItemPack),
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

default_none!(GameMsg);

impl GameMsg {
    pub fn near_enemy(&self) -> bool {
        use GameMsg::*;
        match *self {
            NotInjured(_) | Injured(_) | Scored(_) | Defeated(_) | Missed(_) => true,
            _ => false,
        }
    }
}

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
    pub fn treasure(self) -> i32 {
        self.status().treasure
    }
    pub fn exp(self) -> i32 {
        self.status().exp
    }
    pub fn level(self) -> i32 {
        self.status().level
    }
    pub fn defence(self) -> i32 {
        self.status().defence
    }
    pub fn attack(self) -> &'static Vec<Dice> {
        &self.status().attack
    }
    pub fn hp(self) -> Dice {
        Dice::new(self.status().level, 8)
    }
    pub fn has_attr(self, attr: EnemyAttr) -> bool {
        self.status().attr.contains(attr)
    }
}

#[derive(Clone, Debug)]
pub struct EnemyHist {
    pub cd: Coord,
    pub hp_ex: DamageVal,
    pub running: bool,
    pub typ: Enemy,
    pub visible: bool, // このフィールドは探索時は無視
}

impl EnemyHist {
    pub fn new(typ: Enemy, cd: Coord) -> EnemyHist {
        EnemyHist {
            cd: cd,
            hp_ex: typ.hp().expect_val(),
            running: false,
            typ: typ,
            visible: true,
        }
    }
    // just for test
    pub fn from_type(typ: Enemy) -> EnemyHist {
        EnemyHist {
            cd: Coord::default(),
            hp_ex: typ.hp().expect_val(),
            running: false,
            typ: typ,
            visible: true,
        }
    }
    pub fn is_live(&self) -> bool {
        let threshold = -0.5;
        *self.hp_ex > threshold
    }
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

macro_rules! enem_attr {
    () => (EnemyAttr::NONE);
    ($x:ident) => (EnemyAttr::$x);
    ($x:ident, $($y:ident),*) => ({
        let mut res = enem_attr!($($y),*);
        res.insert(EnemyAttr::$x);
        res
    })
}

#[derive(Debug)]
struct EnemyStatus {
    treasure: i32,     // gold
    attr: EnemyAttr, // MEANとか ゲーム内でflag使わないでif文処理してるやつもこれで
    exp: i32,        // 得られる経験値
    level: i32,      // レベル (多分hit率およびhp)
    defence: i32,    // アーマー(これも多分hit率だけ)
    attack: Vec<Dice>, // 攻撃
}

lazy_static!{
    static ref ENEMIES: [EnemyStatus; 26] =[
        EnemyStatus { // Aquator
            treasure: 0,
            attr: enem_attr!(MEAN, RUSTS_ARMOR),
            exp: 20,
            level: 5,
            defence: 2,
            attack: vec![Dice::new(0, 0)],
        },
        EnemyStatus { // Bat
            treasure: 0,
            attr: enem_attr!(FLYING, RANDOM),
            exp: 1,
            level: 1,
            defence: 3,
            attack: vec![Dice::new(1, 2)],
        },
        EnemyStatus { // Centaur
            treasure: 15,
            attr: enem_attr!(),
            exp: 17,
            level: 4,
            defence: 4,
            attack: vec![Dice::new(1, 2), Dice::new(1, 5), Dice::new(1, 5)],
        },
        EnemyStatus { // Dragon
            treasure: 100,
            attr: enem_attr!(MEAN),
            exp: 5000,
            level: 10,
            defence: 3,
            attack: vec![Dice::new(1, 8), Dice::new(1, 8), Dice::new(3, 10)],
        },
        EnemyStatus { // Emu
            treasure: 0,
            attr: enem_attr!(MEAN),
            exp: 2,
            level: 1,
            defence: 7,
            attack: vec![Dice::new(1, 2)],
        },
        EnemyStatus { // Venus Flytrap
            treasure: 0,
            attr: enem_attr!(MEAN),
            exp: 80,
            level: 8,
            defence: 3,
            attack: vec![], // special
        },
        EnemyStatus { // Griffin
            treasure: 20,
            attr: enem_attr!(FLYING, MEAN, REGENERATE),
            exp: 2000,
            level: 13,
            defence: 2,
            attack: vec![Dice::new(4, 3), Dice::new(3, 5)],
        },
        EnemyStatus { // Hobgoblin
            treasure: 0,
            attr: enem_attr!(MEAN),
            exp: 3,
            level: 1,
            defence: 5,
            attack: vec![Dice::new(1, 8)],
        },
        EnemyStatus { // Icemonster
            treasure: 0,
            attr: enem_attr!(FREEZES),
            exp: 5,
            level: 1,
            defence: 9,
            attack: vec![Dice::new(0, 0)],
        },
        EnemyStatus { // Jabberwock
            treasure: 70,
            attr: enem_attr!(),
            exp: 3000,
            level: 15,
            defence: 6,
            attack: vec![Dice::new(2, 12), Dice::new(2, 4)],
        },
        EnemyStatus { // Kestrel
            treasure: 0,
            attr: enem_attr!(),
            exp: 1,
            level: 1,
            defence: 7,
            attack: vec![Dice::new(1, 4)],
        },
        EnemyStatus { // Leperachaun
            treasure: 0,
            attr: enem_attr!(STEAL_GOLD),
            exp: 10,
            level: 3,
            defence: 8,
            attack: vec![Dice::new(1, 1)],
        },
        EnemyStatus { // Medusa
            treasure: 40,
            attr: enem_attr!(MEAN),
            exp: 200,
            level: 8,
            defence: 2,
            attack: vec![Dice::new(3, 4), Dice::new(3, 4), Dice::new(2, 5)],
        },
        EnemyStatus { // Nymph
            treasure: 100,
            attr: enem_attr!(),
            exp: 37,
            level: 3,
            defence: 9,
            attack: vec![Dice::new(0, 0)],
        },
        EnemyStatus { // Orc
            treasure: 15,
            attr: enem_attr!(GREEDY),
            exp: 5,
            level: 1,
            defence: 6,
            attack: vec![Dice::new(1, 8)],
        },
        EnemyStatus { // Phantom
            treasure: 0,
            attr: enem_attr!(INVISIBLE),
            exp: 120,
            level: 8,
            defence: 3,
            attack: vec![Dice::new(4, 4)],
        },
        EnemyStatus { // Quagga
            treasure: 0,
            attr: enem_attr!(MEAN),
            exp: 15,
            level: 3,
            defence: 3,
            attack: vec![Dice::new(1, 5), Dice::new(1, 5)],
        },
        EnemyStatus { // Rattlesnake
            treasure: 0,
            attr: enem_attr!(REDUCE_STR, MEAN),
            exp: 9,
            level: 2,
            defence: 3,
            attack: vec![Dice::new(1, 6)],
        },
        EnemyStatus { // Snake
            treasure: 0,
            attr: enem_attr!(MEAN),
            exp: 2,
            level: 1,
            defence: 5,
            attack: vec![Dice::new(1, 3)],
        },
        EnemyStatus { // Troll
            treasure: 50,
            attr: enem_attr!(MEAN, REGENERATE),
            exp: 120,
            level: 6,
            defence: 4,
            attack: vec![Dice::new(1, 8), Dice::new(1, 8), Dice::new(2, 6)],
        },
        EnemyStatus { // Urvile (Black Unicorn)
            treasure: 0,
            attr: enem_attr!(MEAN),
            exp: 190,
            level: 7,
            defence: -2,
            attack: vec![Dice::new(1, 9), Dice::new(1, 9), Dice::new(2, 9)],
        },
        EnemyStatus { // Vampire
            treasure: 20,
            attr: enem_attr!(MEAN, REGENERATE),
            exp: 350,
            level: 8,
            defence: 1,
            attack: vec![Dice::new(1, 19)],
        },
        EnemyStatus { // Wraith
            treasure: 0,
            attr: enem_attr!(),
            exp: 55,
            level: 5,
            defence: 4,
            attack: vec![Dice::new(1, 6)],
        },
        EnemyStatus { // Xeroc
            treasure: 30,
            attr: enem_attr!(),
            exp: 100,
            level: 7,
            defence: 7,
            attack: vec![Dice::new(4, 4)],
        },
        EnemyStatus { // Yeti
            treasure: 30,
            attr: enem_attr!(),
            exp: 50,
            level: 4,
            defence: 6,
            attack: vec![Dice::new(1, 6), Dice::new(1, 6)],
        },
        EnemyStatus { // Zombie
            treasure: 0,
            attr: enem_attr!(MEAN),
            exp: 6,
            level: 2,
            defence: 8,
            attack: vec![Dice::new(1, 8)],
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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ItemPack {
    pub id: u8,
    pub name: String,
    pub num: u32,
    pub typ: Item,
    pub val: Option<i32>, // Armorの固有値など
}

impl ItemPack {
    pub fn new(id: u8, name: &str, num: u32, typ: Item) -> ItemPack {
        ItemPack {
            id: id,
            name: name.to_owned(),
            num: num,
            typ: typ,
            val: None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Weapon {
    Mace,
    LongSword,
    Bow,
    Arrow,
    Dagger,
    TwoHandedSword,
    Dart,
    Shuriken,
    Spear,
    None,
}

default_none!(Weapon);

bitflags! {
    pub struct WeaponAttr: u8 {
        const NONE = 0b000;
        const MANY = 0b001;
        const MISL = 0b010;
    }
}

impl Default for WeaponAttr {
    fn default() -> WeaponAttr {
        WeaponAttr::NONE
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct WeaponStatus {
    attr: WeaponAttr,
    wield: Dice,
    throw: Dice,
}
impl WeaponStatus {
    fn new(a: WeaponAttr, w: Dice, t: Dice) -> WeaponStatus {
        WeaponStatus {
            attr: a,
            wield: w,
            throw: t,
        }
    }
}
impl Weapon {
    fn status(self) -> WeaponStatus {
        let type0 = WeaponAttr::NONE;
        let type1 = WeaponAttr::MANY | WeaponAttr::MISL;
        let type2 = WeaponAttr::MISL;
        match self {
            Weapon::Mace => WeaponStatus::new(type0, Dice::new(2, 4), Dice::new(1, 3)),
            Weapon::LongSword => WeaponStatus::new(type0, Dice::new(3, 4), Dice::new(1, 2)),
            Weapon::Bow => WeaponStatus::new(type0, Dice::new(1, 1), Dice::new(1, 1)),
            Weapon::Arrow => WeaponStatus::new(type1, Dice::new(1, 1), Dice::new(2, 3)),
            Weapon::Dagger => WeaponStatus::new(type2, Dice::new(1, 6), Dice::new(1, 4)),
            Weapon::TwoHandedSword => WeaponStatus::new(type0, Dice::new(4, 4), Dice::new(1, 2)),
            Weapon::Dart => WeaponStatus::new(type1, Dice::new(1, 1), Dice::new(1, 3)),
            Weapon::Shuriken => WeaponStatus::new(type1, Dice::new(1, 2), Dice::new(2, 4)),
            Weapon::Spear => WeaponStatus::new(type2, Dice::new(2, 3), Dice::new(1, 6)),
            Weapon::None => WeaponStatus::new(type0, Dice::new(1, 4), Dice::default()),
        }
    }
    pub fn has_attr(self, attr: WeaponAttr) -> bool {
        self.status().attr.contains(attr)
    }
    pub fn wield(self) -> Dice {
        self.status().wield
    }
    pub fn throw(self) -> Dice {
        self.status().throw
    }
}

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
    pub fn need_guess(&self) -> bool {
        match *self {
            Surface::Stair | Surface::Trap | Surface::None => true,
            _ => false,
        }
    }
    pub fn can_be_floor(&self) -> bool {
        match *self {
            Surface::Floor | Surface::Trap | Surface::Stair => true,
            _ => false,
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

impl FieldObject {
    pub fn is_item(&self) -> bool {
        match *self {
            FieldObject::Item(_) => true,
            _ => false,
        }
    }
}
