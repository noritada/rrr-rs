use crate::ast::{Ast, AstKind, Len};
use crate::Error;
use std::fmt;

pub(crate) trait AstVisitor {
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

pub(crate) struct SchemaOnelineDisplay<'a>(&'a Ast);

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

pub(crate) struct SchemaTreeDisplay<'a>(&'a Ast);

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
}
