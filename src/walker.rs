use crate::{
    ast::{Ast, AstKind, Size},
    utils::FromBytes,
    value::Value,
    Error,
};

pub struct BufWalker<'w> {
    buf: &'w [u8],
    pos: usize,
}

impl<'w> BufWalker<'w> {
    pub fn new(buf: &'w [u8]) -> Self {
        BufWalker { buf, pos: 0 }
    }

    pub(crate) fn pos(&mut self) -> usize {
        self.pos
    }
    pub(crate) fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub(crate) fn read(&mut self, node: &Ast) -> Result<Value, Error> {
        let value = match node.kind {
            AstKind::Int8 => Value::Number(self.read_number::<i8>()?.into()),
            AstKind::Int16 => Value::Number(self.read_number::<i16>()?.into()),
            AstKind::Int32 => Value::Number(self.read_number::<i32>()?.into()),
            AstKind::UInt8 => Value::Number(self.read_number::<u8>()?.into()),
            AstKind::UInt16 => Value::Number(self.read_number::<u16>()?.into()),
            AstKind::UInt32 => Value::Number(self.read_number::<u32>()?.into()),
            AstKind::Float32 => Value::Number(self.read_number::<f32>()?.into()),
            AstKind::Float64 => Value::Number(self.read_number::<f64>()?.into()),
            // assuming that strings are utf8-encoded
            AstKind::Str => Value::String(String::from_utf8_lossy(self.read_str()?).to_string()),
            AstKind::NStr(size) => {
                Value::String(String::from_utf8_lossy(self.read_nstr(size)?).to_string())
            }
            AstKind::Struct { .. } => Value::new_struct(),
            AstKind::Array { .. } => Value::new_array(),
        };
        Ok(value)
    }

    pub(crate) fn read_number<N>(&mut self) -> Result<N, Error>
    where
        N: FromBytes,
    {
        let start = self.pos;
        self.pos += std::mem::size_of::<N>();
        if self.pos > (self.buf).len() {
            return Err(Error::General);
        }
        let val = FromBytes::from_be_bytes(&self.buf[start..self.pos]);
        Ok(val)
    }

    pub(crate) fn read_str(&mut self) -> Result<&[u8], Error> {
        let start = self.pos;
        self.skip_str()?;
        let string = &self.buf[start..(self.pos - 1)]; // remove trailing b'\0'
        Ok(string)
    }

    pub(crate) fn read_nstr(&mut self, size: usize) -> Result<&[u8], Error> {
        let start = self.pos;
        self.pos += size;
        let string = &self.buf[start..self.pos];
        Ok(string)
    }

    pub(crate) fn skip(&mut self, node: &Ast) -> Result<(), Error> {
        match node.size() {
            Size::Known(size) => {
                self.pos += size;
                Ok(())
            }
            Size::Unknown => self.skip_str(),
            Size::Undefined => Ok(()),
        }
    }

    pub(crate) fn skip_str(&mut self) -> Result<(), Error> {
        for b in &self.buf[self.pos..] {
            self.pos += 1;
            if *b == b'\0' {
                return Ok(());
            }
        }
        Err(Error::General)
    }

    pub(crate) fn reached_end(&self) -> bool {
        self.pos == self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_reading_number {
        ($(($name:ident, $buf:expr, $ty:ident, $expected:expr),)*) => ($(
            #[test]
            fn $name() -> Result<(), Box<dyn std::error::Error>> {
                let buf = $buf;
                let mut walker = BufWalker::new(buf.as_slice());
                walker.set_pos(2);
                let result = walker.read_number::<$ty>()?;
                assert_eq!(result, $expected);
                Ok(())
            }
        )*);
    }

    test_reading_number! {
        (
            reading_i8,
            vec![0x00, 0x00, 0xfe, 0x00, 0x00],
            i8,
            -2
        ),
        (
            reading_i16,
            vec![0x00, 0x00, 0xfe, 0xdc, 0x00, 0x00],
            i16,
            -292
        ),
        (
            reading_i32,
            vec![0x00, 0x00, 0xfe, 0xdc, 0xba, 0x98, 0x00],
            i32,
            -19088744
        ),
        (
            reading_u8,
            vec![0x00, 0x00, 0xfe, 0x00, 0x00],
            u8,
            254
        ),
        (
            reading_u16,
            vec![0x00, 0x00, 0xfe, 0xdc, 0x00, 0x00],
            u16,
            65244
        ),
        (
            reading_u32,
            vec![0x00, 0x00, 0xfe, 0xdc, 0xba, 0x98, 0x00, 0x00],
            u32,
            4275878552
        ),
        (
            reading_f32,
            vec![0x00, 0x00, 0xbf, 0x80, 0x00, 0x00, 0x00, 0x00],
            f32,
            -1.0
        ),
        (
            reading_f64,
            vec![
                0x00, 0x00, 0xbf, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            f64,
            -1.0
        ),
    }

    #[test]
    fn read_str() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0x54, 0x4f, 0x4b, 0x59, 0x4f, 0x00, 0x00, 0x00];
        let mut walker = BufWalker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_str()?;
        assert_eq!(result, "TOKYO".as_bytes());
        Ok(())
    }

    #[test]
    fn read_nstr() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0x54, 0x4f, 0x4b, 0x00, 0x00, 0x00];
        let mut walker = BufWalker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_nstr(4)?;
        assert_eq!(result, "TOK\x00".as_bytes());
        Ok(())
    }
}
