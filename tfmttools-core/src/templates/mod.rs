pub const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

mod context;
mod loader;
mod template;

pub use loader::TemplateLoader;
pub use template::Template;
