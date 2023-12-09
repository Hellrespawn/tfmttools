pub fn rows_required_for_string(string: &str, width: usize) -> usize {
    string.lines().fold(0, |acc, el| {
        acc + console::measure_text_width(el).div_ceil(width)
    })
}
