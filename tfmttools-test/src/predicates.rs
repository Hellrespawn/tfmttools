use camino::{Utf8Path, Utf8PathBuf};

use crate::test_case::TestType;

pub struct PredicateInput<'pi, I>
where
    I: Iterator<Item = (&'pi String, &'pi Option<String>)>,
{
    source_dir: Utf8PathBuf,
    target_dir: Utf8PathBuf,
    reference: I,
    test_type: TestType,
}

impl<'pi, I> PredicateInput<'pi, I>
where
    I: Iterator<Item = (&'pi String, &'pi Option<String>)>,
{
    pub fn new(
        source_dir: Utf8PathBuf,
        target_dir: Utf8PathBuf,
        reference: I,
        test_type: TestType,
    ) -> Self {
        Self { source_dir, target_dir, reference, test_type }
    }
}

pub struct PredicateResults(pub Vec<PredicateResult>);

impl PredicateResults {
    pub fn is_error(&self) -> bool {
        self.0.iter().any(PredicateResult::is_error)
    }
}

impl std::fmt::Display for PredicateResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let errors =
            self.0.iter().filter(|pr| pr.is_error()).collect::<Vec<_>>();

        for (i, result) in errors.iter().enumerate() {
            write!(f, "{result}")?;

            if i != errors.len() - 1 {
                writeln!(f)?;
            }
        }

        Ok(())
    }
}

pub struct PredicateResult {
    pub source: Utf8PathBuf,
    pub target: Option<Utf8PathBuf>,
    pub failed_source_predicate: bool,
    pub failed_target_predicate: bool,
}

impl PredicateResult {
    pub fn is_error(&self) -> bool {
        self.failed_source_predicate || self.failed_target_predicate
    }
}

impl std::fmt::Display for PredicateResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.failed_source_predicate {
            write!(f, "{}: source failed predicate", self.source)?
        }

        if self.failed_source_predicate && self.failed_target_predicate {
            writeln!(f)?;
        }

        if self.failed_target_predicate {
            write!(
                f,
                "{}: target failed predicate",
                self.target
                    .as_ref()
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "None".to_owned())
            )?
        }

        Ok(())
    }
}

pub fn check_reference<'pi, I>(
    input: PredicateInput<'pi, I>,
) -> PredicateResults
where
    I: Iterator<Item = (&'pi String, &'pi Option<String>)>,
{
    let results = input
        .reference
        .map(|(source_name, target_name)| {
            let source = input.source_dir.join(source_name);
            let target = target_name
                .as_ref()
                .map(|target_name| input.target_dir.join(target_name));

            PredicateResult {
                failed_source_predicate: !check_source(
                    &source,
                    target.as_deref(),
                    input.test_type,
                ),
                failed_target_predicate: !check_target(
                    &source,
                    target.as_deref(),
                    input.test_type,
                ),
                source,
                target,
            }
        })
        .collect();

    PredicateResults(results)
}

fn check_source(
    source: &Utf8Path,
    target: Option<&Utf8Path>,
    test_type: TestType,
) -> bool {
    match test_type {
        TestType::Apply | TestType::Redo | TestType::PreviousData => {
            !source.exists()
        },
        TestType::Undo => target.is_none() || source.is_file(),
    }
}

fn check_target(
    _source: &Utf8Path,
    target: Option<&Utf8Path>,
    test_type: TestType,
) -> bool {
    match test_type {
        TestType::Apply | TestType::Redo | TestType::PreviousData => {
            target.is_none_or(|p| p.is_file())
        },
        TestType::Undo => target.is_none_or(|p| !p.exists()),
    }
}
