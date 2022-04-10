use std::collections::HashMap;

type ParamLevel = usize;
type ParamValue = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamStack {
    level: ParamLevel,
    stacks: HashMap<String, Vec<(ParamLevel, ParamValue)>>,
}

impl ParamStack {
    pub(crate) fn new() -> Self {
        ParamStack {
            level: 0,
            stacks: HashMap::new(),
        }
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.stacks.contains_key(name)
    }

    pub(crate) fn add_entry(&mut self, name: &str) {
        // ignores the original entry even if it existed
        self.stacks.insert(name.to_string(), Vec::new());
    }

    #[inline]
    pub(crate) fn create_scope(&mut self) {
        self.level += 1;
    }

    pub(crate) fn clear_scope(&mut self) {
        for (_, stack) in self.stacks.iter_mut() {
            if let Some((l, _)) = stack.last() {
                if *l == self.level {
                    stack.pop();
                }
            }
        }
        self.level -= 1;
    }

    pub(crate) fn get_value(&self, name: &str) -> Option<&ParamValue> {
        let (_, value) = self.stacks.get(name).and_then(|stack| stack.last())?;
        Some(value)
    }

    pub(crate) fn push_value(&mut self, name: &str, value: ParamValue) -> Option<()> {
        self.stacks
            .get_mut(name)
            .map(|stack| stack.push((self.level, value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope() {
        let mut params = ParamStack::new();
        params.add_entry("p1");

        params.create_scope();
        params.push_value("p1", 1);
        assert_eq!(params.stacks.get("p1"), Some(&vec![(1, 1),]));

        params.create_scope();
        assert_eq!(params.stacks.get("p1"), Some(&vec![(1, 1),]));

        params.create_scope();
        params.push_value("p1", 2);
        assert_eq!(params.stacks.get("p1"), Some(&vec![(1, 1), (3, 2)]));

        params.clear_scope();
        assert_eq!(params.stacks.get("p1"), Some(&vec![(1, 1),]));

        params.clear_scope();
        assert_eq!(params.stacks.get("p1"), Some(&vec![(1, 1),]));

        params.clear_scope();
        assert_eq!(params.stacks.get("p1"), Some(&Vec::new()));
    }
}
