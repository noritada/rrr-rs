pub(crate) trait FromBytes {
    fn from_be_bytes(bytes: &[u8]) -> Self;
}

impl<const N: usize> FromBytes for [u8; N] {
    fn from_be_bytes(bytes: &[u8]) -> [u8; N] {
        // panics if N is larger than the slice length
        bytes[..N].try_into().unwrap()
    }
}

macro_rules! add_impl_for_types {
    ($($ty:ty,)*) => ($(
        impl FromBytes for $ty {
            fn from_be_bytes(bytes: &[u8]) -> $ty {
                <$ty>::from_be_bytes(FromBytes::from_be_bytes(bytes))
            }
        }
    )*);
}

add_impl_for_types![i8, i16, i32, u8, u16, u32, f32, f64,];
