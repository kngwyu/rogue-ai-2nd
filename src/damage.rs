// calc damage
use data::{Enemy, EnemyHist, PlayerStatus, Weapon};
use std::ops::Deref;
use std::cmp::{max, min};
use rand::{thread_rng, Rng};

float_alias!(DamageVal, f64);
float_alias!(ProbVal, f64, -0.1 => 1.1);

// 攻撃するがわのレベル、アーマー(10 - 実際の表示)、補正値
fn hit_rate_sub(level: i32, armor: i32, correct: i32) -> ProbVal {
    let mut val = level + armor + correct;
    val = min(val, 20);
    val = max(val, 0);
    ProbVal(val as f64 / 20.0f64)
}

// 補正値はRunningとstrength以外考慮しない
pub fn hit_rate_attack(player: &PlayerStatus, ene: &EnemyHist) -> ProbVal {
    let st = player.cur_str;
    let str_p = str_plus(st).unwrap_or_default() + if ene.running { 0 } else { 4 };
    hit_rate_sub(player.exp_level, ene.typ.defence(), str_p + 1)
}

// 補正値は考慮しない
pub fn hit_rate_deffence(player: &PlayerStatus, ene: &Enemy) -> ProbVal {
    let arm = 10 - player.arm;
    hit_rate_sub(ene.level(), arm as i32, 1)
}

pub fn expect_dam_attack(player: &PlayerStatus, weapon: Weapon, throw: bool) -> DamageVal {
    let dice = if throw {
        weapon.throw()
    } else {
        weapon.wield()
    };
    let plus = DamageVal(add_dam(player.cur_str).unwrap_or_default() as f64);
    dice.expect_val() + plus
}

// 補正値はたぶんない
pub fn expect_dam_deffence(enem: Enemy) -> DamageVal {
    enem.attack().expect_val()
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Dice {
    num: i32,
    typ: i32,
}

impl Dice {
    pub fn new(n: i32, t: i32) -> Dice {
        Dice { num: n, typ: t }
    }
}

pub trait DiceDamage {
    fn expect_val(self) -> DamageVal;
    fn random_val(self) -> DamageVal;
    fn min_val(self) -> DamageVal;
    fn max_val(self) -> DamageVal;
}

impl DiceDamage for Dice {
    fn expect_val(self) -> DamageVal {
        let sum = (1..self.typ + 1).fold(0f64, |acc, x| acc + x as f64);
        DamageVal((sum * self.num as f64) / self.typ as f64)
    }
    fn random_val(self) -> DamageVal {
        let mut rng = thread_rng();
        let sum = (0..self.num).fold(0.0, |acc, _| acc + rng.gen_range(0, self.typ) as f64 + 1.0);
        DamageVal(sum)
    }
    fn min_val(self) -> DamageVal {
        DamageVal(self.num as f64)
    }
    fn max_val(self) -> DamageVal {
        DamageVal((self.num * self.typ) as f64)
    }
}

impl<I, T> DiceDamage for I
where
    I: IntoIterator<Item = T>,
    T: Deref<Target = Dice>,
{
    // 線形性があるから、足すだけ
    fn expect_val(self) -> DamageVal {
        self.into_iter()
            .fold(DamageVal::default(), |acc, d| acc + d.expect_val())
    }
    fn random_val(self) -> DamageVal {
        self.into_iter()
            .fold(DamageVal::default(), |acc, d| acc + d.random_val())
    }
    fn min_val(self) -> DamageVal {
        self.into_iter()
            .fold(DamageVal::default(), |acc, d| acc + d.min_val())
    }
    fn max_val(self) -> DamageVal {
        self.into_iter()
            .fold(DamageVal::default(), |acc, d| acc + d.random_val())
    }
}

fn str_plus(strength: i32) -> Option<i32> {
    const STR_PLUS: [i32; 32] = [
        -7, -6, -5, -4, -3, -2, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2,
        2, 2, 2, 3,
    ];
    if strength <= 0 || strength > 32 {
        return None;
    }
    Some(STR_PLUS[strength as usize - 1])
}

fn add_dam(strength: i32) -> Option<i32> {
    const ADD_DAM: [i32; 32] = [
        -7, -6, -5, -4, -3, -2, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 3, 3, 4, 5, 5, 5, 5, 5, 5,
        5, 5, 5, 6,
    ];
    if strength <= 0 || strength > 32 {
        return None;
    }
    Some(ADD_DAM[strength as usize - 1])
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn dice_test() {
        assert_approx_eq!(*Dice::new(1, 6).expect_val(), 3.5);
        assert_approx_eq!(*Dice::new(3, 6).expect_val(), 10.5);
        let v = vec![Dice::new(1, 6), Dice::new(1, 5)];
        assert_approx_eq!(*v.expect_val(), 6.5);
    }
    #[test]
    fn hit_rate_test() {
        let player = PlayerStatus::initial();
        let ene = EnemyHist::from_type(Enemy::Emu);
        println!("{:?}", hit_rate_attack(&player, &ene));
        println!("{:?}", hit_rate_deffence(&player, &ene.typ));
    }
}
