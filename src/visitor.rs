use crate::ast::{Ast, AstKind, Len, Schema};
use crate::param::ParamStack;
use crate::utils::json_escape_str;
use crate::value::{Number, Value};
use crate::walker::BufWalker;
use crate::Error;
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
        formatter.visit(inner).unwrap();
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
        write!(self.f, "{name}:")
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
            let is_root = name.is_empty();
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
                Len::Fixed(n) => write!(self.f, "{{{n}"),
                Len::Variable(s) => write!(self.f, "{{{s}"),
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
            AstKind::NStr(n) => write!(self.f, "<{n}>NSTR"),
            AstKind::Struct(..) => unreachable!(),
            AstKind::Array(..) => unreachable!(),
        }?;
        Ok(())
    }
}

pub struct JsonDisplay<'s, 'b> {
    schema: &'s Schema,
    buf: &'b [u8],
}

impl<'s, 'b> JsonDisplay<'s, 'b> {
    pub fn new(schema: &'s Schema, buf: &'b [u8]) -> Self {
        Self { schema, buf }
    }
}

impl<'s, 'b> fmt::Display for JsonDisplay<'s, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut formatter = JsonSerializer::new(f, self.buf, self.schema.params.clone());
        formatter.visit(&self.schema.ast).unwrap();
        Ok(())
    }
}

pub struct JsonSerializer<'a, 'f, 'b> {
    f: &'f mut fmt::Formatter<'a>,
    walker: BufWalker<'b>,
    params: ParamStack,
}

impl<'a, 'f, 'b> JsonSerializer<'a, 'f, 'b> {
    pub fn new(f: &'f mut fmt::Formatter<'a>, buf: &'b [u8], params: ParamStack) -> Self {
        Self {
            f,
            walker: BufWalker::new(buf),
            params,
        }
    }

    fn write_number(&mut self, n: &Number) -> fmt::Result {
        match *n {
            Number::Int8(n) => write!(self.f, "{n}"),
            Number::Int16(n) => write!(self.f, "{n}"),
            Number::Int32(n) => write!(self.f, "{n}"),
            Number::UInt8(n) => write!(self.f, "{n}"),
            Number::UInt16(n) => write!(self.f, "{n}"),
            Number::UInt32(n) => write!(self.f, "{n}"),
            Number::Float32(n) => write!(self.f, "{n}"),
            Number::Float64(n) => write!(self.f, "{n}"),
        }
    }

    fn write_string(&mut self, s: &str) -> Result<(), Error> {
        write!(self.f, "\"{}\"", json_escape_str(s))?;
        Ok(())
    }
}

impl<'a, 'f, 'b> AstVisitor for JsonSerializer<'a, 'f, 'b> {
    fn visit_struct(&mut self, node: &Ast) -> Result<(), Error> {
        if let Ast {
            kind: AstKind::Struct(children),
            ..
        } = node
        {
            write!(self.f, "{{")?;
            self.params.create_scope();

            let mut children = children.iter().peekable();
            while let Some(child) = children.next() {
                write!(self.f, "\"{}\":", json_escape_str(&child.name))?;
                self.visit(child)?;
                if children.peek().is_some() {
                    write!(self.f, ",")?;
                }
            }

            self.params.clear_scope();
            write!(self.f, "}}")?;
            Ok(())
        } else {
            unreachable!()
        }
    }

    fn visit_array(&mut self, node: &Ast) -> Result<(), Error> {
        if let Ast {
            kind: AstKind::Array(len, child),
            ..
        } = node
        {
            write!(self.f, "[")?;

            let len = match *len {
                Len::Fixed(ref n) => n,
                Len::Variable(ref s) => self.params.get_value(s).ok_or(Error::General)?,
            };
            let mut iter = (0..*len).peekable();
            while let Some(_) = iter.next() {
                self.visit(child)?;
                if iter.peek().is_some() {
                    write!(self.f, ",")?;
                }
            }

            write!(self.f, "]")?;
            Ok(())
        } else {
            unreachable!()
        }
    }

    fn visit_builtin(&mut self, node: &Ast) -> Result<(), Error> {
        let value = self.walker.read(node)?;
        match value {
            Value::Number(ref n) => self.write_number(n)?,
            Value::String(ref s) => self.write_string(s)?,
            _ => unreachable!(),
        };

        let name = node.name.as_str();
        if self.params.contains(name) {
            if let Value::Number(ref n) = value {
                self.params.push_value(name, (*n).clone().try_into()?);
            } else {
                return Err(Error::General); // parameters should be positive numbers
            }
        }
        Ok(())
    }
}

#[cfg(test)]
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
    fn json_serialization() {
        let input = "count:UINT8,fld1:{count}[sfld1:[ssfld1:{count}[count:UINT8,sssfld1:{count}[ssssfld1:{count}[sssssfld1:UINT8,count:UINT8]]]]]";
        let schema = input.parse::<Schema>().unwrap();
        let buf = vec![
            0x02, 0x02, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x04, 0x04, 0x03, 0x01, 0x01, 0x02,
            0x02, 0x03, 0x03, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07, 0x08, 0x08, 0x09,
            0x09, 0x01, 0x01, 0x01, 0x02, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x04, 0x04,
        ];
        let actual = format!("{}", JsonDisplay::new(&schema, &buf));
        let expected = r#"
            {
                "count": 2,
                "fld1": [
                    {"sfld1": {
                        "ssfld1": [
                            {
                                "count": 2,
                                "sssfld1": [
                                    {
                                        "ssssfld1": [
                                            {"sssssfld1": 1, "count": 1},
                                            {"sssssfld1": 2, "count": 2}
                                        ]
                                    },
                                    {
                                        "ssssfld1": [
                                            {"sssssfld1": 3, "count": 3},
                                            {"sssssfld1": 4, "count": 4}
                                        ]
                                    }
                                ]
                            },
                            {
                                "count": 3,
                                "sssfld1": [
                                    {
                                        "ssssfld1": [
                                            {"sssssfld1": 1, "count": 1},
                                            {"sssssfld1": 2, "count": 2},
                                            {"sssssfld1": 3, "count": 3}
                                        ]
                                    },
                                    {
                                        "ssssfld1": [
                                            {"sssssfld1": 4, "count": 4},
                                            {"sssssfld1": 5, "count": 5},
                                            {"sssssfld1": 6, "count": 6}
                                        ]
                                    },
                                    {
                                        "ssssfld1": [
                                            {"sssssfld1": 7, "count": 7},
                                            {"sssssfld1": 8, "count": 8},
                                            {"sssssfld1": 9, "count": 9}
                                        ]
                                    }
                                ]
                            }
                        ]
                    }},
                    {"sfld1": {
                        "ssfld1": [
                            {
                                "count": 1,
                                "sssfld1": [
                                    {
                                        "ssssfld1": [
                                            {"sssssfld1": 1, "count": 1}
                                        ]
                                    }
                                ]
                            },
                            {
                                "count": 2,
                                "sssfld1": [
                                    {
                                        "ssssfld1": [
                                            {"sssssfld1": 1, "count": 1},
                                            {"sssssfld1": 2, "count": 2}
                                        ]
                                    },
                                    {
                                        "ssssfld1": [
                                            {"sssssfld1": 3, "count": 3},
                                            {"sssssfld1": 4, "count": 4}
                                        ]
                                    }
                                ]
                            }
                        ]
                    }}
                ]
            }
        "#;
        let expected = expected
            .chars()
            .filter(|c| *c != ' ' && *c != '\n')
            .collect::<String>();

        assert_eq!(actual, expected);
    }
}
