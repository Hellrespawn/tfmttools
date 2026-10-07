mod context;
mod frontmatter;
mod source;
mod template;

pub use frontmatter::{ArgKind, ArgSpec, Frontmatter};
pub use source::parse_template_source;
pub use template::Template;

mod compiled;
pub use compiled::{BoundTemplate, CompiledTemplate};

mod audio_context;
