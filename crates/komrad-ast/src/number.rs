use crate::prelude::literal;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Add, Div, Mul, Rem, Sub};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Number {
    Int(literal::Int),
    UInt(literal::UInt),
    Float(literal::Float),
}

impl Number {
    pub fn is_zero(&self) -> bool {
        match self {
            Number::Int(i) => *i == 0,
            Number::UInt(u) => *u == 0,
            Number::Float(f) => *f == 0.0,
        }
    }
}

impl Hash for Number {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Number::Int(i) => i.hash(state),
            Number::UInt(u) => u.hash(state),
            Number::Float(f) => f.to_bits().hash(state),
        }
    }
}

impl From<literal::Int> for Number {
    fn from(value: literal::Int) -> Self {
        Number::Int(value)
    }
}

impl From<literal::UInt> for Number {
    fn from(value: literal::UInt) -> Self {
        Number::UInt(value)
    }
}

impl From<literal::Float> for Number {
    fn from(value: literal::Float) -> Self {
        Number::Float(value)
    }
}

impl Add for Number {
    type Output = Number;

    fn add(self, other: Number) -> Self::Output {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => Number::Int(i1 + i2),
            (Number::UInt(u1), Number::UInt(u2)) => Number::UInt(u1 + u2),
            (Number::Float(f1), Number::Float(f2)) => Number::Float(f1 + f2),
            _ => panic!("Cannot add different number types"),
        }
    }
}

impl Sub for Number {
    type Output = Number;

    fn sub(self, other: Number) -> Self::Output {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => Number::Int(i1 - i2),
            (Number::UInt(u1), Number::UInt(u2)) => Number::UInt(u1 - u2),
            (Number::Float(f1), Number::Float(f2)) => Number::Float(f1 - f2),
            _ => panic!("Cannot subtract different number types"),
        }
    }
}

impl Mul for Number {
    type Output = Number;

    fn mul(self, other: Number) -> Self::Output {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => Number::Int(i1 * i2),
            (Number::UInt(u1), Number::UInt(u2)) => Number::UInt(u1 * u2),
            (Number::Float(f1), Number::Float(f2)) => Number::Float(f1 * f2),
            _ => panic!("Cannot multiply different number types"),
        }
    }
}

impl Div for Number {
    type Output = Number;

    fn div(self, other: Number) -> Self::Output {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => Number::Int(i1 / i2),
            (Number::UInt(u1), Number::UInt(u2)) => Number::UInt(u1 / u2),
            (Number::Float(f1), Number::Float(f2)) => Number::Float(f1 / f2),
            _ => panic!("Cannot divide different number types"),
        }
    }
}

impl Rem for Number {
    type Output = Number;

    fn rem(self, other: Number) -> Self::Output {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => Number::Int(i1 % i2),
            (Number::UInt(u1), Number::UInt(u2)) => Number::UInt(u1 % u2),
            (Number::Float(f1), Number::Float(f2)) => Number::Float(f1 % f2),
            _ => panic!("Cannot calculate remainder of different number types"),
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => i1 == i2,
            (Number::UInt(u1), Number::UInt(u2)) => u1 == u2,
            (Number::Float(f1), Number::Float(f2)) => f1 == f2,
            _ => false,
        }
    }
}

impl Eq for Number {}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => i1.partial_cmp(i2),
            (Number::UInt(u1), Number::UInt(u2)) => u1.partial_cmp(u2),
            (Number::Float(f1), Number::Float(f2)) => f1.partial_cmp(f2),
            _ => None,
        }
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Number::Int(i1), Number::Int(i2)) => i1.cmp(i2),
            (Number::UInt(u1), Number::UInt(u2)) => u1.cmp(u2),
            (Number::Float(f1), Number::Float(f2)) => f1.partial_cmp(f2).unwrap(),
            _ => panic!("Cannot compare different number types"),
        }
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::Int(i) => write!(f, "{}", i),
            Number::UInt(u) => write!(f, "{}", u),
            Number::Float(fl) => write!(f, "{}", fl),
        }
    }
}

impl Number {}
