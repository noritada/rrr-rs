mod utils;
mod value;
use crate::utils::FromBytes;
use crate::value::Value;

struct Field {
    kind: FieldKind,
    name: String,
}

impl Field {
    fn size(&self) -> Size {
        match self.kind {
            FieldKind::Int8 => Size::Known(std::mem::size_of::<i8>()),
            FieldKind::Int16 => Size::Known(std::mem::size_of::<i16>()),
            FieldKind::Int32 => Size::Known(std::mem::size_of::<i32>()),
            FieldKind::UInt8 => Size::Known(std::mem::size_of::<u8>()),
            FieldKind::UInt16 => Size::Known(std::mem::size_of::<u16>()),
            FieldKind::UInt32 => Size::Known(std::mem::size_of::<u32>()),
            FieldKind::Float32 => Size::Known(std::mem::size_of::<f32>()),
            FieldKind::Float64 => Size::Known(std::mem::size_of::<f64>()),
            FieldKind::Str => Size::Unknown,
            FieldKind::NStr(size) => Size::Known(size),
            FieldKind::Struct { .. } => Size::Undefined,
            FieldKind::Array { .. } => Size::Undefined,
        }
    }

    fn read(&self, buf: &[u8], pos: &mut usize) -> Result<Value, Error> {
        let value = match self.kind {
            FieldKind::Int8 => Value::Number(self.read_number::<i8>(buf, pos)?.into()),
            FieldKind::Int16 => Value::Number(self.read_number::<i16>(buf, pos)?.into()),
            FieldKind::Int32 => Value::Number(self.read_number::<i32>(buf, pos)?.into()),
            FieldKind::UInt8 => Value::Number(self.read_number::<u8>(buf, pos)?.into()),
            FieldKind::UInt16 => Value::Number(self.read_number::<u16>(buf, pos)?.into()),
            FieldKind::UInt32 => Value::Number(self.read_number::<u32>(buf, pos)?.into()),
            FieldKind::Float32 => Value::Number(self.read_number::<f32>(buf, pos)?.into()),
            FieldKind::Float64 => Value::Number(self.read_number::<f64>(buf, pos)?.into()),
            // assuming that strings are utf8-encoded
            FieldKind::Str => {
                Value::String(String::from_utf8_lossy(self.read_str(buf, pos)?).to_string())
            }
            FieldKind::NStr(size) => {
                Value::String(String::from_utf8_lossy(self.read_nstr(buf, pos, size)?).to_string())
            }
            FieldKind::Struct { .. } => unimplemented!(),
            FieldKind::Array { .. } => unimplemented!(),
        };
        Ok(value)
    }

    fn read_number<N>(&self, buf: &[u8], pos: &mut usize) -> Result<N, Error>
    where
        N: FromBytes,
    {
        let start = *pos;
        *pos += std::mem::size_of::<N>();
        if *pos > (*buf).len() {
            return Err(Error);
        }
        let val = FromBytes::from_be_bytes(&buf[start..*pos]);
        Ok(val)
    }

    fn read_str<'a>(&self, buf: &'a [u8], pos: &mut usize) -> Result<&'a [u8], Error> {
        let start = *pos;
        self.skip_str(buf, pos)?;
        let string = &buf[start..(*pos - 1)]; // remove trailing b'\0'
        Ok(string)
    }

    fn read_nstr<'a>(
        &self,
        buf: &'a [u8],
        pos: &mut usize,
        size: usize,
    ) -> Result<&'a [u8], Error> {
        let start = *pos;
        *pos += size;
        let string = &buf[start..*pos];
        Ok(string)
    }

    fn skip(&self, buf: &[u8], pos: &mut usize) -> Result<(), Error> {
        match self.size() {
            Size::Known(size) => {
                *pos += size;
                Ok(())
            }
            Size::Unknown => self.skip_str(buf, pos),
            Size::Undefined => unimplemented!(), // not expected
        }
    }

    fn skip_str(&self, buf: &[u8], pos: &mut usize) -> Result<(), Error> {
        for b in &buf[*pos..] {
            *pos += 1;
            if *b == b'\0' {
                return Ok(());
            }
        }
        Err(Error)
    }
}

enum FieldKind {
    Int8,
    Int16,
    Int32,
    UInt8,
    UInt16,
    UInt32,
    Float32,
    Float64,
    Str,
    NStr(usize),
    Struct { members: Vec<Field> },
    Array { len: usize, element: Vec<Field> },
}

enum Size {
    Known(usize),
    Unknown,
    Undefined,
}

