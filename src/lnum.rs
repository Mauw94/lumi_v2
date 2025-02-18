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

    pub fn default_float() -> LNum {
        LNum::Float(0.0)
    }

    pub fn real_val(&self) -> f64 {
        match self {
            LNum::Byte(b) => *b as f64,
            LNum::Int(lint) => match lint {
                LInt::Small(i) => *i as f64,
                LInt::Big(i) => *i as f64,
                LInt::Long(i) => *i as f64,
            },
            LNum::Float(f) => *f,
        }
    }

    pub fn negate(&self) -> Self {
        match self {
            LNum::Byte(_) => panic!("Cannot negate byte."),
            LNum::Int(lint) => LNum::Int(LInt::negate(lint)),
            LNum::Float(f) => LNum::Float(-*f),
        }
    }

    fn is_integer(n: f64) -> bool {
        n.fract() == 0.0
    }
}

impl std::fmt::Display for LNum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LNum::Byte(b) => write!(f, "{}", b),
            LNum::Int(lint) => write!(f, "{}", lint),
            LNum::Float(fl) => write!(f, "{}", fl),
        }
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

    pub fn negate(&self) -> Self {
        match self {
            LInt::Small(s) => LInt::Small(-*s),
            LInt::Big(b) => LInt::Big(-*b),
            LInt::Long(l) => LInt::Long(-*l),
        }
    }

    pub fn fits_in<T: Bounded + NumCast>(f: i64) -> bool {
        let min = T::min_value().to_i64().unwrap();
        let max = T::max_value().to_i64().unwrap();
        f >= min && f <= max
    }
}

impl std::fmt::Display for LInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LInt::Small(s) => write!(f, "{}", s),
            LInt::Big(b) => write!(f, "{}", b),
            LInt::Long(l) => write!(f, "{}", l),
        }
    }
}
