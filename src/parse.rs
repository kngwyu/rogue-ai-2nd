use regex::{Regex, RegexSet};
use data::*;
use std::collections::BinaryHeap;
pub struct StatusParse {
    re: Regex,
    is_hungry: Regex,
}

impl StatusParse {
    #[cfg_attr(feature = "clippy", allow(trivial_regex))]
    pub fn new() -> Self {
        StatusParse { re:
                          Regex::new(
                r"(?x)
Level:\D*
(?P<stage>\d*) # Stage Level
.*Gold:\D*
(?P<gold>\d*) # Gold
.*Hp:\D*
(?P<curhp>\d*) # CurHp
\(
(?P<maxhp>\d*) # MaxHp
\).*Str:\D*
(?P<curstr>\d*) # CurStr
\(
(?P<maxstr>\d*) # MaxStr
\).*Arm:\D*
(?P<arm>\d*) # Arm
.*Exp:\D*
(?P<explevel>\d*) # ExpLevel
\D*
(?P<exp>\d*) # Exp
",
            ).unwrap(),
                      is_hungry: Regex::new(r"Hungry").unwrap(), }
    }
    pub fn parse(&self, s: &str) -> Option<PlayerStatus> {
        match self.re.captures(s) {
            Some(caps) => {
                let get = |t: &str| -> u32 {
                    let capped = &caps[t];
                    capped.parse::<u32>().unwrap()
                };
                let hung = self.is_hungry.is_match(s);
                Some(PlayerStatus { stage_level: get("stage") as _,
                                    gold: get("gold"),
                                    cur_hp: get("curhp"),
                                    max_hp: get("maxhp"),
                                    cur_str: get("curstr"),
                                    max_str: get("maxstr"),
                                    arm: get("arm") as _,
                                    exp_level: get("explevel"),
                                    exp: get("exp"),
                                    hungry: hung, })
            }
            None => None,
        }
    }
}

pub struct MsgParse {
    rset: RegexSet,
    detect_enemy: Regex,
    detect_item: Regex,
    item_set: RegexSet,
    integer: Regex,
    potion: Regex,
    scroll: Regex,
    ring: Regex,
}

impl MsgParse {
    #[cfg_attr(feature = "clippy", allow(trivial_regex))]
    pub fn new() -> Self {
        MsgParse {
            rset: RegexSet::new(&[
                r"--More--",                     // 0
                r"The .*n't",                    // 1
                r"The .*? miss",                 // 2
                r"The .*? injured",              // 3
                r"The .*? hit",                  // 4
                r"Which direction",              // 5
                r"You scored",                   // 6
                r"You have defeated",            // 7
                r"You .*? miss",                 // 8
                r"You .*?n't",                   // 9
                r"You now have ",                // 10
                r"You found",                    // 11
                r"You are now wearing",          // 12
                r"You used to be wearing",       // 13
                r"Which object do you want to",  // 14
                r"Welcome to level",             // 15
                r"Yum",                          // 16
                r"There's no room in your pack", // 17
                r"You moved onto",               // 18
                r"Dropped",                      // 19
                r"do you want to call",          // 20
            ]).unwrap(),
            detect_enemy: Regex::new(r"(?i)the.*?(?P<enemy>\w)").unwrap(),
            detect_item: Regex::new(
                r"You now have (?P<num>a|\d*)[ |[\w^a]](?P<item>.*?)\((?P<id>\w)\)",
            ).unwrap(),
            item_set: RegexSet::new(&[
                r"potion",           // 0
                r"scroll",           // 1
                r"ring",             // 2
                r"food",             // 3
                r"mold",             // 4
                r"eathor armor",     // 5
                r"tudded leadther",  // 6
                r"ing mail",         // 7
                r"cale mail",        // 8
                r"hain mail",        // 9
                r"plint mail",       // 10
                r"anded mail",       // 11
                r"late mail",        // 12
                r"mace",             // 13
                r"long sword",       // 14
                r"bow",              // 15
                r"dagger",           // 16
                r"two handed sword", // 17
                r"dart",             // 18
                r"shuriken",         // 19
                r"spear",            // 20
                r"wand",             // 21
                r"staff",            // 22
                r"mulet",            // 23
            ]).unwrap(),
            integer: Regex::new(r"\D*(?P<int>\d*)").unwrap(),
            potion: Regex::new(r".*?(?P<name>\w*) potion").unwrap(),
            scroll: Regex::new(r".*'(?P<name>.*)'").unwrap(),
            ring: Regex::new(r".* (?P<name>\w*) ring").unwrap(),
        }
    }

