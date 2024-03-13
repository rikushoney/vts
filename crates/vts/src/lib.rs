pub mod arch;
mod database;
mod stringtable;
pub mod yosys;

pub trait OpaqueKey {
    fn as_index(&self) -> usize;

    fn from_index(idx: usize) -> Self;

    fn max_index() -> usize;
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
