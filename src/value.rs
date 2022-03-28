#[derive(Debug, PartialEq)]
pub(crate) enum Value {
    Number(Number),
    String(String),
}

#[derive(Debug, PartialEq)]
pub(crate) enum Number {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    Float32(f32),
    Float64(f64),
}

macro_rules! add_impl_for_types {
    ($(($ty:ty,$variant:ident),)*) => ($(
        impl From<$ty> for Number {
            fn from(n: $ty) -> Number {
                Number::$variant(n)
            }
        }
    )*);
}

add_impl_for_types![
    (i8, Int8),
    (i16, Int16),
    (i32, Int32),
    (u8, UInt8),
    (u16, UInt16),
    (u32, UInt32),
    (f32, Float32),
    (f64, Float64),
];
