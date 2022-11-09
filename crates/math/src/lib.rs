#![feature(convert_float_to_int)]

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, Sub, SubAssign};

pub mod matrix;
pub mod vector;

pub mod prelude {
    pub use crate::{matrix::Matrix, vector::Vector};
}

pub trait Numeric:
    Add
    + AddAssign
    + Sub
    + SubAssign
    + Mul
    + SubAssign
    + Div
    + DivAssign
    + Clone
    + Copy
    + Default
    + std::fmt::Debug
{
}

impl<T> Numeric for T where
    T: Add
        + AddAssign
        + Sub
        + SubAssign
        + Mul
        + SubAssign
        + Div
        + DivAssign
        + Clone
        + Copy
        + Default
        + std::fmt::Debug
{
}
