use std::borrow::Cow;

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

pub fn json_escape_str(input: &str) -> Cow<str> {
    for (i, byte) in input.as_bytes().iter().enumerate() {
        if json_escape_byte(byte).is_some() {
            // assuming that 1 byte would be converted to 2 bytes
            let mut escaped_string = String::with_capacity(input.len() * 2);
            escaped_string.push_str(&input[..i]);
            for byte in input[i..].as_bytes().iter() {
                match json_escape_byte(byte) {
                    Some(b'u') => escaped_string.push_str(&format!("\\u{:04X}", byte)),
                    Some(b) => {
                        escaped_string.push('\\');
                        escaped_string.push(b as char);
                    }
                    None => escaped_string.push(*byte as char),
                }
            }
            return Cow::Owned(escaped_string);
        }
    }

    Cow::Borrowed(input)
}

fn json_escape_byte(input: &u8) -> Option<u8> {
    // see https://datatracker.ietf.org/doc/html/rfc8259
    match *input {
        0x08 => Some(b'b'),
        0x09 => Some(b't'),
        0x0a => Some(b'n'),
        0x0c => Some(b'f'),
        0x0d => Some(b'r'),
        0x00..=0x1f | 0x7f => Some(b'u'), // should be '\uXXXX'
        0x22 => Some(b'"'),
        0x5c => Some(b'\\'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_json_escape {
        ($(($name:ident, $input_start:expr, $input_end:expr, $expected:expr),)*) => ($(
            #[test]
            fn $name() {
                let input: String = ($input_start..$input_end).map(|b| b as char).collect();
                let actual = json_escape_str(input.as_str());
                let expected = $expected
                    .iter()
                    .map(|s| s.to_owned())
                    .collect::<Vec<_>>()
                    .join("");
                assert_eq!(actual, expected);
            }
        )*);
    }

    test_json_escape! {
        (
            json_escape_for_all_ascii_characters,
            0x00u8,
            0x80u8,
            vec![
                r##"\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t\n\u000B\f\r\u000E\u000F"##,
                r##"\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001A\u001B\u001C\u001D\u001E\u001F"##,
                r##" !\"#$%&'()*+,-./0123456789:;<=>?"##,
                r##"@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_"##,
                r##"`abcdefghijklmnopqrstuvwxyz{|}~\u007F"##,
            ]
        ),
        (
            json_escape_with_no_escapes,
            0x61u8,
            0x7bu8,
            vec![r##"abcdefghijklmnopqrstuvwxyz"##,]
        ),
        (
            json_escape_for_string_to_be_escaped_from_the_middle,
            0x41u8,
            0x5eu8,
            vec![r##"ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]"##,]
        ),
    }
}
