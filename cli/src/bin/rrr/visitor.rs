use rrr::{Ast, AstKind, AstVisitor, Error};

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
    fn visit_struct(&mut self, node: &Ast) -> Result<(), Error> {
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

    fn visit_array(&mut self, node: &Ast) -> Result<(), Error> {
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

    fn visit_builtin(&mut self, _: &Ast) -> Result<(), Error> {
        self.visit_default()
    }
}
