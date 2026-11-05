use std::path::Path;
use std::process::ExitCode;

use tfmttools_picotmpl::Template;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [path] = args.as_slice() else {
        eprintln!("Usage: dump_ast <template-file>");
        return ExitCode::FAILURE;
    };
    match dump(Path::new(path)) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        },
    }
}

fn dump(path: &Path) -> Result<String, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let template = Template::compile(&source).map_err(|error| {
        let (line, column) = error.line_column(&source);
        format!("{}:{line}:{column}: {}", path.display(), error.message)
    })?;
    Ok(template.dump_ast())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::dump;

    #[test]
    fn dumps_nested_template_ast() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stef.tfmt");
        let output = dump(&path).unwrap();
        assert!(output.contains("\n"));
        assert!(output.contains("  Guard $album\n"));
        assert!(output.contains("    Interpolation $album\n"));
    }

    #[test]
    fn reports_template_errors_with_file_and_location() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let error = dump(&path).unwrap_err();
        assert!(error.starts_with(&format!("{}:1:1:", path.display())));
    }

    #[test]
    fn reports_file_errors_with_path() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/nonexistent.tfmt");
        let error = dump(&path).unwrap_err();
        assert!(error.starts_with(&format!("{}:", path.display())));
    }
}
