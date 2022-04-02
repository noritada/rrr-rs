pub(crate) struct Ast {
    pub(crate) kind: AstKind,
    pub(crate) name: String,
}

impl Ast {
    pub(crate) fn size(&self) -> Size {
        match self.kind {
            AstKind::Int8 => Size::Known(std::mem::size_of::<i8>()),
            AstKind::Int16 => Size::Known(std::mem::size_of::<i16>()),
            AstKind::Int32 => Size::Known(std::mem::size_of::<i32>()),
            AstKind::UInt8 => Size::Known(std::mem::size_of::<u8>()),
            AstKind::UInt16 => Size::Known(std::mem::size_of::<u16>()),
            AstKind::UInt32 => Size::Known(std::mem::size_of::<u32>()),
            AstKind::Float32 => Size::Known(std::mem::size_of::<f32>()),
            AstKind::Float64 => Size::Known(std::mem::size_of::<f64>()),
            AstKind::Str => Size::Unknown,
            AstKind::NStr(size) => Size::Known(size),
            AstKind::Struct { .. } => Size::Undefined,
            AstKind::Array { .. } => Size::Undefined,
        }
    }
}

pub(crate) enum AstKind {
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
    Struct { members: Vec<Ast> },
    Array { len: usize, element: Box<Ast> }, // use Box to avoid E0072
}

pub(crate) enum Size {
    Known(usize),
    Unknown,
    Undefined,
}
