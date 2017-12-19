use regex::{Regex, RegexSet};
use data::PlayerStatus;
pub struct StatusParse {
    re: Regex,
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
        }
    }
    pub fn parse(&self, s: &str) -> PlayerStatus {
        let caps = self.re.captures(s).unwrap();
        let get = |t: &str| -> u32 {
            let capped = &caps[t];
            capped.parse::<u32>().unwrap()
        };
        PlayerStatus {
            stage_level: get("stage") as _,
            gold: get("gold"),
            cur_hp: get("curhp"),
            max_hp: get("maxhp"),
            cur_str: get("curstr"),
            max_str: get("maxstr"),
            arm: get("arm") as _,
            exp_level: get("explevel"),
            exp: get("exp"),
        }
    }
}

pub struct MsgParse {
    rset: RegexSet,
    detect_enemy: Regex,
    detect_item: Regex,
    detect_int: Regex,
}

impl MsgParse {
    fn new() -> Self {
        MsgParse {
            rset: RegexSet::new(
                &[
                    r"--More--",
                    r"The .*? has injured you",
                    r"The .*? swings",
                    r"You have defeated",
                    r"Which direction",
                    r"You now have ",
                    r"You have found",
                    r"You are now wearing",
                    r"You used to be wearing",
                    r"Which object do you want to",
                    r"Welcome to level",
                    r"You feel a bite in your leg",
                    r"Yum",
                    r"Your purse",
                    r"There's no room in your pack",
                    r"You moved onto",
                ],
            )
                  .unwrap(),
            detect_enemy: Regex::new(r"The (?P<enemy>\w)").unwrap(),
            detect_item: Regex::new(r"The ").unwrap(),
            detect_int: Regex::new(r"\D*(?P<int>\d*)\D*").unwrap(),
        }
    }
    fn parse(&self, s: &str) {
        let m = self.rset.matches(s);

    }
}

#[cfg(test)]
mod test {
    use ::*;
    #[test]
    fn status_test() {
        let text = "Level: 3  Gold: 237    Hp: 18(25)  Str: 16(16)  Arm: 4   Exp: 3/23";
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
        };
        assert_eq!(res, parser.parse(text));
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
            "Which direction?",
            "You have defeated the emu",
            "You found 32 gold pieces",
            "You now have a yellow potion (g)",
            "You now have a scroll titled 'tuenes eepme' (h)",
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
            parser.parse(s);
        }
    }
}
