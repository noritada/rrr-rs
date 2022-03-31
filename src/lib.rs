mod utils;
mod value;
mod walker;

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
    Array { len: usize, element: Box<Field> }, // use Box to avoid E0072
}

enum Size {
    Known(usize),
    Unknown,
    Undefined,
}

fn visit<'f, F, G>(field: &'f Field, start_f: &mut F, end_f: &mut G) -> Result<(), Error>
where
    F: FnMut(&'f Field) -> Result<(), Error>,
    G: FnMut(&'f Field) -> Result<(), Error>,
{
    start_f(field)?;
    match field {
        Field {
            kind: FieldKind::Struct { members },
            name: _,
        } => {
            for member in members.iter() {
                visit(member, start_f, end_f)?;
            }
        }
        Field {
            kind: FieldKind::Array { len, element },
            name: _,
        } => {
            for _ in 0..(*len) {
                visit(element, start_f, end_f)?;
            }
        }
        _ => {}
    }
    end_f(field)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
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
    use crate::value::{Number, Value};
    use crate::walker::Walker;

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
                            element: Box::new(Field {
                                name: "[]".to_owned(),
                                kind: FieldKind::Struct {
                                    members: vec![
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
                            }),
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
                            element: Box::new(Field {
                                name: "[]".to_owned(),
                                kind: FieldKind::Struct {
                                    members: vec![
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
                            }),
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
    fn visitor_basic_functionality() -> Result<(), Box<dyn std::error::Error>> {
        let ast = ast_without_str();

        let mut pos = 0;
        let mut inc_pos = |field: &Field| -> Result<(), Error> {
            match field.size() {
                Size::Known(size) => pos += size,
                Size::Unknown => unimplemented!(),
                Size::Undefined => {}
            };
            Ok(())
        };
        visit(&ast, &mut inc_pos, &mut |_| Ok(()))?;
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
        let mut walker = Walker::new(buf.as_slice());
        let mut vec = Vec::new();
        let mut read = |field: &Field| {
            if !matches!(
                field.kind,
                FieldKind::Struct { .. } | FieldKind::Array { .. }
            ) {
                let value = walker.read(field)?;
                vec.push(value);
            }
            Ok(())
        };
        visit(&ast, &mut read, &mut |_| Ok(()))?;
        assert_eq!(walker.pos(), 63);
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
        let mut walker = Walker::new(buf.as_slice());
        let mut skip = |field: &Field| walker.skip(field);
        visit(&ast, &mut skip, &mut |_| Ok(()))?;
        assert_eq!(walker.pos(), 63);
        Ok(())
    }
}
