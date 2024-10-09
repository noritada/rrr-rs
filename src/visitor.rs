use std::fmt;

use crate::{
    ast::{Ast, AstKind, Len, Schema},
    param::ParamStack,
    utils::json_escape_str,
    value::{Number, Value},
    walker::BufWalker,
    Error,
};

pub trait AstVisitor {
    type ResultItem;

    fn visit_struct(&mut self, node: &Ast) -> Result<Self::ResultItem, Error>;
    fn visit_array(&mut self, node: &Ast) -> Result<Self::ResultItem, Error>;
    fn visit_builtin(&mut self, node: &Ast) -> Result<Self::ResultItem, Error>;

    fn visit(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        match node.kind {
            AstKind::Struct(_) => self.visit_struct(node),
            AstKind::Array(_, _) => self.visit_array(node),
            _ => self.visit_builtin(node),
        }
    }
}

pub struct SchemaOnelineDisplay<'a>(pub &'a Ast);

impl fmt::Display for SchemaOnelineDisplay<'_> {
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
        let is_array_element = name == "[]";
        if !is_array_element {
            write!(self.f, "{name}:")?;
        }
        Ok(())
    }
}

impl AstVisitor for SchemaOnelineFormatter<'_, '_> {
    type ResultItem = ();

    fn visit_struct(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        if let Ast {
            name,
            kind: AstKind::Struct(children),
        } = node
        {
            let is_root = name.is_empty();
            if !is_root {
                self.write_name(name)?;
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

    fn visit_array(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        if let Ast {
            name,
            kind: AstKind::Array(len, child),
        } = node
        {
            self.write_name(name)?;
            match len {
                Len::Fixed(n) => write!(self.f, "{{{n}}}"),
                Len::Variable(s) => write!(self.f, "{{{s}}}"),
                Len::Unlimited => write!(self.f, "+"),
            }?;
            self.visit(child)
        } else {
            unreachable!()
        }
    }

    fn visit_builtin(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
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
    rule: JsonFormattingStyle,
}

impl<'s, 'b> JsonDisplay<'s, 'b> {
    pub fn new(schema: &'s Schema, buf: &'b [u8], rule: JsonFormattingStyle) -> Self {
        Self { schema, buf, rule }
    }
}

impl fmt::Display for JsonDisplay<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut formatter =
            JsonSerializer::new(f, self.buf, self.schema.params.clone(), &self.rule);
        formatter.visit(&self.schema.ast).unwrap();
        Ok(())
    }
}

#[derive(PartialEq, Eq)]
pub enum JsonFormattingStyle {
    Minimal,
    Pretty,
}

pub struct JsonSerializer<'a, 'f, 'b, 'r> {
    f: &'f mut fmt::Formatter<'a>,
    walker: BufWalker<'b>,
    params: ParamStack,
    rule: &'r JsonFormattingStyle,
    // Indent level for formatting. This differs from `ParamStack::level`, which is a scope level
    // and does not increment for arrays.
    level: IndentLevel,
}

impl<'a, 'f, 'b, 'r> JsonSerializer<'a, 'f, 'b, 'r> {
    pub fn new(
        f: &'f mut fmt::Formatter<'a>,
        buf: &'b [u8],
        params: ParamStack,
        rule: &'r JsonFormattingStyle,
    ) -> Self {
        Self {
            f,
            walker: BufWalker::new(buf),
            params,
            rule,
            level: IndentLevel::new(),
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

    fn write_post_colon_space(&mut self) -> Result<(), Error> {
        if self.rule == &JsonFormattingStyle::Pretty {
            write!(self.f, " ")?;
        }
        Ok(())
    }

    fn write_newline(&mut self) -> Result<(), Error> {
        if self.rule == &JsonFormattingStyle::Pretty {
            writeln!(self.f)?;
        }
        Ok(())
    }

    fn write_indent(&mut self) -> Result<(), Error> {
        if self.rule == &JsonFormattingStyle::Pretty {
            for _ in 0..(self.level.0) {
                write!(self.f, "  ")?;
            }
        }
        Ok(())
    }
}

impl AstVisitor for JsonSerializer<'_, '_, '_, '_> {
    type ResultItem = ();

    fn visit_struct(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        if let Ast {
            kind: AstKind::Struct(children),
            ..
        } = node
        {
            write!(self.f, "{{")?;
            self.write_newline()?;
            self.params.create_scope();
            self.level.increment();

            let mut children = children.iter().peekable();
            while let Some(child) = children.next() {
                self.write_indent()?;
                write!(self.f, "\"{}\":", json_escape_str(&child.name))?;
                self.write_post_colon_space()?;
                self.visit(child)?;
                if children.peek().is_some() {
                    write!(self.f, ",")?;
                }
                self.write_newline()?;
            }

            self.level.decrement();
            self.params.clear_scope();
            self.write_indent()?;
            write!(self.f, "}}")?;
            Ok(())
        } else {
            unreachable!()
        }
    }

    fn visit_array(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        if let Ast {
            kind: AstKind::Array(len, child),
            ..
        } = node
        {
            write!(self.f, "[")?;
            self.write_newline()?;
            self.level.increment();

            // should be simplified and reusable
            if matches!(*len, Len::Unlimited) {
                let mut is_first = true;
                while !self.walker.reached_end() {
                    if is_first {
                        is_first = false;
                    } else {
                        write!(self.f, ",")?;
                        self.write_newline()?;
                    }
                    self.write_indent()?;
                    self.visit(child)?;
                }
            } else {
                let len = match *len {
                    Len::Fixed(ref n) => n,
                    Len::Variable(ref s) => self.params.get_value(s).ok_or(Error::General)?,
                    Len::Unlimited => unreachable!(),
                };
                let mut iter = (0..*len).peekable();
                while let Some(_) = iter.next() {
                    self.write_indent()?;
                    self.visit(child)?;
                    if iter.peek().is_some() {
                        write!(self.f, ",")?;
                        self.write_newline()?;
                    }
                }
            }
            self.write_newline()?;

            self.level.decrement();
            self.write_indent()?;
            write!(self.f, "]")?;
            Ok(())
        } else {
            unreachable!()
        }
    }

    fn visit_builtin(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
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
                return Err(Error::General); // parameters should be positive
                                            // numbers
            }
        }
        Ok(())
    }
}

struct IndentLevel(usize);

impl IndentLevel {
    fn new() -> Self {
        Self(0)
    }

    fn increment(&mut self) {
        self.0 += 1;
    }

    fn decrement(&mut self) {
        self.0 -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Schema;

    macro_rules! test_schema_oneline_display {
        ($(($name:ident, $schema:expr),)*) => ($(
            #[test]
            fn $name() {
                let input = $schema;
                let schema = input.parse::<Schema>().unwrap();
                let output = format!("{}", SchemaOnelineDisplay(&schema.ast));

                assert_eq!(output, input);
            }
        )*);
    }

    test_schema_oneline_display! {
        (
            schema_oneline_display_for_data_with_fixed_length_builtin_type_array,
            "fld1:{3}INT8"
        ),
        (
            schema_oneline_display_for_data_with_variable_length_struct_array,
            "fld1:[sfld1:[ssfld1:<4>NSTR,ssfld2:STR,ssfld3:INT32]],\
            fld2:INT8,fld3:{fld1}[sfld1:<4>NSTR,sfld2:STR,sfld3:INT32],\
            fld3:+INT8"
        ),
    }

    const NESTED_DATA_SCHEMA: &str =
        "count:UINT8,fld1:{count}[sfld1:[ssfld1:{count}[count:UINT8,sssfld1:{count}[ssssfld1:\
        {count}[sssssfld1:UINT8,count:UINT8]]]]]";
    const NESTED_DATA_BUF: &[u8] = &[
        0x02, 0x02, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x04, 0x04, 0x03, 0x01, 0x01, 0x02, 0x02,
        0x03, 0x03, 0x04, 0x04, 0x05, 0x05, 0x06, 0x06, 0x07, 0x07, 0x08, 0x08, 0x09, 0x09, 0x01,
        0x01, 0x01, 0x02, 0x01, 0x01, 0x02, 0x02, 0x03, 0x03, 0x04, 0x04,
    ];
    const NESTED_DATA_EXPECTED: &str = r#"{
  "count": 2,
  "fld1": [
    {
      "sfld1": {
        "ssfld1": [
          {
            "count": 2,
            "sssfld1": [
              {
                "ssssfld1": [
                  {
                    "sssssfld1": 1,
                    "count": 1
                  },
                  {
                    "sssssfld1": 2,
                    "count": 2
                  }
                ]
              },
              {
                "ssssfld1": [
                  {
                    "sssssfld1": 3,
                    "count": 3
                  },
                  {
                    "sssssfld1": 4,
                    "count": 4
                  }
                ]
              }
            ]
          },
          {
            "count": 3,
            "sssfld1": [
              {
                "ssssfld1": [
                  {
                    "sssssfld1": 1,
                    "count": 1
                  },
                  {
                    "sssssfld1": 2,
                    "count": 2
                  },
                  {
                    "sssssfld1": 3,
                    "count": 3
                  }
                ]
              },
              {
                "ssssfld1": [
                  {
                    "sssssfld1": 4,
                    "count": 4
                  },
                  {
                    "sssssfld1": 5,
                    "count": 5
                  },
                  {
                    "sssssfld1": 6,
                    "count": 6
                  }
                ]
              },
              {
                "ssssfld1": [
                  {
                    "sssssfld1": 7,
                    "count": 7
                  },
                  {
                    "sssssfld1": 8,
                    "count": 8
                  },
                  {
                    "sssssfld1": 9,
                    "count": 9
                  }
                ]
              }
            ]
          }
        ]
      }
    },
    {
      "sfld1": {
        "ssfld1": [
          {
            "count": 1,
            "sssfld1": [
              {
                "ssssfld1": [
                  {
                    "sssssfld1": 1,
                    "count": 1
                  }
                ]
              }
            ]
          },
          {
            "count": 2,
            "sssfld1": [
              {
                "ssssfld1": [
                  {
                    "sssssfld1": 1,
                    "count": 1
                  },
                  {
                    "sssssfld1": 2,
                    "count": 2
                  }
                ]
              },
              {
                "ssssfld1": [
                  {
                    "sssssfld1": 3,
                    "count": 3
                  },
                  {
                    "sssssfld1": 4,
                    "count": 4
                  }
                ]
              }
            ]
          }
        ]
      }
    }
  ]
}"#;

    macro_rules! test_json_serialization {
        ($(($name:ident, $schema:expr, $buf:expr, $expected:expr),)*) => ($(
            #[test]
            fn $name() {
                let schema = $schema.parse::<Schema>().unwrap();
                let buf = $buf;
                let actual = format!("{}", JsonDisplay::new(&schema, &buf, JsonFormattingStyle::Minimal));
                let expected = $expected
                    .chars()
                    .filter(|c| *c != ' ' && *c != '\n')
                    .collect::<String>();

                assert_eq!(actual, expected);
            }
        )*);
    }

    test_json_serialization! {
        (
            json_serialization_for_data_with_fixed_length_builtin_type_array,
            "fld1:{3}INT8",
            vec![0x01, 0x02, 0x03],
            r#"
                {
                    "fld1": [1, 2, 3]
                }
            "#
        ),
        (
            json_serialization_for_data_with_variable_length_struct_array,
            NESTED_DATA_SCHEMA,
            NESTED_DATA_BUF,
            NESTED_DATA_EXPECTED
        ),
    }

    #[test]
    fn json_serialization_with_pretty_printing_style() {
        let schema = NESTED_DATA_SCHEMA.parse::<Schema>().unwrap();
        let actual = format!(
            "{}",
            JsonDisplay::new(&schema, NESTED_DATA_BUF, JsonFormattingStyle::Pretty)
        );
        let expected = NESTED_DATA_EXPECTED.to_string();

        assert_eq!(actual, expected);
    }
}
