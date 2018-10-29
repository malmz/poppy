use num_traits::PrimInt;
use num_traits::Float;
use std::marker::PhantomData;
use std::ops;
use std;

mod private {
    pub trait Sealed {}
    impl Sealed for super::Q7 {}
}

pub trait Fractional: private::Sealed {
    const Shift: u8;
}

pub enum Q7 {}
impl Fractional for Q7 { const Shift: u8 = 7; }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Fixed<T, Q> 
    where 
        Q: Fractional,
        T: PrimInt,
{
    bits: T,
    marker: PhantomData<Q>,
}

impl<T, Q> Fixed<T, Q> 
    where 
        Q: Fractional,
        T: PrimInt,
{
    pub fn from_raw(bits: T) -> Self {
        Self {
            bits,
            marker: PhantomData,
        }
    }

    pub fn from_f32(val: f32) -> Fixed<u32, Q> {
        Fixed {
            bits: (val*(2.0.powi(Q::Shift as i32)).round()).try_into(),
            marker: PhantomData,
        }
    }
}



impl<T, Q> Fixed<T, Q>
    where
        Q: Fractional,
        T: PrimInt + ops::Shl<u8, Output=T>,
{
    pub fn new(val: T) -> Self {
        Self {
            bits: val<<Q::Shift,
            marker: PhantomData,
        }
    }
}

impl <T, Q> Fixed<T, Q>
    where 
        Q: Fractional,
        T: PrimInt + ops::Shr<u8, Output=T> + ops::BitAnd<u8, Output=T>,
{
    pub fn split(self) -> (T, T) {
        (self.bits >> Q::Shift, self.bits & (1<<Q::Shift) - 1)
    }
}

impl<Q, T> ops::Add for Fixed<T, Q> 
    where 
        Q: Fractional,
        T: PrimInt + ops::Add<Output=T>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Fixed {
            bits: self.bits + rhs.bits,
            marker: PhantomData,
        }
    }
}

impl<Q, T> ops::Sub for Fixed<T, Q> 
    where 
        Q: Fractional,
        T: PrimInt + ops::Sub<Output=T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Fixed {
            bits: self.bits - rhs.bits,
            marker: PhantomData,
        }
    }
}

impl<Q, T> ops::Mul for Fixed<T, Q> 
    where 
        Q: Fractional,
        T: PrimInt + ops::Mul<Output=T> + ops::Shr<u8, Output=T>,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Fixed {
            bits: self.bits * rhs.bits >> Q::Shift,
            marker: PhantomData,
        }
    }
}