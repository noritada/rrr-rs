use crate::Error;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
pub(crate) enum Value {
    Number(Number),
    String(String),
    Struct(RefCell<Vec<Rc<Value>>>),
}

impl Value {
    pub(crate) fn new_struct() -> Self {
        Self::Struct(RefCell::new(Vec::new()))
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Number {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    Float32(f32),
    Float64(f64),
}

macro_rules! add_impl_for_types {
    ($(($ty:ty,$variant:ident),)*) => ($(
        impl From<$ty> for Number {
            fn from(n: $ty) -> Number {
                Number::$variant(n)
            }
        }
    )*);
}

add_impl_for_types![
    (i8, Int8),
    (i16, Int16),
    (i32, Int32),
    (u8, UInt8),
    (u16, UInt16),
    (u32, UInt32),
    (f32, Float32),
    (f64, Float64),
];

#[derive(Debug)]
pub(crate) struct ValueTree {
    heads: Vec<Rc<Value>>,
    completed: bool,
}

impl ValueTree {
    pub(crate) fn new() -> Self {
        Self {
            heads: Vec::new(),
            completed: false,
        }
    }

    pub(crate) fn add_value(&mut self, value: Value) -> Result<(), Error> {
        if self.completed {
            return Err(Error); // TODO: make more descriptive
        }

        let value_is_struct = matches!(value, Value::Struct { .. });
        let value_rc = Rc::new(value);
        let head = self.heads.last_mut();
        if let Some(head_value) = head {
            if let Value::Struct(vec) = head_value.as_ref() {
                vec.borrow_mut().push(Rc::clone(&value_rc));
                if value_is_struct {
                    self.heads.push(value_rc);
                }
            } else {
                return Err(Error); // TODO: make more descriptive
            }
        } else {
            if value_is_struct {
                self.heads.push(value_rc);
            }
        }

        Ok(())
    }

    pub(crate) fn close_value(&mut self) -> Result<(), Error> {
        if self.completed {
            return Err(Error); // TODO: make more descriptive
        }

        if self.heads.len() == 1 {
            self.completed = true;
        } else {
            let _ = self.heads.pop();
        }
        Ok(())
    }

    pub(crate) fn get<'s>(&'s mut self) -> Result<&'s Value, Error> {
        if !self.completed {
            return Err(Error); // TODO: make more descriptive
        }

        let value_rc = self.heads.first().ok_or(Error)?; // TODO: make more descriptive
        let value = value_rc.as_ref();
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_tree_with_single_empty_layer() -> Result<(), Box<dyn std::error::Error>> {
        let mut tree = ValueTree::new();
        tree.add_value(Value::new_struct())?;
        tree.close_value()?;

        let result = &*tree.get()?;
        assert_eq!(result, &Value::Struct(RefCell::new(Vec::new())));
        Ok(())
    }

    #[test]
    fn value_tree_with_single_layer() -> Result<(), Box<dyn std::error::Error>> {
        let mut tree = ValueTree::new();
        tree.add_value(Value::new_struct())?;
        tree.add_value(Value::Number(2022u16.into()))?;
        tree.add_value(Value::Number(1u8.into()))?;
        tree.close_value()?;

        let result = &*tree.get()?;
        assert_eq!(
            result,
            &Value::Struct(RefCell::new(vec![
                Rc::new(Value::Number(Number::UInt16(2022))),
                Rc::new(Value::Number(Number::UInt8(1))),
            ]))
        );
        Ok(())
    }

    #[test]
    fn value_tree_with_two_layers_without_non_struct_values(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut tree = ValueTree::new();
        tree.add_value(Value::new_struct())?;
        tree.add_value(Value::new_struct())?;
        tree.close_value()?;
        tree.close_value()?;

        let result = &*tree.get()?;
        assert_eq!(
            result,
            &Value::Struct(RefCell::new(vec![Rc::new(Value::Struct(RefCell::new(
                Vec::new()
            ))),]))
        );
        Ok(())
    }

    #[test]
    fn value_tree_with_single_layer_with_number_and_struct(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut tree = ValueTree::new();
        tree.add_value(Value::new_struct())?;
        tree.add_value(Value::Number(2022u16.into()))?;
        tree.add_value(Value::new_struct())?;
        tree.close_value()?;
        tree.close_value()?;

        let result = &*tree.get()?;
        assert_eq!(
            result,
            &Value::Struct(RefCell::new(vec![
                Rc::new(Value::Number(Number::UInt16(2022))),
                Rc::new(Value::Struct(RefCell::new(Vec::new()))),
            ]))
        );
        Ok(())
    }

    #[test]
    fn value_tree_with_two_layers_with_numbers() -> Result<(), Box<dyn std::error::Error>> {
        let mut tree = ValueTree::new();
        tree.add_value(Value::new_struct())?;
        tree.add_value(Value::new_struct())?;
        tree.add_value(Value::Number(2022u16.into()))?;
        tree.add_value(Value::Number(1u8.into()))?;
        tree.close_value()?;
        tree.close_value()?;

        let result = &*tree.get()?;
        assert_eq!(
            result,
            &Value::Struct(RefCell::new(vec![Rc::new(Value::Struct(RefCell::new(
                vec![
                    Rc::new(Value::Number(Number::UInt16(2022))),
                    Rc::new(Value::Number(Number::UInt8(1))),
                ]
            ))),]))
        );
        Ok(())
    }

    #[test]
    fn value_tree_with_layers_unclosed() -> Result<(), Box<dyn std::error::Error>> {
        let mut tree = ValueTree::new();
        tree.add_value(Value::new_struct())?;
        tree.add_value(Value::new_struct())?;
        tree.close_value()?;

        let result = tree.get();
        assert_eq!(result, Err(Error));
        Ok(())
    }
}