    fn enemy(&self, s: &str) -> Enemy {
        let cap = self.detect_enemy.captures(s).unwrap();
        Enemy::from(cap["enemy"].as_bytes()[0])
    }

    fn match_item(&self, s: &str) -> Item {
        let matches: Vec<_> = self.item_set.matches(s).into_iter().collect();
        match matches[0] {
            0 => Item::Potion,
            1 => Item::Scroll,
            2 => Item::Ring,
            3 => Item::Food(Food::Ration),
            4 => Item::Food(Food::SlimeMold),
            5 => Item::Armor(Armor::Leather),
            6 => Item::Armor(Armor::Studded),
            7 => Item::Armor(Armor::Ring),
            8 => Item::Armor(Armor::Scale),
            9 => Item::Armor(Armor::Chain),
            10 => Item::Armor(Armor::Splint),
            11 => Item::Armor(Armor::Banded),
            12 => Item::Armor(Armor::Plate),
            13 => Item::Weapon(Weapon::Mace),
            14 => Item::Weapon(Weapon::LongSword),
            15 => Item::Weapon(Weapon::Bow),
            16 => Item::Weapon(Weapon::Dagger),
            17 => Item::Weapon(Weapon::TwoHandedSword),
            18 => Item::Weapon(Weapon::Dart),
            19 => Item::Weapon(Weapon::Shuriken),
            20 => Item::Weapon(Weapon::Spear),
            21 | 22 => Item::Wand,
            23 => Item::Amulet,
            _ => Item::None,
        }
    }

    fn item(&self, s: &str) -> ItemWithId {
        let cap = self.detect_item.captures(s).unwrap();
        let num = if cap["num"].is_empty() || cap["num"].as_bytes()[0] == b'a' {
            1
        } else {
            str::parse::<u32>(&cap["num"]).unwrap()
        };
        let id = cap["id"].as_bytes()[0];
        let matches: Vec<_> = self.item_set.matches(&cap["item"]).into_iter().collect();
        macro_rules! ret_item {
            ($item:expr) => (ItemWithId($item, String::new(), id, num));
            ($item:expr,$str:expr) => (ItemWithId($item, $str.to_owned(), id, num));
        }
        let ret_with_n = |item: Item, re: &Regex| -> ItemWithId {
            let cp = re.captures(&cap["item"]).unwrap();
            ret_item!(item, &cp["name"])
        };
        if matches.is_empty() {
            ret_item!(Item::None)
        } else {
            match matches[0] {
                0 => ret_with_n(Item::Potion, &self.potion),
                1 => ret_with_n(Item::Scroll, &self.scroll),
                2 => ret_with_n(Item::Ring, &self.ring),
                3 => ret_item!(Item::Food(Food::Ration)),
                4 => ret_item!(Item::Food(Food::SlimeMold)),
                5 => ret_item!(Item::Armor(Armor::Leather)),
                6 => ret_item!(Item::Armor(Armor::Studded)),
                7 => ret_item!(Item::Armor(Armor::Ring)),
                8 => ret_item!(Item::Armor(Armor::Scale)),
                9 => ret_item!(Item::Armor(Armor::Chain)),
                10 => ret_item!(Item::Armor(Armor::Splint)),
                11 => ret_item!(Item::Armor(Armor::Banded)),
                12 => ret_item!(Item::Armor(Armor::Plate)),
                13 => ret_item!(Item::Weapon(Weapon::Mace)),
                14 => ret_item!(Item::Weapon(Weapon::LongSword)),
                15 => ret_item!(Item::Weapon(Weapon::Bow)),
                16 => ret_item!(Item::Weapon(Weapon::Dagger)),
                17 => ret_item!(Item::Weapon(Weapon::TwoHandedSword)),
                18 => ret_item!(Item::Weapon(Weapon::Dart)),
                19 => ret_item!(Item::Weapon(Weapon::Shuriken)),
                20 => ret_item!(Item::Weapon(Weapon::Spear)),
                21 | 22 => ret_item!(Item::Wand),
                23 => ret_item!(Item::Amulet),
                _ => ret_item!(Item::None),
            }
        }
    }

