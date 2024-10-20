use std::fmt;

use console::Style;
use rrr::{Ast, AstKind, AstVisitor, Error, Len};

pub(crate) struct FieldCounter(usize);

impl FieldCounter {
    pub(crate) fn new() -> Self {
        Self(0)
    }

    pub(crate) fn count(node: &Ast) -> Result<usize, Error> {
        let mut counter = Self::new();
        counter.visit(node)?;
        let Self(count) = counter;
        Ok(count)
    }

    #[inline]
    fn visit_default(&mut self) -> Result<(), Error> {
        self.increment();
        Ok(())
    }

    #[inline]
    fn increment(&mut self) {
        let Self(ref mut inner) = self;
        *inner += 1;
    }
}

impl AstVisitor for FieldCounter {
    type ResultItem = ();

    fn visit_struct(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        self.visit_default()?;
        if let Ast {
            kind: AstKind::Struct(children),
            ..
        } = node
        {
            for child in children.iter() {
                self.visit(child)?;
            }
        }
        Ok(())
    }

    fn visit_array(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        self.visit_default()?;
        if let Ast {
            kind: AstKind::Array(_, child),
            ..
        } = node
        {
            self.visit(child)?;
        }
        Ok(())
    }

    fn visit_builtin(&mut self, _: &Ast) -> Result<Self::ResultItem, Error> {
        self.visit_default()
    }
}

pub(crate) struct SchemaTreeDisplay<'a>(pub &'a Ast);

impl<'a> fmt::Display for SchemaTreeDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut formatter = SchemaTreeFormatter::new(f);
        let Self(inner) = self;
        formatter.visit(inner).unwrap();
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
        writeln!(self.f)
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
            } else if *has_next_sibling {
                "├── "
            } else {
                "└── "
            };
            write!(self.f, "{symbol}")?;
        }
        Ok(())
    }

    fn write_type(&mut self, name: &str, kind: &AstKind) -> fmt::Result {
        let yellow = Style::new().yellow().bold();
        write!(self.f, "{}: ", yellow.apply_to(name))?;
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
            AstKind::NStr(n) => write!(self.f, "<{n}>NSTR"),
            AstKind::Struct(..) => write!(self.f, "Struct"),
            AstKind::Array(len, ..) => {
                write!(self.f, "Array (length: ")?;
                match len {
                    Len::Fixed(n) => write!(self.f, "fixed ({n})"),
                    Len::Variable(s) => write!(self.f, "variable ({s})"),
                    Len::Unlimited => write!(self.f, "unlimited"),
                }?;
                write!(self.f, ")")
            }
        }
    }
}

impl<'a, 'f> AstVisitor for SchemaTreeFormatter<'a, 'f> {
    type ResultItem = ();

    fn visit_struct(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        if let Ast {
            name,
            kind: AstKind::Struct(children),
        } = node
        {
            self.write_line(prettify_special_field_name(name), &node.kind)?;
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

    fn visit_array(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        if let Ast {
            kind: AstKind::Array(_, child),
            ..
        } = node
        {
            self.write_line(prettify_special_field_name(&node.name), &node.kind)?;
            self.levels.push(false);
            self.visit(child)?;
            self.levels.pop();
            Ok(())
        } else {
            unreachable!()
        }
    }

    fn visit_builtin(&mut self, node: &Ast) -> Result<Self::ResultItem, Error> {
        self.write_line(prettify_special_field_name(&node.name), &node.kind)?;
        Ok(())
    }
}

fn prettify_special_field_name(name: &str) -> &str {
    match name {
        "" => "/",
        "[]" => "[index]",
        s => s,
    }
}

#[cfg(test)]
mod tests {
    use rrr::{DataReaderOptions, Schema};

    use super::*;

    macro_rules! test_schema_tree_display {
        ($(($name:ident, $input:expr, $expected:expr),)*) => ($(
            #[test]
            fn $name() {
                let input = $input;
                let options = DataReaderOptions::default();
                let schema = Schema::try_from((input.as_bytes(), options)).unwrap();
                let actual = format!("{}", SchemaTreeDisplay(&schema.ast));
                let actual = console::strip_ansi_codes(&actual);
                let expected = $expected;

                assert_eq!(actual, expected);
            }
        )*);
    }

    test_schema_tree_display! {
        (
            schema_tree_display_for_data_with_fixed_length_builtin_type_array,
            "fld1:{3}INT8",
            "/: Struct
└── fld1: Array (length: fixed (3))
    └── [index]: INT8
"
        ),
        (
            schema_tree_display_for_data_with_variable_length_struct_array,
            "fld1:[sfld1:[ssfld1:<4>NSTR,ssfld2:STR,ssfld3:INT32]],\
            fld2:INT8,fld3:{fld1}[sfld1:<4>NSTR,sfld2:STR,sfld3:INT32]",
            "/: Struct
├── fld1: Struct
│   └── sfld1: Struct
│       ├── ssfld1: <4>NSTR
│       ├── ssfld2: STR
│       └── ssfld3: INT32
├── fld2: INT8
└── fld3: Array (length: variable (fld1))
    └── [index]: Struct
        ├── sfld1: <4>NSTR
        ├── sfld2: STR
        └── sfld3: INT32
"
        ),
        (
            schema_tree_display_for_data_with_unlimited_length_struct_array,
            "fld1:[sfld1:[ssfld1:<4>NSTR,ssfld2:STR,ssfld3:INT32]],\
            fld2:INT8,fld3:+[sfld1:<4>NSTR,sfld2:STR,sfld3:INT32]",
            "/: Struct
├── fld1: Struct
│   └── sfld1: Struct
│       ├── ssfld1: <4>NSTR
│       ├── ssfld2: STR
│       └── ssfld3: INT32
├── fld2: INT8
└── fld3: Array (length: unlimited)
    └── [index]: Struct
        ├── sfld1: <4>NSTR
        ├── sfld2: STR
        └── sfld3: INT32
"
        ),
    }
}
