use crate::ast::{Ast, AstKind, Len};
use crate::Error;
use std::fmt;

pub(crate) trait AstVisitor {
    fn visit_struct(&mut self, name: &str, children: &Vec<Ast>) -> Result<(), Error>;
    fn visit_array(&mut self, name: &str, len: &Len, child: &Ast) -> Result<(), Error>;
    fn visit_builtin(&mut self, node: &Ast) -> Result<(), Error>;

    fn visit(&mut self, node: &Ast) -> Result<(), Error> {
        match node {
            Ast {
                kind: AstKind::Struct(children),
                name,
            } => self.visit_struct(name, children),

            Ast {
                kind: AstKind::Array(len, child),
                name,
            } => self.visit_array(name, len, child),

            node => self.visit_builtin(node),
        }
    }
}

pub(crate) struct OnelineSchema(Ast);

impl fmt::Display for OnelineSchema {
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
    fn visit_struct(&mut self, name: &str, children: &Vec<Ast>) -> Result<(), Error> {
        let is_array_element = name == "[]";
        let is_root = name == "";
        if !is_array_element && !is_root {
            self.write_name(name)?;
        }
        if !is_root {
            write!(self.f, "[")?;
        }

        let mut first = true;
        for child in children.iter() {
            if first {
                first = false;
            } else {
                write!(self.f, ",")?;
            }
            self.visit(child)?;
        }

        if !is_root {
            write!(self.f, "]")?;
        }
        Ok(())
    }

    fn visit_array(&mut self, name: &str, len: &Len, child: &Ast) -> Result<(), Error> {
        self.write_name(name)?;
        match len {
            Len::Fixed(n) => write!(self.f, "{{{}", n),
            Len::Variable(s) => write!(self.f, "{{{}", s),
        }?;
        write!(self.f, "}}")?;
        self.visit(child)
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

mod tests {
    use super::*;
    use crate::ast::Schema;

    #[test]
    fn schema_oneline_display() {
        let input = "fld1:[sfld1:[ssfld1:<4>NSTR,ssfld2:STR,ssfld3:INT32]],\
            fld2:INT8,fld3:{fld1}[sfld1:<4>NSTR,sfld2:STR,sfld3:INT32]";
        let schema = input.parse::<Schema>().unwrap();
        let output = format!("{}", OnelineSchema(schema.ast));

        assert_eq!(output, input);
    }
}
