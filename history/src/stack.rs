use serde::{Deserialize, Serialize};

/// A stack which preserves popped items
#[derive(Debug, Serialize, Deserialize)]
pub struct RefStack<T> {
    inner: Vec<T>,
    cursor: usize,
}

impl<T> RefStack<T> {
    pub fn new() -> Self {
        Self { inner: Vec::new(), cursor: 0 }
    }

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

    pub fn pop(&mut self) -> Option<&T> {
        self.popn(1).map(|s| &s[0])
    }

    pub fn popn(&mut self, n: usize) -> Option<&[T]> {
        let start = self.cursor.saturating_sub(n);
        let end = self.cursor;

        let range = start..end;

        if range.is_empty() {
            None
        } else {
            let amount = end - start;

            let items = self.inner.get(start..end);

            self.cursor = self.cursor.saturating_sub(amount);

            items
        }
    }

    pub fn unpop(&mut self) -> Option<&T> {
        self.unpopn(1).map(|s| &s[0])
    }

    pub fn unpopn(&mut self, n: usize) -> Option<&[T]> {
        let start = self.cursor;
        let end = std::cmp::min(self.cursor + n, self.inner.len());

        let range = start..end;

        if range.is_empty() {
            None
        } else {
            let amount = end - start;

            let items = self.inner.get(start..end);

            self.cursor += amount;

            items
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty_ref_stack() {
        let mut stack: RefStack<usize> = RefStack::new();

        assert_eq!(stack.popn(1), None);
        assert_eq!(stack.cursor, 0);
        assert_eq!(stack.popn(3), None);
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.unpopn(1), None);
        assert_eq!(stack.cursor, 0);
        assert_eq!(stack.unpopn(3), None);
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

        assert_eq!(stack.popn(1), Some(&["c"][..]));
        assert_eq!(stack.popn(1), Some(&["b"][..]));

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

        assert_eq!(stack.popn(1), Some(&["c"][..]));
        assert_eq!(stack.popn(1), Some(&["b"][..]));

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 1);

        assert_eq!(stack.unpopn(2), Some(&["b", "c"][..]));
        assert_eq!(stack.cursor, 3);
    }

    #[test]
    fn test_ref_stack_too_big_n() {
        let mut stack = RefStack::new();

        stack.extend(["a", "b", "c"]);

        assert_eq!(stack.unpopn(5), None);
        assert_eq!(stack.cursor, 3);

        assert_eq!(stack.popn(5), Some(&["a", "b", "c"][..]));
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.popn(5), None);
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.unpopn(5), Some(&["a", "b", "c"][..]));
        assert_eq!(stack.cursor, 3);
    }
}
