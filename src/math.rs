#![allow(dead_code)]

use std::{ops::*, fmt::{Display, Write}};
use num_traits::AsPrimitive;

pub trait Scalar: 'static + Copy + Clone + PartialEq + std::fmt::Debug {}
impl<T: 'static + Copy + Clone + PartialEq + std::fmt::Debug> Scalar for T {}

#[derive(Eq, PartialEq, Clone, Hash, Debug, Copy)]
pub struct Vector<T: Scalar, const N: usize>(pub [T; N]);

impl<T: Scalar, const N: usize> Vector<T, N> {
	pub fn make<F: Fn(usize) -> T>(f: F) -> Vector<T, N> {
		let mut r: [T; N] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
		for i in 0..N { r[i] = f(i); }
		Vector(r)
	}

	pub fn map<U: Scalar, F: Fn(T) -> U>(&self, f: F) -> Vector<U, N> {
		Vector(self.0.map(f))
	}

	pub fn zip_map<
		T2: Scalar,
		V2: Into<Vector<T2, N>>,
		U: Scalar,
		F: Fn(T, T2) -> U
	>(&self, other: V2, f: F) -> Vector<U, N> {
		let other = other.into();
		Vector::<U, N>::make(|i| f(self.0[i], other.0[i]))
	}
}

type DotProductOutput<T> = <<T as Mul>::Output as Add>::Output;

pub trait DotProduct<T: Scalar + Mul, const N: usize>
where
	<T as Mul>::Output: Add<Output = <T as Mul>::Output>,
	<<T as Mul>::Output as Add>::Output: Scalar,
{
	fn dot(&self, other: &Vector<T, N>) -> DotProductOutput<T>;
}

impl<T: Scalar + Mul, const N: usize> DotProduct<T, N> for Vector<T, N>
where
	<T as Mul>::Output: Add<Output = <T as Mul>::Output>,
	<<T as Mul>::Output as Add>::Output: Scalar,
{
	fn dot(
		&self,
		other: &Vector<T, N>
	) -> DotProductOutput<T> {
		let mut r = self.0[0] * other.0[0];
		for i in 1..N {
			r = r + self.0[i] * other.0[i];
		}
		r
	}
}

macro_rules! mag_impl_for_vec {
	($T:ty, $U:ty) => {
		impl<const N: usize> Vector<$T, N> {
			pub fn mag_squared(&self) -> DotProductOutput<$T> { self.dot(self) }
			pub fn mag(&self) -> DotProductOutput<$T> { (self.dot(self) as $U).sqrt() as DotProductOutput<$T> }
		}

		impl<const N: usize> Vector<$T, N>
		where Vector<$T, N>: Div<DotProductOutput<$T>> {
			// TODO: move to separate impl (and make VectorMagnitude a trait?)
			pub fn normalized(self) -> <Vector<DotProductOutput<$T>, N> as Div<DotProductOutput<$T>>>::Output {
				self / self.mag()
			}
		}
	};
}

mag_impl_for_vec!(f32, f32);
mag_impl_for_vec!(f64, f64);
mag_impl_for_vec!(i8, f32);
mag_impl_for_vec!(i16, f32);
mag_impl_for_vec!(i32, f64);
mag_impl_for_vec!(i64, f32);

impl<T: Scalar, const N: usize> Vector<T, N> {
	pub fn lerp<U: num_traits::One + Scalar>(
		self,
		target: Vector<T, N>,
		time: U
	) -> <<Vector<T, N> as Mul<U>>::Output as Add>::Output
	where
		T: Mul<U>,
		Vector<T, N>: Mul<U>,
		<Vector<T, N> as Mul<U>>::Output: Add,
		U: Sub<Output = U>
	{
		self * (U::one() - time) + target * time
	}
}


impl<T: Scalar, const N: usize> Vector<T, N> {
	pub fn each_as<U: Scalar>(&self) -> Vector<U, N>
	where T: AsPrimitive<U>
	{
		self.map(|c| c.as_())
	}
}

impl<T: Scalar + num_traits::Zero, const N: usize> Vector<T, N> {
	pub fn zero() -> Self {
		Self::make(|_| T::zero())
	}
}

pub trait VectorInto<T: Scalar, const N: usize> {
	fn vector<U: Scalar + From<T>>(self) -> Vector<U, N>;
}

macro_rules! ignore_for {
	(($n:tt) $r:tt) => { $r }
}

macro_rules! impl_tuple_conversion_vec {
	($n:literal; $($coords:ident),+; $($indices:tt),+) => {
		impl<T: Scalar> From<Vector<T, $n>> for ($(ignore_for!(($coords) T)),+,) {
			fn from(value: Vector<T, $n>) -> Self {
				($(value.$coords),+,)
			}
		}
		impl<T: Scalar> VectorInto<T, $n> for ($(ignore_for!(($coords) T)),+,) {
			fn vector<U: Scalar + From<T>>(self) -> Vector<U, $n> {
				Vector([$(self.$indices.into()),+,])
			}
		}
	};
}

impl_tuple_conversion_vec!(1; x; 0);
impl_tuple_conversion_vec!(2; x, y; 0, 1);
impl_tuple_conversion_vec!(3; x, y, z; 0, 1, 2);
impl_tuple_conversion_vec!(4; x, y, z, w; 0, 1, 2, 3);

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

