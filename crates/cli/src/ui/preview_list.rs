use std::fmt::Write;
use std::str::FromStr;

use color_eyre::Result;

use super::ItemName;

#[derive(Debug, PartialEq, Eq)]
pub struct ParsePreviewListSizeError;

impl std::fmt::Display for ParsePreviewListSizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Valid options: small, medium, large")
    }
}

impl std::error::Error for ParsePreviewListSizeError {}

#[derive(Debug, Clone, Copy)]
pub enum PreviewListSize {
    Small,
    Medium,
    Large,
}

impl PreviewListSize {
    pub fn new(columns: usize) -> Self {
        match columns {
            0..24 => Self::Small,
            24..36 => Self::Medium,
            36.. => Self::Large,
        }
    }

    fn preview_amount(self) -> usize {
        match self {
            PreviewListSize::Small => 8,
            PreviewListSize::Medium => 12,
            PreviewListSize::Large => 16,
        }
    }
}

impl FromStr for PreviewListSize {
    type Err = ParsePreviewListSizeError;

    fn from_str(s: &str) -> Result<Self, ParsePreviewListSizeError> {
        match s.to_lowercase().as_str() {
            "small" | "sm" | "s" => Ok(Self::Small),
            "medium" | "md" | "m" => Ok(Self::Medium),
            "large" | "lg" | "l" => Ok(Self::Large),
            _ => Err(ParsePreviewListSizeError),
        }
    }
}

pub struct PreviewList<'s, S, I>
where
    S: ToString,
    I: ExactSizeIterator<Item = S>,
{
    iter: I,
    item_name: ItemName<'s>,
    list_size: PreviewListSize,
}

impl<'s, S, I> PreviewList<'s, S, I>
where
    S: ToString,
    I: ExactSizeIterator<Item = S>,
{
    pub fn new(iter: I, list_size: PreviewListSize) -> Self {
        Self { iter, item_name: ItemName::default(), list_size }
    }

    pub fn with_item_name(mut self, item_name: ItemName<'s>) -> Self {
        self.item_name = item_name;
        self
    }

    fn preview_amount(&self) -> usize {
        self.list_size.preview_amount()
    }

    fn total(&self) -> usize {
        self.iter.len()
    }

    fn can_preview_all(&self) -> bool {
        self.total() <= self.preview_amount()
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
