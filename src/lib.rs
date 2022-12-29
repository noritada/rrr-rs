mod ast;
mod param;
mod reader;
mod utils;
mod value;
mod visitor;
mod walker;

use std::borrow::Cow;

pub use crate::{
    ast::{Ast, AstKind, Len, Location, Schema, SchemaParseError, SchemaParseErrorKind},
    reader::{DataReader, DataReaderOptions},
    utils::json_escape_str,
    visitor::{AstVisitor, JsonDisplay, SchemaOnelineDisplay},
};

fn visit<'f, F, G>(node: &'f Ast, start_f: &mut F, end_f: &mut G) -> Result<(), Error>
where
    F: FnMut(&'f Ast) -> Result<(), Error>,
    G: FnMut(&'f Ast) -> Result<(), Error>,
{
    start_f(node)?;
    match node {
        Ast {
            kind: AstKind::Struct(members),
            name: _,
        } => {
            for member in members.iter() {
                visit(member, start_f, end_f)?;
            }
        }
        Ast {
            kind: AstKind::Array(len, element),
            name: _,
        } => {
            let len = match len {
                Len::Fixed(n) => n,
                Len::Unlimited => panic!("error: unlimited length array is not supported"),
                Len::Variable(_) => panic!("error: variable length array is not supported"),
            };
            for _ in 0..(*len) {
                visit(element, start_f, end_f)?;
            }
        }
        _ => {}
    }
    end_f(node)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    General,
    Unhandled(Cow<'static, str>),
    Schema(SchemaParseError, Vec<u8>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::General => write!(f, "error in processing data"),
            Self::Unhandled(s) => write!(f, "error in processing data: {s}"),
            Self::Schema(e, _b) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<std::fmt::Error> for Error {
    fn from(_: std::fmt::Error) -> Self {
        Self::General
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Unhandled(Cow::from(e.to_string()))
    }
}

impl Error {
    pub(crate) fn from_string(s: String) -> Self {
        Self::Unhandled(Cow::Owned(s))
    }

    pub(crate) fn from_str(s: &'static str) -> Self {
        Self::Unhandled(Cow::Borrowed(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Schema, Size};
    use crate::value::{Number, Value, ValueTree};
    use crate::walker::BufWalker;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn schema_without_str() -> Result<Schema, Error> {
        let ast = "date:[year:UINT16,month:UINT8,day:UINT8],\
            data:{4}[loc:<4>NSTR,temp:INT16,rhum:UINT16],comment:<16>NSTR";
        ast.parse()
    }

    fn schema_with_str() -> Result<Schema, Error> {
        let ast = "date:[year:UINT16,month:UINT8,day:UINT8],\
            data:{4}[loc:STR,temp:INT16,rhum:UINT16],comment:<16>NSTR";
        ast.parse()
    }

    #[test]
    fn visitor_basic_functionality() -> Result<(), Box<dyn std::error::Error>> {
        let schema = schema_without_str()?;

        let mut pos = 0;
        let mut inc_pos = |node: &Ast| -> Result<(), Error> {
            match node.size() {
                Size::Known(size) => pos += size,
                Size::Unknown => unimplemented!(),
                Size::Undefined => {}
            };
            Ok(())
        };
        visit(&schema.ast, &mut inc_pos, &mut |_| Ok(()))?;
        assert_eq!(pos, 52);
        Ok(())
    }

    #[test]
    fn visitor_read() -> Result<(), Box<dyn std::error::Error>> {
        let schema = schema_with_str()?;

        let buf = vec![
            0x07, 0xe6, 0x01, 0x01, 0x54, 0x4f, 0x4b, 0x59, 0x4f, 0x00, 0x00, 0x64, 0x00, 0x0a,
            0x4f, 0x53, 0x41, 0x4b, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x4e, 0x41, 0x47, 0x4f,
            0x59, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x46, 0x55, 0x4b, 0x55, 0x4f, 0x4b, 0x41,
            0x00, 0x00, 0x64, 0x00, 0x0a, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
        ];
        let mut walker = BufWalker::new(buf.as_slice());
        let mut vec = Vec::new();
        let mut read = |node: &Ast| {
            if !matches!(node.kind, AstKind::Struct { .. } | AstKind::Array { .. }) {
                let value = walker.read(node)?;
                vec.push(value);
            }
            Ok(())
        };
        visit(&schema.ast, &mut read, &mut |_| Ok(()))?;
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
    fn visitor_read_and_structure() -> Result<(), Box<dyn std::error::Error>> {
        let schema = schema_with_str()?;

        let buf = vec![
            0x07, 0xe6, 0x01, 0x01, 0x54, 0x4f, 0x4b, 0x59, 0x4f, 0x00, 0x00, 0x64, 0x00, 0x0a,
            0x4f, 0x53, 0x41, 0x4b, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x4e, 0x41, 0x47, 0x4f,
            0x59, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x46, 0x55, 0x4b, 0x55, 0x4f, 0x4b, 0x41,
            0x00, 0x00, 0x64, 0x00, 0x0a, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
        ];
        let mut walker = BufWalker::new(buf.as_slice());
        let tree = Rc::new(RefCell::new(ValueTree::new()));
        let tree_close = Rc::clone(&tree);
        let mut add = |node: &Ast| {
            let value = walker.read(node)?;
            tree.borrow_mut().add_value(value)?;
            Ok(())
        };
        let mut close = |node: &Ast| {
            if matches!(node.kind, AstKind::Struct { .. } | AstKind::Array { .. }) {
                tree_close.borrow_mut().close_value()?;
            }
            Ok(())
        };
        visit(&schema.ast, &mut add, &mut close)?;
        assert_eq!(walker.pos(), 63);
        assert_eq!(
            tree.as_ref().borrow_mut().get()?,
            &Value::Struct(RefCell::new(vec![
                Rc::new(Value::Struct(RefCell::new(vec![
                    Rc::new(Value::Number(Number::UInt16(2022))),
                    Rc::new(Value::Number(Number::UInt8(1))),
                    Rc::new(Value::Number(Number::UInt8(1))),
                ]))),
                Rc::new(Value::Array(RefCell::new(vec![
                    Rc::new(Value::Struct(RefCell::new(vec![
                        Rc::new(Value::String("TOKYO".to_owned())),
                        Rc::new(Value::Number(Number::Int16(100))),
                        Rc::new(Value::Number(Number::UInt16(10))),
                    ]))),
                    Rc::new(Value::Struct(RefCell::new(vec![
                        Rc::new(Value::String("OSAKA".to_owned())),
                        Rc::new(Value::Number(Number::Int16(100))),
                        Rc::new(Value::Number(Number::UInt16(10))),
                    ]))),
                    Rc::new(Value::Struct(RefCell::new(vec![
                        Rc::new(Value::String("NAGOYA".to_owned())),
                        Rc::new(Value::Number(Number::Int16(100))),
                        Rc::new(Value::Number(Number::UInt16(10))),
                    ]))),
                    Rc::new(Value::Struct(RefCell::new(vec![
                        Rc::new(Value::String("FUKUOKA".to_owned())),
                        Rc::new(Value::Number(Number::Int16(100))),
                        Rc::new(Value::Number(Number::UInt16(10))),
                    ]))),
                ]))),
                Rc::new(Value::String("0123456789abcdef".to_owned())),
            ]))
        );
        Ok(())
    }

    #[test]
    fn visitor_skip() -> Result<(), Box<dyn std::error::Error>> {
        let schema = schema_with_str()?;

        let buf = vec![
            0x07, 0xe6, 0x01, 0x01, 0x54, 0x4f, 0x4b, 0x59, 0x4f, 0x00, 0x00, 0x64, 0x00, 0x0a,
            0x4f, 0x53, 0x41, 0x4b, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x4e, 0x41, 0x47, 0x4f,
            0x59, 0x41, 0x00, 0x00, 0x64, 0x00, 0x0a, 0x46, 0x55, 0x4b, 0x55, 0x4f, 0x4b, 0x41,
            0x00, 0x00, 0x64, 0x00, 0x0a, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38,
            0x39, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
        ];
        let mut walker = BufWalker::new(buf.as_slice());
        let mut skip = |node: &Ast| walker.skip(node);
        visit(&schema.ast, &mut skip, &mut |_| Ok(()))?;
        assert_eq!(walker.pos(), 63);
        Ok(())
    }
}
