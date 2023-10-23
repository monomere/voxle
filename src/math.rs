#![allow(dead_code)]

use std::ops::*;

pub trait Scalar: 'static + Copy + Clone + PartialEq + std::fmt::Debug {}
impl<T: 'static + Copy + Clone + PartialEq + std::fmt::Debug> Scalar for T {}

#[derive(Eq, PartialEq, Clone, Hash, Debug, Copy)]
pub struct Vector<T: Scalar, const N: usize>(pub [T; N]);

impl<T: Scalar, const N: usize> Vector<T, N> {
	pub fn map<U: Scalar, F: Fn(T) -> U>(&self, f: F) -> Vector<U, N> {
		Vector(self.0.map(f))
	}
}

impl<T: Scalar> From<Vector<T, 1>> for (T,) {
	fn from(value: Vector<T, 1>) -> Self {
		(value.x,)
	}
}

impl<T: Scalar> From<Vector<T, 2>> for (T, T) {
	fn from(value: Vector<T, 2>) -> Self {
		(value.x, value.y)
	}
}

impl<T: Scalar> From<Vector<T, 3>> for (T, T, T) {
	fn from(value: Vector<T, 3>) -> Self {
		(value.x, value.y, value.z)
	}
}

impl<T: Scalar> From<Vector<T, 4>> for (T, T, T, T) {
	fn from(value: Vector<T, 4>) -> Self {
		(value.x, value.y, value.z, value.w)
	}
}

macro_rules! coord_struct(
	($T: ident; $($comps: ident),*) => {
		#[repr(C)]
		#[derive(Eq, PartialEq, Clone, Hash, Debug, Copy)]
		pub struct $T<T: Scalar> {
			$(pub $comps: T),*
		}
	}
);

coord_struct!(X; x);
coord_struct!(XY; x, y);
coord_struct!(XYZ; x, y, z);
coord_struct!(XYZW; x, y, z, w);

macro_rules! impl_deref_for_vec {
	($n:literal,$name:ident) => {
		impl<T: Scalar> Deref for Vector<T, $n> {
			type Target = $name<T>;
		
			fn deref(&self) -> &Self::Target {
				unsafe { &*(self.0.as_ptr() as *const Self::Target) }
			}
		}
	};
}

impl_deref_for_vec!(1, X);
impl_deref_for_vec!(2, XY);
impl_deref_for_vec!(3, XYZ);
impl_deref_for_vec!(4, XYZW);

impl<T: Scalar, const N: usize> From<T> for Vector<T, N> {
	fn from(value: T) -> Self {
		let mut r: [T; N] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
		for i in 0..N { r[i] = value; }
		Vector(r)
	}
}

macro_rules! impl_operator_for_vec {
	($trait_name:ident, $fn_name:ident) => {
		impl<T: Scalar + $trait_name<T>, const N: usize, Rhs> $trait_name<Rhs> for Vector<T, N>
		where
			Vector<T, N>: From<Rhs>,
			<T as $trait_name<T>>::Output: Scalar,
		{
			type Output = Vector<<T as $trait_name<T>>::Output, N>;

			fn $fn_name(self, rhs: Rhs) -> Self::Output {
				let rhs = Vector::<T, N>::from(rhs);
				let mut r: [< T as $trait_name<T>>::Output; N] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
				for i in 0..N {
					r[i] = self.0[i].$fn_name(rhs.0[i])
				}
				Vector(r)
			}
		}
	};
}

macro_rules! impl_unary_operator_for_vec {
	($trait_name:ident, $fn_name:ident) => {
		impl<T: Scalar + $trait_name, const N: usize> $trait_name for Vector<T, N>
		where <T as $trait_name>::Output: Scalar,
		{
			type Output = Vector<<T as $trait_name>::Output, N>;

			fn $fn_name(self) -> Self::Output {
				Vector(self.0.map(|x| x.$fn_name()))
			}
		}
	};
}

impl_operator_for_vec!(Add, add);
impl_operator_for_vec!(Sub, sub);
impl_operator_for_vec!(Mul, mul);
impl_operator_for_vec!(Div, div);
impl_unary_operator_for_vec!(Neg, neg);
impl_unary_operator_for_vec!(Not, not);

pub type Vec1<T> = Vector<T, 1>;
pub type Vec2<T> = Vector<T, 2>;
pub type Vec3<T> = Vector<T, 3>;
pub type Vec4<T> = Vector<T, 4>;

pub fn vec3<T: Scalar>(x: T, y: T, z: T) -> Vec3<T> { Vector([x, y, z]) }

pub type Vec1i32 = Vec1<i32>;
pub type Vec2i32 = Vec2<i32>;
pub type Vec3i32 = Vec3<i32>;
pub type Vec4i32 = Vec4<i32>;
