use std::fmt::Write;

use color_eyre::Result;

use super::ItemName;
use crate::TERM;

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

    pub fn with_leading(mut self, leading_lines: usize) -> Self {
        self.leading_lines = leading_lines;
        self
    }

    pub fn with_trailing(mut self, trailing_lines: usize) -> Self {
        self.trailing_lines = trailing_lines;
        self
    }

    pub fn with_item_name(mut self, item_name: ItemName<'s>) -> Self {
        self.item_name = item_name;
        self
    }

    fn padding(&self) -> usize {
        self.leading_lines + self.trailing_lines
    }

    pub fn total(&self) -> usize {
        self.iter.len()
    }

    pub fn can_preview_all(&self) -> bool {
        self.total() <= self.preview_amount()
    }

    pub fn preview_amount(&self) -> usize {
        std::cmp::max(
            Self::MIN_PREVIEW_AMOUNT,
            TERM.size().0 as usize - self.padding(),
        )
    }

    pub fn title(&self) -> String {
        let preview_amount = self.preview_amount();
        let total = self.total();

        if self.can_preview_all() {
            format!("Previewing {total} {}", self.item_name.by_amount(total))
        } else {
            format!(
                "Previewing {preview_amount} of {total} {}",
                self.item_name.by_amount(total)
            )
        }
    }

    pub fn into_string(self) -> Result<String> {
        let preview_amount = self.preview_amount();

        let total = self.total();

        let indices = if self.can_preview_all() {
            (0..total).collect()
        } else {
            rounded_linear_space(0, total, preview_amount)?
        };

        let iter = self
            .iter
            .enumerate()
            .filter(|(index, _)| indices.contains(index))
            .map(|(index, item)| (index + 1, item));

        let enumeration_width = total.to_string().len();

        let mut string = String::new();

        for (index, item) in iter {
            writeln!(
                string,
                "{:>enumeration_width$}) {}",
                index,
                item.to_string()
            )
            .expect("writeln! into mut string should never fail");
        }

        Ok(string)
    }

    pub fn print(self) -> Result<()> {
        println!("{}:\n{}", self.title(), self.into_string()?);

        Ok(())
    }
}

fn rounded_linear_space(
    start: usize,
    end: usize,
    amount: usize,
) -> Result<Vec<usize>> {
    let start: f32 = u16::try_from(start)?.into();
    let end: f32 = u16::try_from(end)?.into();

    let size: f32 = (u16::try_from(amount)? - 1).into();
    let dx = (end - start) / size;

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    Ok((1..=amount)
        .scan(-dx, |a, _| {
            *a += dx;
            Some(*a)
        })
        .map(|f| f.round() as usize)
        .collect())
}