fn visit<F>(field: &Field, f: &mut F) -> Result<(), Error>
where
    F: FnMut(&Field) -> Result<(), Error>,
{
    match field {
        Field {
            kind: FieldKind::Struct { members },
            name: _,
        } => {
            for member in members.iter() {
                visit(member, f)?;
            }
        }
        Field {
            kind: FieldKind::Array { len, element },
            name: _,
        } => {
            for _ in 0..(*len) {
                for member in element.iter() {
                    visit(member, f)?;
                }
            }
        }
        _ => f(field)?,
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct Error;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "error in processing data")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Number;

    fn ast_without_str() -> Field {
        Field {
            name: "".to_owned(),
            kind: FieldKind::Struct {
                members: vec![
                    Field {
                        name: "date".to_owned(),
                        kind: FieldKind::Struct {
                            members: vec![
                                Field {
                                    name: "year".to_owned(),
                                    kind: FieldKind::UInt16,
                                },
                                Field {
                                    name: "month".to_owned(),
                                    kind: FieldKind::UInt8,
                                },
                                Field {
                                    name: "day".to_owned(),
                                    kind: FieldKind::UInt8,
                                },
                            ],
                        },
                    },
                    Field {
                        name: "data".to_owned(),
                        kind: FieldKind::Array {
                            len: 4,
                            element: vec![
                                Field {
                                    name: "loc".to_owned(),
                                    kind: FieldKind::NStr(4),
                                },
                                Field {
                                    name: "temp".to_owned(),
                                    kind: FieldKind::Int16,
                                },
                                Field {
                                    name: "rhum".to_owned(),
                                    kind: FieldKind::UInt16,
                                },
                            ],
                        },
                    },
                    Field {
                        name: "comment".to_owned(),
                        kind: FieldKind::NStr(16),
                    },
                ],
            },
        }
    }

    fn ast_with_str() -> Field {
        Field {
            name: "".to_owned(),
            kind: FieldKind::Struct {
                members: vec![
                    Field {
                        name: "date".to_owned(),
                        kind: FieldKind::Struct {
                            members: vec![
                                Field {
                                    name: "year".to_owned(),
                                    kind: FieldKind::UInt16,
                                },
                                Field {
                                    name: "month".to_owned(),
                                    kind: FieldKind::UInt8,
                                },
                                Field {
                                    name: "day".to_owned(),
                                    kind: FieldKind::UInt8,
                                },
                            ],
                        },
                    },
                    Field {
                        name: "data".to_owned(),
                        kind: FieldKind::Array {
                            len: 4,
                            element: vec![
                                Field {
                                    name: "loc".to_owned(),
                                    kind: FieldKind::Str,
                                },
                                Field {
                                    name: "temp".to_owned(),
                                    kind: FieldKind::Int16,
                                },
                                Field {
                                    name: "rhum".to_owned(),
                                    kind: FieldKind::UInt16,
                                },
                            ],
                        },
                    },
                    Field {
                        name: "comment".to_owned(),
                        kind: FieldKind::NStr(16),
                    },
                ],
            },
        }
    }

    #[test]
    fn field_read_i8() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Field {
            name: "dummy".to_owned(),
            kind: FieldKind::Int8,
        };

        let buf = vec![0x00, 0x00, 0xfe, 0x00, 0x00];
        let mut pos = 2;
        let result = ast.read_number::<i8>(&buf, &mut pos)?;
        assert_eq!(result, -2);
        Ok(())
    }

    #[test]
    fn field_read_i16() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Field {
            name: "dummy".to_owned(),
            kind: FieldKind::Int16,
        };

        let buf = vec![0x00, 0x00, 0xfe, 0xdc, 0x00, 0x00];
        let mut pos = 2;
        let result = ast.read_number::<i16>(&buf, &mut pos)?;
        assert_eq!(result, -292);
        Ok(())
    }

    #[test]
    fn field_read_i32() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Field {
            name: "dummy".to_owned(),
            kind: FieldKind::Int32,
        };

        let buf = vec![0x00, 0x00, 0xfe, 0xdc, 0xba, 0x98, 0x00];
        let mut pos = 2;
        let result = ast.read_number::<i32>(&buf, &mut pos)?;
        assert_eq!(result, -19088744);
        Ok(())
    }

    #[test]
    fn field_read_u8() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Field {
            name: "dummy".to_owned(),
            kind: FieldKind::UInt8,
        };

        let buf = vec![0x00, 0x00, 0xfe, 0x00, 0x00];
        let mut pos = 2;
        let result = ast.read_number::<u8>(&buf, &mut pos)?;
        assert_eq!(result, 254);
        Ok(())
    }

    #[test]
    fn field_read_u16() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Field {
            name: "dummy".to_owned(),
            kind: FieldKind::UInt16,
        };

        let buf = vec![0x00, 0x00, 0xfe, 0xdc, 0x00, 0x00];
        let mut pos = 2;
        let result = ast.read_number::<u16>(&buf, &mut pos)?;
        assert_eq!(result, 65244);
        Ok(())
    }

    #[test]
    fn field_read_u32() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Field {
            name: "dummy".to_owned(),
            kind: FieldKind::UInt32,
        };

        let buf = vec![0x00, 0x00, 0xfe, 0xdc, 0xba, 0x98, 0x00, 0x00];
        let mut pos = 2;
        let result = ast.read_number::<u32>(&buf, &mut pos)?;
        assert_eq!(result, 4275878552);
        Ok(())
    }

    #[test]
    fn field_read_f32() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Field {
            name: "dummy".to_owned(),
            kind: FieldKind::Float32,
        };

        let buf = vec![0x00, 0x00, 0xbf, 0x80, 0x00, 0x00, 0x00, 0x00];
        let mut pos = 2;
        let result = ast.read_number::<f32>(&buf, &mut pos)?;
        assert_eq!(result, -1.0);
        Ok(())
    }

    #[test]
    fn field_read_f64() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Field {
            name: "dummy".to_owned(),
            kind: FieldKind::Float32,
        };

        let buf = vec![
            0x00, 0x00, 0xbf, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut pos = 2;
        let result = ast.read_number::<f64>(&buf, &mut pos)?;
        assert_eq!(result, -1.0);
        Ok(())
    }

    #[test]
    fn field_read_str() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Field {
            name: "s".to_owned(),
            kind: FieldKind::Str,
        };

        let buf = vec![0x00, 0x00, 0x54, 0x4f, 0x4b, 0x59, 0x4f, 0x00, 0x00, 0x00];
        let mut pos = 2;
        let result = ast.read_str(&buf, &mut pos)?;
        assert_eq!(result, "TOKYO".as_bytes());
        Ok(())
    }

    #[test]
    fn field_read_nstr() -> Result<(), Box<dyn std::error::Error>> {
        let ast = Field {
            name: "s".to_owned(),
            kind: FieldKind::NStr(4),
        };

        let buf = vec![0x00, 0x00, 0x54, 0x4f, 0x4b, 0x00, 0x00, 0x00];
        let mut pos = 2;
        let result = ast.read_nstr(&buf, &mut pos, 4)?;
        assert_eq!(result, "TOK\x00".as_bytes());
        Ok(())
    }

    #[test]
    fn visitor_basic_functionality() -> Result<(), Box<dyn std::error::Error>> {
        let ast = ast_without_str();

        let mut pos = 0;
        let mut inc_pos = |field: &Field| -> Result<(), Error> {
            match field.size() {
                Size::Known(size) => pos += size,
                Size::Unknown => unimplemented!(),
                Size::Undefined => unreachable!(),
            };
            Ok(())
        };
        visit(&ast, &mut inc_pos)?;
        assert_eq!(pos, 52);
        Ok(())
    }

    #[test]
    fn visitor_read() -> Result<(), Box<dyn std::error::Error>> {
        let ast = ast_with_str();

        let buf = vec![
            0x07, 0xe6, 0x01, 0x01, 0x54, 0x4f, 0x4b, 0x59, 0x4f, 0x00, 0x00, 0x64, 0x00, 0x0a,
            0x4f, 0x53, 0x41, 0x4b, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x4e, 0x41, 0x47, 0x4f,
            0x59, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x46, 0x55, 0x4b, 0x55, 0x4f, 0x4b, 0x41,
            0x00, 0x00, 0x64, 0x00, 0x0a, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
        ];
        let mut pos = 0;
        let mut vec = Vec::new();
        let mut read = |field: &Field| {
            let value = field.read(&buf, &mut pos)?;
            vec.push(value);
            Ok(())
        };
        visit(&ast, &mut read)?;
        assert_eq!(pos, 63);
        assert_eq!(
            vec,
            vec![
                Value::Number(Number::UInt16(2022)),
                Value::Number(Number::UInt8(1)),
                Value::Number(Number::UInt8(1)),
                Value::String("TOKYO".to_owned()),
                Value::Number(Number::Int16(100)),
                Value::Number(Number::UInt16(10)),
                Value::String("OSAKA".to_owned()),
                Value::Number(Number::Int16(100)),
                Value::Number(Number::UInt16(10)),
                Value::String("NAGOYA".to_owned()),
                Value::Number(Number::Int16(100)),
                Value::Number(Number::UInt16(10)),
                Value::String("FUKUOKA".to_owned()),
                Value::Number(Number::Int16(100)),
                Value::Number(Number::UInt16(10)),
                Value::String("0123456789abcdef".to_owned()),
            ]
        );
        Ok(())
    }

    #[test]
    fn visitor_skip() -> Result<(), Box<dyn std::error::Error>> {
        let ast = ast_with_str();

        let buf = vec![
            0x07, 0xe6, 0x01, 0x01, 0x54, 0x4f, 0x4b, 0x59, 0x4f, 0x00, 0x00, 0x64, 0x00, 0x0a,
            0x4f, 0x53, 0x41, 0x4b, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x4e, 0x41, 0x47, 0x4f,
            0x59, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x46, 0x55, 0x4b, 0x55, 0x4f, 0x4b, 0x41,
            0x00, 0x00, 0x64, 0x00, 0x0a, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
        ];
        let mut pos = 0;
        let mut skip = |field: &Field| field.skip(&buf, &mut pos);
        visit(&ast, &mut skip)?;
        assert_eq!(pos, 63);
        Ok(())
    }
}
