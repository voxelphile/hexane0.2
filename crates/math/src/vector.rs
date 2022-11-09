#![allow(clippy::needless_range_loop)]

use std::convert::FloatToInt;
use std::ops::{Add, AddAssign, Deref, DerefMut, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

use crate::Numeric;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Vector<T, const N: usize>
where
    T: Numeric,
{
    data: [T; N],
}

impl<T, const N: usize> Vector<T, N>
where
    T: Numeric,
{
    pub fn new(data: [T; N]) -> Self {
        Self { data }
    }
}

impl<T, const N: usize> Vector<T, N>
where
    T: Numeric,
{
    pub fn resize<const M: usize>(self) -> Vector<T, M> {
        let mut data = [T::default(); M];

        for i in 0..M {
            if i > N {
                break;
            }

            data[i] = self[i];
        }

        Vector::<T, M> { data }
    }

    pub fn cast<U>(self) -> Vector<U, N>
    where
        U: Numeric + Default + From<T>,
    {
        let mut data = [U::default(); N];

        for i in 0..N {
            data[i] = self[i].into();
        }

        Vector::<U, N> { data }
    }
}

impl<T: Default, const N: usize> Default for Vector<T, N>
where
    T: Numeric,
{
    fn default() -> Self {
        let data = [T::default(); N];
        Self { data }
    }
}

impl<T, const N: usize> From<[T; N]> for Vector<T, N>
where
    T: Numeric,
{
    fn from(data: [T; N]) -> Self {
        Self::new(data)
    }
}

impl<T, const N: usize> Deref for Vector<T, N>
where
    T: Numeric,
{
    type Target = [T; N];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, const N: usize> DerefMut for Vector<T, N>
where
    T: Numeric,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T, const N: usize> Vector<T, N>
where
    T: Numeric,
    T: Add<Output = T>,
    T: Mul<Output = T>,
{
    pub fn dot(self, rhs: Self) -> T {
        let mut dot = T::default();
        for i in 0..N {
            dot += self.data[i] * rhs.data[i];
        }
        dot
    }
}

impl<const N: usize> Vector<f32, N> {
    pub fn distance(&self, other: &Self) -> f32 {
        self.distance_squared(other).sqrt()
    }

    pub fn distance_squared(&self, other: &Self) -> f32 {
        let mut accum = 0.0;
        for i in 0..N {
            accum += (self.data[i] - other.data[i]) * (self.data[i] - other.data[i])
        }
        accum
    }

    pub fn magnitude(&self) -> f32 {
        let mut accum = 0.0;
        for i in 0..N {
            accum += self.data[i] * self.data[i];
        }
        accum.sqrt()
    }

    pub fn normalize(self) -> Self {
        let magnitude = self.magnitude();
        self / magnitude
    }

    pub fn castf<U>(self) -> Vector<U, N>
    where
        f32: FloatToInt<U>,
        U: Numeric + Default,
    {
        let mut data = [U::default(); N];

        for i in 0..N {
            data[i] = unsafe { self[i].to_int_unchecked() };
        }

        Vector::<U, N> { data }
    }
}

impl<const N: usize> Vector<f64, N> {
    pub fn distance(&self, other: &Self) -> f64 {
        self.distance_squared(other).sqrt()
    }

    pub fn distance_squared(&self, other: &Self) -> f64 {
        let mut accum = 0.0;
        for i in 0..N {
            accum += (self.data[i] - other.data[i]) * (self.data[i] - other.data[i])
        }
        accum
    }

    pub fn magnitude(&self) -> f64 {
        let mut accum = 0.0;
        for i in 0..N {
            accum += self.data[i] * self.data[i];
        }
        accum.sqrt()
    }

    pub fn normalize(self) -> Self {
        let magnitude = self.magnitude();
        self / magnitude
    }

    pub fn castf<U>(self) -> Vector<U, N>
    where
        f64: FloatToInt<U>,
        U: Numeric + Default,
    {
        let mut data = [U::default(); N];

        for i in 0..N {
            data[i] = unsafe { self[i].to_int_unchecked() };
        }

        Vector::<U, N> { data }
    }
}

//TODO seven dimension cross product lol
impl<T> Vector<T, 3>
where
    T: Numeric,
    T: Mul<Output = T>,
    T: Sub<Output = T>,
    [T; 3]: Default,
{
    pub fn cross(self, rhs: Self) -> Self {
        let mut data = [T::default(); 3];
        data[0] = self.data[1] * rhs.data[2] - self.data[2] * rhs.data[1];
        data[1] = self.data[2] * rhs.data[0] - self.data[0] * rhs.data[2];
        data[2] = self.data[0] * rhs.data[1] - self.data[1] * rhs.data[0];
        Self { data }
    }
}

impl<T, const N: usize> Add for Vector<T, N>
where
    T: Numeric,
    T: Add<Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut data = [T::default(); N];
        for i in 0..N {
            data[i] = self.data[i] + rhs.data[i];
        }
        Self { data }
    }
}

impl<T, const N: usize> AddAssign for Vector<T, N>
where
    T: Numeric,
    T: Add<Output = T>,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<T, const N: usize> Sub for Vector<T, N>
where
    T: Numeric,
    T: Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut data = [T::default(); N];
        for i in 0..N {
            data[i] = self.data[i] - rhs.data[i];
        }
        Self { data }
    }
}

impl<T, const N: usize> SubAssign for Vector<T, N>
where
    T: Numeric,
    T: Sub<Output = T>,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<T, const N: usize> Mul<T> for Vector<T, N>
where
    T: Numeric,
    T: Mul<Output = T>,
{
    type Output = Self;
    fn mul(self, rhs: T) -> Self::Output {
        let mut data = self.data;
        for i in 0..N {
            data[i] = data[i] * rhs;
        }
        Self { data }
    }
}

impl<T, const N: usize> MulAssign<T> for Vector<T, N>
where
    T: Numeric,
    T: Mul<Output = T>,
{
    fn mul_assign(&mut self, rhs: T) {
        *self = *self * rhs;
    }
}

impl<T, const N: usize> Div<T> for Vector<T, N>
where
    T: Numeric,
    T: Div<Output = T>,
{
    type Output = Self;
    fn div(self, rhs: T) -> Self::Output {
        let mut data = self.data;
        for i in 0..N {
            data[i] = data[i] / rhs;
        }
        Self { data }
    }
}

impl<T, const N: usize> DivAssign<T> for Vector<T, N>
where
    T: Numeric,
    T: Div<Output = T>,
{
    fn div_assign(&mut self, rhs: T) {
        *self = *self / rhs;
    }
}
