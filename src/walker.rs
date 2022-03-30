use crate::utils::FromBytes;
use crate::value::Value;
use crate::{Error, Field, FieldKind, Size};

pub(crate) struct Walker<'w> {
    buf: &'w [u8],
    pos: usize,
}

impl<'w> Walker<'w> {
    pub(crate) fn new(buf: &'w [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub(crate) fn pos(&mut self) -> usize {
        self.pos
    }
    pub(crate) fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub(crate) fn read(&mut self, field: &Field) -> Result<Value, Error> {
        let value = match field.kind {
            FieldKind::Int8 => Value::Number(self.read_number::<i8>()?.into()),
            FieldKind::Int16 => Value::Number(self.read_number::<i16>()?.into()),
            FieldKind::Int32 => Value::Number(self.read_number::<i32>()?.into()),
            FieldKind::UInt8 => Value::Number(self.read_number::<u8>()?.into()),
            FieldKind::UInt16 => Value::Number(self.read_number::<u16>()?.into()),
            FieldKind::UInt32 => Value::Number(self.read_number::<u32>()?.into()),
            FieldKind::Float32 => Value::Number(self.read_number::<f32>()?.into()),
            FieldKind::Float64 => Value::Number(self.read_number::<f64>()?.into()),
            // assuming that strings are utf8-encoded
            FieldKind::Str => Value::String(String::from_utf8_lossy(self.read_str()?).to_string()),
            FieldKind::NStr(size) => {
                Value::String(String::from_utf8_lossy(self.read_nstr(size)?).to_string())
            }
            FieldKind::Struct { .. } => unimplemented!(),
            FieldKind::Array { .. } => unimplemented!(),
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
            return Err(Error);
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

    pub(crate) fn skip(&mut self, field: &Field) -> Result<(), Error> {
        match field.size() {
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
        Err(Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_i8() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0xfe, 0x00, 0x00];
        let mut walker = Walker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_number::<i8>()?;
        assert_eq!(result, -2);
        Ok(())
    }

    #[test]
    fn read_i16() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0xfe, 0xdc, 0x00, 0x00];
        let mut walker = Walker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_number::<i16>()?;
        assert_eq!(result, -292);
        Ok(())
    }

    #[test]
    fn read_i32() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0xfe, 0xdc, 0xba, 0x98, 0x00];
        let mut walker = Walker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_number::<i32>()?;
        assert_eq!(result, -19088744);
        Ok(())
    }

    #[test]
    fn read_u8() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0xfe, 0x00, 0x00];
        let mut walker = Walker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_number::<u8>()?;
        assert_eq!(result, 254);
        Ok(())
    }

    #[test]
    fn read_u16() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0xfe, 0xdc, 0x00, 0x00];
        let mut walker = Walker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_number::<u16>()?;
        assert_eq!(result, 65244);
        Ok(())
    }

    #[test]
    fn read_u32() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0xfe, 0xdc, 0xba, 0x98, 0x00, 0x00];
        let mut walker = Walker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_number::<u32>()?;
        assert_eq!(result, 4275878552);
        Ok(())
    }

    #[test]
    fn read_f32() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0xbf, 0x80, 0x00, 0x00, 0x00, 0x00];
        let mut walker = Walker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_number::<f32>()?;
        assert_eq!(result, -1.0);
        Ok(())
    }

    #[test]
    fn read_f64() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![
            0x00, 0x00, 0xbf, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut walker = Walker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_number::<f64>()?;
        assert_eq!(result, -1.0);
        Ok(())
    }

    #[test]
    fn read_str() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0x54, 0x4f, 0x4b, 0x59, 0x4f, 0x00, 0x00, 0x00];
        let mut walker = Walker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_str()?;
        assert_eq!(result, "TOKYO".as_bytes());
        Ok(())
    }

    #[test]
    fn read_nstr() -> Result<(), Box<dyn std::error::Error>> {
        let buf = vec![0x00, 0x00, 0x54, 0x4f, 0x4b, 0x00, 0x00, 0x00];
        let mut walker = Walker::new(buf.as_slice());
        walker.set_pos(2);
        let result = walker.read_nstr(4)?;
        assert_eq!(result, "TOK\x00".as_bytes());
        Ok(())
    }
}
