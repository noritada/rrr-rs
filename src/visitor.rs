use crate::ast::{Ast, AstKind, Len};
use crate::Error;
use std::borrow::Cow;
use std::fmt;

pub trait AstVisitor {
    fn visit_struct(&mut self, node: &Ast) -> Result<(), Error>;
    fn visit_array(&mut self, node: &Ast) -> Result<(), Error>;
    fn visit_builtin(&mut self, node: &Ast) -> Result<(), Error>;

    fn visit(&mut self, node: &Ast) -> Result<(), Error> {
        match node.kind {
            AstKind::Struct(_) => self.visit_struct(node),
            AstKind::Array(_, _) => self.visit_array(node),
            _ => self.visit_builtin(node),
        }
    }
}

pub struct SchemaOnelineDisplay<'a>(pub &'a Ast);

impl<'a> fmt::Display for SchemaOnelineDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut formatter = SchemaOnelineFormatter::new(f);
        let Self(inner) = self;
        formatter.visit(&inner).unwrap();
        Ok(())
    }
}

struct SchemaOnelineFormatter<'a, 'f> {
    f: &'f mut fmt::Formatter<'a>,
}

impl<'a, 'f> SchemaOnelineFormatter<'a, 'f> {
    fn new(f: &'f mut fmt::Formatter<'a>) -> Self {
        Self { f }
    }

    fn write_name(&mut self, name: &str) -> fmt::Result {
        write!(self.f, "{}:", name)
    }
}

impl<'a, 'f> AstVisitor for SchemaOnelineFormatter<'a, 'f> {
    fn visit_struct(&mut self, node: &Ast) -> Result<(), Error> {
        if let Ast {
            name,
            kind: AstKind::Struct(children),
        } = node
        {
            let is_array_element = name == "[]";
            let is_root = name == "";
            if !is_array_element && !is_root {
                self.write_name(name)?;
            }
            if !is_root {
                write!(self.f, "[")?;
            }

            let mut children = children.iter().peekable();
            while let Some(child) = children.next() {
                self.visit(child)?;
                if children.peek().is_some() {
                    write!(self.f, ",")?;
                }
            }

            if !is_root {
                write!(self.f, "]")?;
            }
            Ok(())
        } else {
            unreachable!()
        }
    }

    fn visit_array(&mut self, node: &Ast) -> Result<(), Error> {
        if let Ast {
            name,
            kind: AstKind::Array(len, child),
        } = node
        {
            self.write_name(name)?;
            match len {
                Len::Fixed(n) => write!(self.f, "{{{}", n),
                Len::Variable(s) => write!(self.f, "{{{}", s),
            }?;
            write!(self.f, "}}")?;
            self.visit(child)
        } else {
            unreachable!()
        }
    }

    fn visit_builtin(&mut self, node: &Ast) -> Result<(), Error> {
        self.write_name(&node.name)?;
        match node.kind {
            AstKind::Int8 => write!(self.f, "INT8"),
            AstKind::Int16 => write!(self.f, "INT16"),
            AstKind::Int32 => write!(self.f, "INT32"),
            AstKind::UInt8 => write!(self.f, "UINT8"),
            AstKind::UInt16 => write!(self.f, "UINT16"),
            AstKind::UInt32 => write!(self.f, "UINT32"),
            AstKind::Float32 => write!(self.f, "FLOAT32"),
            AstKind::Float64 => write!(self.f, "FLOAT64"),
            AstKind::Str => write!(self.f, "STR"),
            AstKind::NStr(n) => write!(self.f, "<{}>NSTR", n),
            AstKind::Struct(..) => unreachable!(),
            AstKind::Array(..) => unreachable!(),
        }?;
        Ok(())
    }
}

pub struct SchemaTreeDisplay<'a>(pub &'a Ast);

impl<'a> fmt::Display for SchemaTreeDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut formatter = SchemaTreeFormatter::new(f);
        let Self(inner) = self;
        formatter.visit(&inner).unwrap();
        Ok(())
    }
}

struct SchemaTreeFormatter<'a, 'f> {
    f: &'f mut fmt::Formatter<'a>,
    levels: Vec<bool>, // elements are `has_next_sibling` values
}

impl<'a, 'f> SchemaTreeFormatter<'a, 'f> {
    fn new(f: &'f mut fmt::Formatter<'a>) -> Self {
        Self {
            f,
            levels: Vec::new(),
        }
    }

    fn write_line(&mut self, name: &str, kind: &AstKind) -> fmt::Result {
        self.write_branch()?;
        self.write_type(name, kind)?;
        write!(self.f, "\n")
    }

    fn write_branch(&mut self) -> fmt::Result {
        let mut levels = self.levels.iter().peekable();
        while let Some(has_next_sibling) = levels.next() {
            let symbol = if levels.peek().is_some() {
                if *has_next_sibling {
                    "│   "
                } else {
                    "    "
                }
            } else {
                if *has_next_sibling {
                    "├── "
                } else {
                    "└── "
                }
            };
            write!(self.f, "{}", symbol)?;
        }
        Ok(())
    }

