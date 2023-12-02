use camino::Utf8PathBuf;

use crate::template::Template;

#[derive(Debug)]
pub(crate) struct App<'templates, 'source> {
    is_running: bool,

    screen: Screen,

    recurse: usize,
    source_directory: Option<Utf8PathBuf>,
    target_directory: Option<Utf8PathBuf>,
    template: Option<Template<'templates, 'source>>,
    arguments: Vec<String>,
}

impl<'templates, 'source> Default for App<'templates, 'source> {
    fn default() -> Self {
        Self {
            is_running: true,
            screen: Screen::Main,
            recurse: 0,
            source_directory: None,
            target_directory: None,
            template: None,
            arguments: Vec::new(),
        }
    }
}

impl<'templates, 'source> App<'templates, 'source> {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.is_running = false;
    }

    pub(crate) fn screen(&self) -> Screen {
        self.screen
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub(crate) enum Screen {
    #[default]
    Main,
    DirSelect,
}

impl Screen {
    pub(crate) fn title(self) -> String {
        let prefix = env!("CARGO_PKG_NAME").to_owned();

        let name = match self {
            Screen::Main => prefix,
            Screen::DirSelect => format!("{prefix} - Pick folder"),
        };

        format!(" {name} ")
    }
}
