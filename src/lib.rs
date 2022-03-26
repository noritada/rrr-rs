struct Field {
    kind: FieldKind,
    name: String,
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

    #[test]
    fn visitor() {
        let ast = Field {
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
        };

        let mut pos = 0;
        let mut inc_pos = |field: &Field| match field {
            Field {
                kind: FieldKind::Int8,
                ..
            } => pos += std::mem::size_of::<i8>(),
            Field {
                kind: FieldKind::Int16,
                ..
            } => pos += std::mem::size_of::<i16>(),
            Field {
                kind: FieldKind::Int32,
                ..
            } => pos += std::mem::size_of::<i32>(),
            Field {
                kind: FieldKind::UInt8,
                ..
            } => pos += std::mem::size_of::<u8>(),
            Field {
                kind: FieldKind::UInt16,
                ..
            } => pos += std::mem::size_of::<u16>(),
            Field {
                kind: FieldKind::UInt32,
                ..
            } => pos += std::mem::size_of::<u32>(),
            Field {
                kind: FieldKind::Float32,
                ..
            } => pos += std::mem::size_of::<f32>(),
            Field {
                kind: FieldKind::Float64,
                ..
            } => pos += std::mem::size_of::<f64>(),
            Field {
                kind: FieldKind::NStr(size),
                ..
            } => pos += size,
            Field {
                kind: FieldKind::Str,
                ..
            } => {
                unimplemented!();
            }
            Field {
                kind: FieldKind::Struct { .. },
                ..
            } => {
                unreachable!();
            }
            Field {
                kind: FieldKind::Array { .. },
                ..
            } => {
                unreachable!();
            }
        };
        visit(&ast, &mut inc_pos);
        assert_eq!(pos, 52)
    }
}
