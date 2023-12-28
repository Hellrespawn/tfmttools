use crate::TERM;

pub struct PreviewList<S, I>
where
    S: ToString,
    I: Iterator<Item = S>,
{
    iter: I,
    total: usize,
    leading_lines: usize,
    trailing_lines: usize,
}

impl<S, I> PreviewList<S, I>
where
    S: ToString,
    I: Iterator<Item = S>,
{
    const MIN_PREVIEW_AMOUNT: usize = 8;

    pub fn new(
        iter: I,
        total: usize,
        leading_lines: usize,
        trailing_lines: usize,
    ) -> Self {
        Self { iter, total, leading_lines, trailing_lines }
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

        if self.total > preview_amount {
            println!("Previewing {} of {} items:", preview_amount, self.total);
        } else {
            println!("Previewing {} items:", self.total);
        };

        let step = self.total.div_ceil(preview_amount);

        let iter = self
            .iter
            .enumerate()
            .map(|(index, item)| (index + 1, item))
            .step_by(step)
            .take(preview_amount);

        let enumeration_width = self.total.to_string().len();

        for (index, item) in iter {
            print!("{index:>enumeration_width$}) ");

            println!("{}", item.to_string());
        }
    }
}
