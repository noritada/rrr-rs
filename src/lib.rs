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

    fn skip(&self, buf: &[u8], pos: &mut usize) {
        match self.size() {
            Size::Known(size) => *pos += size,
            Size::Unknown => {
                for b in &buf[*pos..] {
                    *pos += 1;
                    if *b == b'\0' {
                        break;
                    }
                }
            }
            Size::Undefined => {} // not expected
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
    Array { len: usize, element: Vec<Field> },
}

enum Size {
    Known(usize),
    Unknown,
    Undefined,
}

fn visit<F>(field: &Field, f: &mut F)
where
    F: FnMut(&Field) -> (),
{
    match field {
        Field {
            kind: FieldKind::Struct { members },
            name: _,
        } => {
            for member in members.iter() {
                visit(member, f);
            }
        }
        Field {
            kind: FieldKind::Array { len, element },
            name: _,
        } => {
            for _ in 0..(*len) {
                for member in element.iter() {
                    visit(member, f);
                }
            }
        }
        _ => f(field),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn visitor_basic_functionality() {
        let ast = ast_without_str();

        let mut pos = 0;
        let mut inc_pos = |field: &Field| match field.size() {
            Size::Known(size) => pos += size,
            Size::Unknown => unimplemented!(),
            Size::Undefined => unreachable!(),
        };
        visit(&ast, &mut inc_pos);
        assert_eq!(pos, 52)
    }

    #[test]
    fn visitor_skip() {
        let ast = ast_with_str();

        let buf = vec![
            0x07, 0xe6, 0x01, 0x01, 0x54, 0x4f, 0x4b, 0x59, 0x4f, 0x00, 0x00, 0x64, 0x00, 0x0a,
            0x4f, 0x53, 0x41, 0x4b, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x4e, 0x41, 0x47, 0x4f,
            0x59, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x46, 0x55, 0x4b, 0x55, 0x4f, 0x4b, 0x41,
            0x00, 0x00, 0x64, 0x00, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut pos = 0;
        let mut skip = |field: &Field| field.skip(&buf, &mut pos);
        visit(&ast, &mut skip);
        assert_eq!(pos, 63)
    }
}
