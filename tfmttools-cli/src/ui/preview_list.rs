use crate::TERM;

pub struct ItemName<'i> {
    single: &'i str,
    plural: Option<&'i str>,
}

impl<'i> ItemName<'i> {
    pub fn simple(single: &'i str) -> Self {
        Self { single, plural: None }
    }

    #[allow(dead_code)]
    pub fn new(single: &'i str, plural: &'i str) -> Self {
        Self { single, plural: Some(plural) }
    }

    pub fn single(&self) -> String {
        self.single.to_owned()
    }

    pub fn plural(&self) -> String {
        self.plural.map_or(
            format!("{}s", self.single()),
            std::borrow::ToOwned::to_owned,
        )
    }

    pub fn by_amount(&self, amount: usize) -> String {
        if amount == 1 {
            self.single()
        } else {
            self.plural()
        }
    }
}

impl<'i> Default for ItemName<'i> {
    fn default() -> Self {
        Self { single: "item", plural: None }
    }
}

pub struct PreviewList<'s, S, I>
where
    S: ToString,
    I: ExactSizeIterator<Item = S>,
{
    iter: I,
    item_name: ItemName<'s>,
    leading_lines: usize,
    trailing_lines: usize,
}

impl<'s, S, I> PreviewList<'s, S, I>
where
    S: ToString,
    I: ExactSizeIterator<Item = S>,
{
    const MIN_PREVIEW_AMOUNT: usize = 8;

    pub fn new(iter: I) -> Self {
        Self {
            iter,
            item_name: ItemName::default(),
            leading_lines: 0,
            trailing_lines: 0,
        }
    }

    pub fn leading(mut self, leading_lines: usize) -> Self {
        self.leading_lines = leading_lines;
        self
    }

    pub fn trailing(mut self, trailing_lines: usize) -> Self {
        self.trailing_lines = trailing_lines;
        self
    }

    pub fn item_name(mut self, item_name: ItemName<'s>) -> Self {
        self.item_name = item_name;
        self
    }

    pub fn print(self) {
        let padding = self.leading_lines + self.trailing_lines;

        let preview_amount = std::cmp::max(
            Self::MIN_PREVIEW_AMOUNT,
            TERM.size().0 as usize
                - self.leading_lines
                - self.trailing_lines
                - padding,
        );

        let total = self.iter.len();

        if total > preview_amount {
            println!(
                "Previewing {preview_amount} of {total} {}:",
                self.item_name.by_amount(total)
            );
        } else {
            println!("Previewing {total} {}:", self.item_name.by_amount(total));
        };

        let step = total.div_ceil(preview_amount);

        let iter = self
            .iter
            .enumerate()
            .map(|(index, item)| (index + 1, item))
            .step_by(step)
            .take(preview_amount);

        let enumeration_width = total.to_string().len();

        for (index, item) in iter {
            print!("{index:>enumeration_width$}) ");

            println!("{}", item.to_string());
        }
    }
}
