use serde::{Deserialize, Serialize};

/// A stack which pops references.
///
/// Popped references can be unpopped until a new element is pushed, at which point all elements whose references where popped are discarded. The new element is pushed on top of these.
#[derive(Debug, Serialize, Deserialize)]
pub struct RefStack<T> {
    stack: Vec<T>,
    cursor: usize,
}

impl<T> RefStack<T> {
    /// Create a new `RefStack`
    pub fn new() -> Self {
        Self { stack: Vec::new(), cursor: 0 }
    }

    /// Push a new item onto the stack. This removes any unpopped items.
    pub fn push(&mut self, item: T) {
        self.stack.truncate(self.cursor);
        self.stack.push(item);
        self.cursor = self.stack.len();
    }

    #[cfg(test)]
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.stack.truncate(self.cursor);
        self.stack.extend(iter);
        self.cursor = self.stack.len();
    }

    pub fn get_unpopped_refs(&self) -> &[T] {
        let range = 0..self.cursor;

        if range.is_empty() {
            &[]
        } else {
            let items = self.stack.get(range);

            items.unwrap_or_default()
        }
    }

    pub fn get_popped_refs(&self) -> &[T] {
        let start = self.cursor;
        let end = self.stack.len();

        let range = start..end;

        if range.is_empty() {
            &[]
        } else {
            let items = self.stack.get(start..end);

            items.unwrap_or_default()
        }
    }

    /// Pop up to `n` references
    pub fn pop_refs(&mut self, n: usize) -> &[T] {
        let start = self.cursor.saturating_sub(n);
        let end = self.cursor;

        let range = start..end;

        if range.is_empty() {
            &[]
        } else {
            let amount = end - start;

            let items = self.stack.get(start..end);

            self.cursor = self.cursor.saturating_sub(amount);

            items.unwrap_or_default()
        }
    }

    /// Unpop up to `n` references
    pub fn unpop_refs(&mut self, n: usize) -> &[T] {
        let start = self.cursor;
        let end = std::cmp::min(self.cursor + n, self.stack.len());

        let range = start..end;

        if range.is_empty() {
            &[]
        } else {
            let amount = end - start;

            let items = self.stack.get(start..end);

            self.cursor += amount;

            items.unwrap_or_default()
        }
    }

    pub fn find<P>(&self, predicate: P) -> Option<&T>
    where
        P: Fn(&T) -> bool,
    {
        self.stack.iter().rev().find(|i| predicate(*i))
    }
}

#[cfg(test)]
mod test {
    use crate::stack::RefStack;

    #[test]
    fn test_empty_ref_stack() {
        let mut stack: RefStack<usize> = RefStack::new();

        let empty: &[usize] = &[][..];

        assert_eq!(stack.pop_refs(1), empty);
        assert_eq!(stack.cursor, 0);
        assert_eq!(stack.pop_refs(3), empty);
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.unpop_refs(1), empty);
        assert_eq!(stack.cursor, 0);
        assert_eq!(stack.unpop_refs(3), empty);
        assert_eq!(stack.cursor, 0);
    }

    #[test]
    fn test_ref_stack_before_cursor() {
        let mut stack = RefStack::new();

        stack.push("a");
        stack.push("b");
        stack.push("c");

        assert_eq!(stack.stack, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 3);

        assert_eq!(stack.pop_refs(1), &["c"][..]);
        assert_eq!(stack.pop_refs(1), &["b"][..]);

        assert_eq!(stack.stack, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 1);

        stack.push("d");

        assert_eq!(stack.stack, vec!["a", "d"]);
        assert_eq!(stack.cursor, 2);
    }

    #[test]
    fn test_ref_stack_after_cursor() {
        let mut stack = RefStack::new();

        stack.push("a");
        stack.push("b");
        stack.push("c");

        assert_eq!(stack.stack, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 3);

        assert_eq!(stack.pop_refs(1), &["c"][..]);
        assert_eq!(stack.pop_refs(1), &["b"][..]);

        assert_eq!(stack.stack, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 1);

        assert_eq!(stack.unpop_refs(2), &["b", "c"][..]);
        assert_eq!(stack.cursor, 3);
    }

    #[test]
    fn test_ref_stack_too_big_n() {
        let mut stack = RefStack::new();

        stack.extend(["a", "b", "c"]);

        let empty: &[&str] = &[];

        assert_eq!(stack.unpop_refs(5), empty);
        assert_eq!(stack.cursor, 3);

        assert_eq!(stack.pop_refs(5), &["a", "b", "c"][..]);
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.pop_refs(5), empty);
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.unpop_refs(5), &["a", "b", "c"][..]);
        assert_eq!(stack.cursor, 3);
    }

    #[test]
    fn test_find() {
        let mut stack = RefStack::new();

        stack.extend(["a1", "a2", "b1", "b2"]);

        let option = stack.find(|_| true);

        assert_eq!(
            option,
            Some(&"b2"),
            "Did not retrieve last pushed element."
        );

        let option = stack.find(|i| i.starts_with("a"));

        assert_eq!(
            option,
            Some(&"a2"),
            "Did not retrieve last pushed element matching predicate."
        );
    }
}
