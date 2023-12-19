pub const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

mod context;
mod template;
mod templates;

pub use template::Template;
pub use templates::Templates;
