use serde::{Deserialize, Serialize};

/// A stack which pops references.
///
/// Popped references can be unpopped until a new element is pushed, at which point all elements whose references where popped are discarded. The new element is pushed on top of these.
#[derive(Debug, Serialize, Deserialize)]
pub struct RefStack<T> {
    inner: Vec<T>,
    cursor: usize,
}

impl<T> RefStack<T> {
    /// Create a new `RefStack`
    pub fn new() -> Self {
        Self { inner: Vec::new(), cursor: 0 }
    }

    /// Push a new item onto the stack. This removes any unpopped items.
    pub fn push(&mut self, item: T) {
        self.inner.truncate(self.cursor);
        self.inner.push(item);
        self.cursor = self.inner.len();
    }

    #[cfg(test)]
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.inner.truncate(self.cursor);
        self.inner.extend(iter);
        self.cursor = self.inner.len();
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

            let items = self.inner.get(start..end);

            self.cursor = self.cursor.saturating_sub(amount);

            items.unwrap_or_default()
        }
    }

    /// Unpop up to `n` references
    pub fn unpop_refs(&mut self, n: usize) -> &[T] {
        let start = self.cursor;
        let end = std::cmp::min(self.cursor + n, self.inner.len());

        let range = start..end;

        if range.is_empty() {
            &[]
        } else {
            let amount = end - start;

            let items = self.inner.get(start..end);

            self.cursor += amount;

            items.unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty_ref_stack() {
        let mut stack: RefStack<usize> = RefStack::new();

        assert_eq!(stack.pop_refs(1), &[][..]);
        assert_eq!(stack.cursor, 0);
        assert_eq!(stack.pop_refs(3), &[][..]);
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.unpop_refs(1), &[][..]);
        assert_eq!(stack.cursor, 0);
        assert_eq!(stack.unpop_refs(3), &[][..]);
        assert_eq!(stack.cursor, 0);
    }

    #[test]
    fn test_ref_stack_before_cursor() {
        let mut stack = RefStack::new();

        stack.push("a");
        stack.push("b");
        stack.push("c");

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 3);

        assert_eq!(stack.pop_refs(1), &["c"][..]);
        assert_eq!(stack.pop_refs(1), &["b"][..]);

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 1);

        stack.push("d");

        assert_eq!(stack.inner, vec!["a", "d"]);
        assert_eq!(stack.cursor, 2);
    }

    #[test]
    fn test_ref_stack_after_cursor() {
        let mut stack = RefStack::new();

        stack.push("a");
        stack.push("b");
        stack.push("c");

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 3);

        assert_eq!(stack.pop_refs(1), &["c"][..]);
        assert_eq!(stack.pop_refs(1), &["b"][..]);

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
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
}
