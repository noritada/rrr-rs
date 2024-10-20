/// [`DataReaderOptions`] is a type representing the various flags of
/// [`DataReader`](super::DataReader) and options as the union of those flags.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct DataReaderOptions(u32);

impl DataReaderOptions {
    /// Flag to enable data body reading.
    pub const ENABLE_READING_BODY: Self = Self(1 << 1);
    /// Flag to ignore the value of `data_size` header field.
    pub const IGNORE_DATA_SIZE_FIELD: Self = Self(1 << 2);
    /// Flag to allow a trailing comma in the `format` header field.
    pub const ALLOW_TRAILING_COMMA: Self = Self(1 << 3);

    /// Returns the union of `self` and a `flag`.
    pub fn union(&self, flag: Self) -> Self {
        let Self(self_) = self;
        let Self(flag) = flag;
        Self(*self_ | flag)
    }

    /// Returns whether a particular `flag` is set or not.
    pub fn contains(&self, flag: Self) -> bool {
        let Self(self_) = self;
        let Self(flag) = flag;
        self_ & flag != 0
    }
}

impl std::ops::BitOr for DataReaderOptions {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let inner = self.0 | rhs.0;
        Self(inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_default_is_zero() {
        let actual = DataReaderOptions::default();
        let expected = DataReaderOptions(0);
        assert_eq!(actual, expected);
    }

    macro_rules! test_options_union {
        ($((
            $name:ident,
            $current:expr,
            $another:expr,
            $expected:expr
        ),)*) => ($(
            #[test]
            fn $name() {
                let current = DataReaderOptions($current);
                let another = DataReaderOptions($another);
                let actual = current.union(another);
                let expected = DataReaderOptions($expected);
                assert_eq!(actual, expected);
            }
        )*);
    }

    test_options_union! {
        (options_union_zero_and_non_zero, 0b00, 0b10, 0b10),
        (options_union_non_zero_and_zero, 0b10, 0b00, 0b10),
        (options_union_non_zero_and_non_zero, 0b10, 0b01, 0b11),
        (options_union_the_same, 0b10, 0b10, 0b10),
    }

    macro_rules! test_options_contains {
        ($((
            $name:ident,
            $options:expr,
            $option:expr,
            $expected:expr
        ),)*) => ($(
            #[test]
            fn $name() {
                let options = DataReaderOptions($options);
                let option = DataReaderOptions($option);
                let actual = options.contains(option);
                assert_eq!(actual, $expected);
            }
        )*);
    }

    test_options_contains! {
        (options_non_zero_contains_non_zero, 0b11, 0b10, true),
        (options_non_zero_does_not_contain_non_zero, 0b01, 0b10, false),
        (options_non_zero_does_not_contain_zero, 0b01, 0b00, false),
        (options_zero_does_not_contain_non_zero, 0b00, 0b10, false),
        (options_zero_does_not_contain_zero, 0b00, 0b00, false),
    }
}
