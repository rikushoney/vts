use std::num::{NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8};

pub mod arch;
mod database;
mod stringtable;
pub mod yosys;

pub trait OpaqueKey: Copy {
    fn as_index(&self) -> usize;

    fn from_index(idx: usize) -> Self;

    fn max_index() -> usize;

    fn increment(self) -> Self {
        Self::from_index(self.as_index() + 1)
    }

    fn decrement(self) -> Self {
        Self::from_index(self.as_index() - 1)
    }
}

macro_rules! impl_opaquekey {
    ($ty:path, $max:path) => {
        impl OpaqueKey for $ty {
            fn from_index(idx: usize) -> Self {
                assert!(idx < Self::max_index());
                idx as $ty
            }

            fn as_index(&self) -> usize {
                *self as usize
            }

            fn max_index() -> usize {
                $max as usize
            }
        }
    };
}

impl_opaquekey!(u8, u8::MAX);
impl_opaquekey!(u16, u16::MAX);
impl_opaquekey!(u32, u32::MAX);
impl_opaquekey!(u64, u64::MAX);

macro_rules! impl_opaquekey_nonzero {
    ($ty:path, $prim:path, $max:path) => {
        impl OpaqueKey for $ty {
            fn from_index(idx: usize) -> Self {
                let idx = idx + 1;
                assert!(idx < Self::max_index());
                // Safety: we know `idx` is non-zero, because we added 1 to it
                <$ty>::new(idx as $prim).unwrap()
            }

            fn as_index(&self) -> usize {
                (self.get() - 1) as usize
            }

            fn max_index() -> usize {
                $max as usize
            }
        }
    };
}

impl_opaquekey_nonzero!(NonZeroU8, u8, u8::MAX);
impl_opaquekey_nonzero!(NonZeroU16, u16, u16::MAX);
impl_opaquekey_nonzero!(NonZeroU32, u32, u32::MAX);
impl_opaquekey_nonzero!(NonZeroU64, u64, u64::MAX);
