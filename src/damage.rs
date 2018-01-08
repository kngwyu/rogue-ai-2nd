// calc damage
use data::Enemy;
use std::ops::Deref;
use rand::{thread_rng, Rng};
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

float_alias!(DamageVal, f64);

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
}
