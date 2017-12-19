use regex::{Regex, RegexSet};
use data::{PlayerStatus, Enemy, Msg};
use std::collections::BinaryHeap;
pub struct StatusParse {
    re: Regex,
    is_hungry: Regex,
}
impl StatusParse {
    pub fn new() -> Self {
        StatusParse {
            re: Regex::new(
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
            )
                .unwrap(),
            is_hungry: Regex::new(r"Hungry").unwrap(),
        }
    }
    pub fn parse(&self, s: &str) -> Option<PlayerStatus> {
        match self.re.captures(s) {
            Some(caps) => {
                let get = |t: &str| -> u32 {
                    let capped = &caps[t];
                    capped.parse::<u32>().unwrap()
                };
                let hung = self.is_hungry.is_match(s);
                Some(PlayerStatus {
                    stage_level: get("stage") as _,
                    gold: get("gold"),
                    cur_hp: get("curhp"),
                    max_hp: get("maxhp"),
                    cur_str: get("curstr"),
                    max_str: get("maxstr"),
                    arm: get("arm") as _,
                    exp_level: get("explevel"),
                    exp: get("exp"),
                    hungry: hung,
                })
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
    detect_int: Regex,
}

impl MsgParse {
    fn new() -> Self {
        MsgParse {
            rset: RegexSet::new(
                &[
                    r"--More--",                     // 0
                    r"The .*n't",                  // 1
                    r"The .*? miss",                 // 2
                    r"The .*? injured",              // 3
                    r"The .*? hit",                  // 4
                    r"Which direction",              // 5
                    r"You scored",                   // 6
                    r"You have defeated",            // 7
                    r"You .*? miss",                 // 8
                    r"You .*?n't",                  // 9
                    r"You now have ",                // 10
                    r"You have found",               // 11
                    r"You are now wearing",          // 12
                    r"You used to be wearing",       // 13
                    r"Which object do you want to",  // 14
                    r"Welcome to level",             // 15
                    r"You feel a bite in your leg",  // 16
                    r"Yum",                          // 17
                    r"Your purse",                   // 18
                    r"There's no room in your pack", // 19
                    r"You moved onto",               // 20
                ],
            )
                  .unwrap(),
            detect_enemy: Regex::new(r"(?i)the.*?(?P<enemy>\w)").unwrap(),
            detect_item: Regex::new(r"You now have (?P<num>a|\d*) (?P<item>.*?)\((?P<id>\w)\)")
                .unwrap(),
            item_set: RegexSet::new(
                &[r"portion", r"scroll", r"mail", r"staff", r"[food]|[mold]"],
            )
                      .unwrap(),
            detect_int: Regex::new(r"\D*(?P<int>\d*)\D*").unwrap(),
        }
    }
    fn enemy(&self, s: &str) -> Enemy {
        let cap = self.detect_enemy.captures(s).unwrap();
        Enemy::from_byte(cap["enemy"].as_bytes()[0])
    }
    fn parse(&self, s: &str) -> (Msg, bool) {
        let matches: BinaryHeap<_> = self.rset.matches(s).into_iter().collect();
        let mut more = false;
        let mut res = Msg::None;
        for m in matches {
            print!("{}", m);
            match m {
                0 => more = true,
                1 | 2 => res = Msg::NotInjured(self.enemy(s)),
                3 | 4 => res = Msg::Injured(self.enemy(s)),
                5 => res = Msg::Direction,
                6 => res = Msg::Defeated(self.enemy(s)),
                7 => res = Msg::Scored(self.enemy(s)),                
                8 | 9 => res = Msg::Missed(self.enemy(s)),
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
        let text = "Level: 3  Gold: 237    Hp: 18(25)  Str: 16(16)  Arm: 4   Exp: 3/23  Hungry";
        let parser = StatusParse::new();
        let res = PlayerStatus {
            stage_level: 3,
            gold: 237,
            cur_hp: 18,
            max_hp: 25,
            cur_str: 16,
            max_str: 16,
            arm: 4,
            exp_level: 3,
            exp: 23,
            hungry: true,
        };
        assert_eq!(res, parser.parse(text).unwrap());
    }
    #[test]
    fn msg_test() {
        let drink_msgs = vec![
            "Hey, this tastes great.  It make you feel warm all over--More--",
        ];
        let scroll_msgs = vec!["Your armor is covered by a shimmering gold shield"];
        let msgs = vec![
            "The emu has injured you",
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
            "You moved onto splint mail",
        ];
        let parser = MsgParse::new();
        for s in msgs {
            println!("{}", s);
            println!("{:?}", parser.parse(s));
        }
    }
}