    fn to_int(&self, s: &str) -> u32 {
        let cap = self.integer.captures(s).unwrap();
        str::parse::<u32>(&cap["int"]).unwrap()
    }

    fn gold(&self, s: &str) -> ItemWithId {
        ItemWithId(Item::Gold, String::new(), 0, self.to_int(s))
    }

    pub fn parse(&self, s: &str) -> (Msg, bool) {
        let matches: BinaryHeap<_> = self.rset.matches(s).into_iter().collect();
        let mut more = false;
        let mut res = Msg::None;
        for m in matches {
            match m {
                0 => more = true,
                1 | 2 => res = Msg::NotInjured(self.enemy(s)),
                3 | 4 => res = Msg::Injured(self.enemy(s)),
                5 => res = Msg::Direction,
                6 => res = Msg::Scored(self.enemy(s)),
                7 => res = Msg::Defeated(self.enemy(s)),
                8 | 9 => res = Msg::Missed(self.enemy(s)),
                10 => res = Msg::Item(self.item(s)),
                11 => res = Msg::Item(self.gold(s)),
                12 => res = Msg::ArmorW,
                13 => res = Msg::ArmorT,
                14 => res = Msg::WhichObj,
                15 => res = Msg::LevelUp(self.to_int(s) as _),
                16 => res = Msg::Ate,
                17 => res = Msg::PackFull,
                18 => res = Msg::MovedOnto(self.match_item(s)),
                19 => res = Msg::Dropped,
                20 => res = Msg::CallIt,
                _ => {}
            }
        }
        (res, more)
    }
}

#[cfg(test)]
mod test {
    use ::*;
    #[test]
    fn status_test() {
        let text1 = "Level: 3  Gold: 237    Hp: 18(25)  Str: 16(16)  Arm: 4   Exp: 3/23  Hungry";
        let text2 =
            "Level: 1  Gold: 0      Hp: 12(12)  Str: 16(16)  Arm: 4   Exp: 1/0               ";
        let parser = StatusParse::new();
        let res = PlayerStatus { stage_level: 3,
                                 gold: 237,
                                 cur_hp: 18,
                                 max_hp: 25,
                                 cur_str: 16,
                                 max_str: 16,
                                 arm: 4,
                                 exp_level: 3,
                                 exp: 23,
                                 hungry: true, };
        assert_eq!(res, parser.parse(text1).unwrap());
        assert_eq!(parser.parse(text2).unwrap(), PlayerStatus::default());
    }
    #[test]
    fn msg_test() {
        let drink_msgs = vec!["Hey, this tastes great.  It make you feel warm all over--More--"];
        let scroll_msgs = vec!["Your armor is covered by a shimmering gold shield"];
        let msgs = vec!["The emu has injured you",
                        "The emu swings and hits you",
                        "The bat hit you",
                        "The bat doesn't hit you",
                        "The hobgoblin barely misses you",
                        "Which direction?",
                        "You barely miss the hobgoblin--More--",
                        "You scored an excellent hit on the kestrel--More--",
                        "You have defeated the emu",
                        "You found 32 gold pieces",
                        "You now have a yellow potion (g)",
                        "You now have a scroll titled 'tuenes eepme' (h)",
                        "You now have 2 scrolls titled 'org vly gopsehzok hasnatue' (o)--More--",
                        "You now have scale mail (i)",
                        "You now have a tiger eye ring (f)",
                        "You now have 2 rations of food (a)--More--",
                        "I see no monster there",
                        "You are now wearing +1 ring mail [protection 4]",
                        "You used to be wearing b) +1 ring mail [protection 4]",
                        "Which object do you want to quaff? (* for list): ",
                        "Welcome to level 4--More--",
                        "You feel a bite in your leg and now feel weaker--More--",
                        "Yum, that tasted good",
                        "Your purse feels lighter",
                        "What do you want to call it?",
                        "There's no room in your pack--More--",
                        "You moved onto splint mail"];
        let parser = MsgParse::new();
        for s in msgs {
            println!("{}", s);
            println!("{:?}", parser.parse(s));
        }
    }
}