    fn write_type(&mut self, name: &str, kind: &AstKind) -> fmt::Result {
        write!(self.f, "{}: ", name)?;
        match kind {
            AstKind::Int8 => write!(self.f, "INT8"),
            AstKind::Int16 => write!(self.f, "INT16"),
            AstKind::Int32 => write!(self.f, "INT32"),
            AstKind::UInt8 => write!(self.f, "UINT8"),
            AstKind::UInt16 => write!(self.f, "UINT16"),
            AstKind::UInt32 => write!(self.f, "UINT32"),
            AstKind::Float32 => write!(self.f, "FLOAT32"),
            AstKind::Float64 => write!(self.f, "FLOAT64"),
            AstKind::Str => write!(self.f, "STR"),
            AstKind::NStr(n) => write!(self.f, "<{}>NSTR", n),
            AstKind::Struct(..) => write!(self.f, "Struct"),
            AstKind::Array(len, ..) => {
                write!(self.f, "Array (length: ")?;
                match len {
                    Len::Fixed(n) => write!(self.f, "{}", n),
                    Len::Variable(s) => write!(self.f, "{}", s),
                }?;
                write!(self.f, ")")
            }
        }
    }
}

impl<'a, 'f> AstVisitor for SchemaTreeFormatter<'a, 'f> {
    fn visit_struct(&mut self, node: &Ast) -> Result<(), Error> {
        if let Ast {
            name,
            kind: AstKind::Struct(children),
        } = node
        {
            let name = match name.as_str() {
                "" => "/",
                "[]" => "[index]",
                s => s,
            };

            self.write_line(name, &node.kind)?;
            let mut children = children.iter().peekable();
            while let Some(child) = children.next() {
                let has_next_sibling = children.peek().is_some();
                self.levels.push(has_next_sibling);
                self.visit(child)?;
                self.levels.pop();
            }
            Ok(())
        } else {
            unreachable!()
        }
    }

    fn visit_array(&mut self, node: &Ast) -> Result<(), Error> {
        if let Ast {
            kind: AstKind::Array(_, child),
            ..
        } = node
        {
            self.write_line(&node.name, &node.kind)?;
            self.levels.push(false);
            self.visit(child)?;
            self.levels.pop();
            Ok(())
        } else {
            unreachable!()
        }
    }

    fn visit_builtin(&mut self, node: &Ast) -> Result<(), Error> {
        self.write_line(&node.name, &node.kind)?;
        Ok(())
    }
}

fn json_escape_str(input: &str) -> Cow<str> {
    for (i, byte) in input.as_bytes().iter().enumerate() {
        if json_escape_byte(byte).is_some() {
            // assuming that 1 byte would be converted to 2 bytes
            let mut escaped_string = String::with_capacity(input.len() * 2);
            escaped_string.push_str(&input[..i]);
            for byte in input.as_bytes().iter() {
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
mod tests {
    use super::*;
    use crate::ast::Schema;

    #[test]
    fn schema_oneline_display() {
        let input = "fld1:[sfld1:[ssfld1:<4>NSTR,ssfld2:STR,ssfld3:INT32]],\
            fld2:INT8,fld3:{fld1}[sfld1:<4>NSTR,sfld2:STR,sfld3:INT32]";
        let schema = input.parse::<Schema>().unwrap();
        let output = format!("{}", SchemaOnelineDisplay(&schema.ast));

        assert_eq!(output, input);
    }

    #[test]
    fn schema_tree_display() {
        let input = "fld1:[sfld1:[ssfld1:<4>NSTR,ssfld2:STR,ssfld3:INT32]],\
            fld2:INT8,fld3:{fld1}[sfld1:<4>NSTR,sfld2:STR,sfld3:INT32]";
        let schema = input.parse::<Schema>().unwrap();
        let actual = format!("{}", SchemaTreeDisplay(&schema.ast));
        let expected = "/: Struct
├── fld1: Struct
│   └── sfld1: Struct
│       ├── ssfld1: <4>NSTR
│       ├── ssfld2: STR
│       └── ssfld3: INT32
├── fld2: INT8
└── fld3: Array (length: fld1)
    └── [index]: Struct
        ├── sfld1: <4>NSTR
        ├── sfld2: STR
        └── sfld3: INT32
";

        assert_eq!(actual, expected);
    }

    #[test]
    fn json_escape() {
        let input: String = (0x00u8..0x80u8).map(|b| b as char).collect();
        let actual = json_escape_str(input.as_str());
        let expected = vec![
            r##"\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t\n\u000B\f\r\u000E\u000F"##,
            r##"\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001A\u001B\u001C\u001D\u001E\u001F"##,
            r##" !\"#$%&'()*+,-./0123456789:;<=>?"##,
            r##"@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_"##,
            r##"`abcdefghijklmnopqrstuvwxyz{|}~\u007F"##,
        ];
        let expected = expected
            .iter()
            .map(|s| s.to_owned())
            .collect::<Vec<_>>()
            .join("");
        assert_eq!(actual, expected);
    }
}
