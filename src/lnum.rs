use num_traits::{Bounded, NumCast};

#[derive(Debug, Clone, PartialEq)]
pub enum LNum {
    Byte(u8),
    Int(LInt),
    Float(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LInt {
    Small(i16),
    Big(i32),
    Long(i64),
}

impl LNum {
    pub fn new(n: f64) -> Self {
        if Self::is_integer(n) {
            LNum::Int(LInt::new(n as i64))
        } else {
            LNum::Float(n)
        }
    }

    pub fn default_int() -> LNum {
        LNum::Int(LInt::Small(0))
    }

    fn is_integer(n: f64) -> bool {
        n.fract() == 0.0
    }
}

impl LInt {
    pub fn new(i: i64) -> Self {
        match i {
            _ if Self::fits_in::<i16>(i) => LInt::Small(i as i16),
            _ if Self::fits_in::<i32>(i) => LInt::Big(i as i32),
            _ => LInt::Long(i),
        }
    }

    pub fn fits_in<T: Bounded + NumCast>(f: i64) -> bool {
        let min = T::min_value().to_i64().unwrap();
        let max = T::max_value().to_i64().unwrap();
        f >= min && f <= max
    }
}
