#![allow(clippy::needless_range_loop)]

use std::ops::{Add, AddAssign, Deref, DerefMut, Div, Mul, Sub, SubAssign};

use crate::{vector::Vector, Numeric};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Matrix<T, const N: usize, const M: usize>
where
    T: Numeric,
{
    data: [Vector<T, N>; M],
}

impl<T, const N: usize, const M: usize> Matrix<T, N, M>
where
    T: Numeric,
{
    pub fn new(data: [Vector<T, N>; M]) -> Self {
        Self { data }
    }
}

impl<T, const N: usize> Matrix<T, N, N>
where
    T: Numeric,
    T: From<u8>,
{
    pub fn identity() -> Self {
        let mut data = [Vector::default(); N];

        for i in (0..N * N).step_by(N + 1) {
            data[i % N][i / N] = 1_u8.into()
        }

        Self { data }
    }
}

impl<T: Default, const N: usize, const M: usize> Default for Matrix<T, N, M>
where
    T: Numeric,
    Vector<T, N>: Default,
    [Vector<T, N>; M]: Default,
{
    fn default() -> Self {
        let data = [Vector::default(); M];
        Self { data }
    }
}

impl<T, const N: usize, const M: usize> Matrix<T, N, M>
where
    T: Numeric,
{
    pub fn transpose(self) -> Matrix<T, M, N> {
        let mut data = [Vector::default(); N];

        for i in 0..N {
            for j in 0..M {
                data[i][j] = self[j][i];
            }
        }

        Matrix { data }
    }
}

impl<T, const N: usize> Matrix<T, N, N>
where
    T: Numeric,
    T: From<u8>,
    T: Div<Output = T>,
    T: Mul<Output = T>,
    T: PartialOrd,
{
    pub fn inverse(mut self) -> Self {
        let mut data = *Self::identity();

        //apply guass jordan elimination
        for i in 0..N {
            if self.data[i][i] == T::default() {
                panic!("attempt to divide by zero")
            }

            for j in 0..N {
                if i != j {
                    let ratio = self.data[j][i] / self.data[i][i];
                    for k in 0..N {
                        let value = self.data[i][k];
                        self.data[j][k] -= ratio * value;
                    }
                    for k in 0..N {
                        let value = data[i][k];
                        data[j][k] -= ratio * value;
                    }
                }
            }
        }

        //row operation ot make principal diagonal element equal to 1
        for i in 0..N {
            let divisor = self.data[i][i];
            for j in 0..N {
                self.data[i][j] /= divisor;
            }
            for j in 0..N {
                data[i][j] /= divisor;
            }
        }

        Self { data }
    }
}

impl<T, const N: usize, const M: usize> From<[Vector<T, N>; M]> for Matrix<T, N, M>
where
    T: Numeric,
{
    fn from(data: [Vector<T, N>; M]) -> Self {
        Self::new(data)
    }
}

impl<T, const N: usize, const M: usize> Deref for Matrix<T, N, M>
where
    T: Numeric,
{
    type Target = [Vector<T, N>; M];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T, const N: usize, const M: usize> DerefMut for Matrix<T, N, M>
where
    T: Numeric,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T, const N: usize> Add<Self> for Matrix<T, N, N>
where
    T: Numeric,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut data = self.data;
        for x in 0..N {
            for y in 0..N {
                data[x][y] += rhs.data[x][y];
            }
        }
        Self { data }
    }
}

impl<T, const N: usize> AddAssign<Self> for Matrix<T, N, N>
where
    T: Numeric,
{
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<T, const N: usize> Sub<Self> for Matrix<T, N, N>
where
    T: Numeric,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut data = self.data;
        for x in 0..N {
            for y in 0..N {
                data[x][y] -= rhs.data[x][y];
            }
        }
        Self { data }
    }
}

impl<T, const N: usize> SubAssign<Self> for Matrix<T, N, N>
where
    T: Numeric,
{
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<T, const A: usize, const B: usize, const C: usize> Mul<Matrix<T, C, A>> for Matrix<T, A, B>
where
    T: Numeric,
    T: Add<Output = T>,
    T: Mul<Output = T>,
{
    type Output = Matrix<T, C, B>;

    fn mul(self, rhs: Matrix<T, C, A>) -> Self::Output {
        let mut data = [Vector::default(); B];
        for row in 0..B {
            for col in 0..C {
                let mut cell = T::default();
                for i in 0..A {
                    cell += self.data[row][i] * rhs.data[i][col];
                }
                data[row][col] = cell;
            }
        }
        Matrix::<T, C, B>::new(data)
    }
}

impl<T, const A: usize, const B: usize> Mul<Vector<T, A>> for Matrix<T, A, B>
where
    T: Numeric,
    T: Add<Output = T>,
    T: Mul<Output = T>,
{
    type Output = Vector<T, B>;

    fn mul(self, rhs: Vector<T, A>) -> Self::Output {
        let mut data = [T::default(); B];
        for row in 0..B {
            let mut cell = T::default();
            for i in 0..A {
                cell += self.data[row][i] * rhs[i];
                data[row] = cell;
            }
        }
        Vector::<T, B>::new(data)
    }
}
