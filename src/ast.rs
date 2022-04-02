pub(crate) struct Field {
    pub(crate) kind: FieldKind,
    pub(crate) name: String,
}

impl Field {
    pub(crate) fn size(&self) -> Size {
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

pub(crate) enum FieldKind {
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

pub(crate) enum Size {
    Known(usize),
    Unknown,
    Undefined,
}