macro_rules! impl_swizzle_for_vec {
	($n:literal -> $m:literal : $name:ident => $($fields:ident),+) => {
		impl<T: Scalar> Vector<T, $n> {
			pub fn $name(&self) -> Vector<T, $m> {
				Vector([$(self.$fields),+])
			}
		}
	};
}

macro_rules! impl_swizzles_2d_for_vec {
	(2, $n:literal) => {
		impl_swizzle_for_vec!($n -> 2: xy => x, y);
		impl_swizzle_for_vec!($n -> 2: yx => y, x);
	};
	(3, $n:literal) => {
		impl_swizzles_2d_for_vec!(2, $n);
		impl_swizzle_for_vec!($n -> 2: xz => x, z);
		impl_swizzle_for_vec!($n -> 2: yz => y, z);
		impl_swizzle_for_vec!($n -> 2: zx => z, x);
		impl_swizzle_for_vec!($n -> 2: zy => z, y);
	};
	(4, $n:literal) => {
		impl_swizzles_2d_for_vec!(3, $n);
		impl_swizzle_for_vec!($n -> 2: xw => x, w);
		impl_swizzle_for_vec!($n -> 2: yw => y, w);
		impl_swizzle_for_vec!($n -> 2: zw => z, w);
		impl_swizzle_for_vec!($n -> 2: wx => w, x);
		impl_swizzle_for_vec!($n -> 2: wy => w, y);
		impl_swizzle_for_vec!($n -> 2: wz => w, z);
	};
}

impl_swizzles_2d_for_vec!(2, 2);
impl_swizzles_2d_for_vec!(3, 3);
impl_swizzles_2d_for_vec!(4, 4);

impl<T: Scalar, const N: usize> From<T> for Vector<T, N> {
	fn from(value: T) -> Self {
		Self::make(|_| value)
	}
}

macro_rules! impl_assign_operator_for_vec {
	($trait_name:ident, $fn_name:ident) => {
		impl<T: Scalar + $trait_name<T>, const N: usize, Rhs> $trait_name<Rhs> for Vector<T, N>
		where
			Vector<T, N>: From<Rhs>,
		{
			fn $fn_name(&mut self, rhs: Rhs) {
				let rhs = Vector::<T, N>::from(rhs);
				for i in 0..N {
					self.0[i].$fn_name(rhs.0[i])
				}
			}
		}
	};
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
impl_assign_operator_for_vec!(AddAssign, add_assign);
impl_assign_operator_for_vec!(SubAssign, sub_assign);
impl_assign_operator_for_vec!(MulAssign, mul_assign);
impl_assign_operator_for_vec!(DivAssign, div_assign);
impl_unary_operator_for_vec!(Neg, neg);
impl_unary_operator_for_vec!(Not, not);

pub type Vec1<T> = Vector<T, 1>;
pub type Vec2<T> = Vector<T, 2>;
pub type Vec3<T> = Vector<T, 3>;
pub type Vec4<T> = Vector<T, 4>;

pub const fn vec1<T: Scalar>(x: T,) -> Vec1<T> { Vector([x]) }
pub const fn vec2<T: Scalar>(x: T, y: T) -> Vec2<T> { Vector([x, y]) }
pub const fn vec3<T: Scalar>(x: T, y: T, z: T) -> Vec3<T> { Vector([x, y, z]) }
pub const fn vec4<T: Scalar>(x: T, y: T, z: T, w: T) -> Vec4<T> { Vector([x, y, z, w]) }

pub type Vec1i32 = Vec1<i32>;
pub type Vec2i32 = Vec2<i32>;
pub type Vec3i32 = Vec3<i32>;
pub type Vec4i32 = Vec4<i32>;

pub type Vec1u32 = Vec1<u32>;
pub type Vec2u32 = Vec2<u32>;
pub type Vec3u32 = Vec3<u32>;
pub type Vec4u32 = Vec4<u32>;

pub type Vec1f32 = Vec1<f32>;
pub type Vec2f32 = Vec2<f32>;
pub type Vec3f32 = Vec3<f32>;
pub type Vec4f32 = Vec4<f32>;

impl<T: Scalar + Mul> Vector<T, 3> 
where
	<T as Mul>::Output: Sub + Scalar,
	<<T as Mul>::Output as Sub>::Output: Scalar,
{
	pub fn cross(
		&self,
		other: Vector<T, 3>
	) -> Vector<<<T as Mul>::Output as Sub>::Output, 3> {
		Vector([
			self.y * other.z - self.z * other.y,
			self.z * other.x - self.x * other.z,
			self.x * other.y - self.y * other.x,
		])
	}
}

impl<T: Scalar + Display, const N: usize> std::fmt::Display for Vector<T, N> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_fmt(format_args!("vec{}(", N))?;
		for (i, member) in self.0.iter().enumerate() {
			if let Some(precision) = f.precision() {
				f.write_fmt(format_args!("{1:.*}", precision, member))?;
			} else {
				f.write_fmt(format_args!("{:.2}", member))?;
			}

			if i != N - 1 {
				f.write_str(", ")?;
			}
		}

		f.write_char(')')
	}
}
