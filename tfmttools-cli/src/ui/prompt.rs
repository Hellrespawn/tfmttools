use std::io::{BufRead, Write};

use color_eyre::Result;

pub struct ConfirmationPrompt<'cp> {
    prompt: &'cp str,
    default: Option<bool>,
}

impl<'cp> ConfirmationPrompt<'cp> {
    const REQUIRED_PROMPT: &'static str = "Please enter 'y' or 'n'";

    pub fn new(prompt: &'cp str) -> Self {
        Self { prompt, default: Some(false) }
    }

    pub fn prompt(&self) -> Result<bool> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        let options = self.get_options();

        print!("{} {} ", self.prompt, options);
        stdout.flush()?;

        loop {
            let mut input = String::new();

            stdin.lock().read_line(&mut input)?;

            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                "" if self.default.is_some() => {
                    return Ok(self.default.unwrap());
                },
                _ => {
                    print!("{} {} ", Self::REQUIRED_PROMPT, options);
                    stdout.flush()?;
                },
            }
        }
    }

    fn get_options(&self) -> &'static str {
        match self.default {
            Some(true) => "(Y/n)",
            Some(false) => "(y/N)",
            None => "(y/n)",
        }
    }
}
