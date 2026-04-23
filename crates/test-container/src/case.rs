use tfmttools_test_harness::CaseOutcome;

#[derive(Debug, Clone)]
pub struct ContainerCase {
    name: String,
    description: String,
}

impl ContainerCase {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

pub fn run_case(case: &ContainerCase) -> CaseOutcome {
    CaseOutcome::new(
        case.name().to_owned(),
        case.description().to_owned(),
        0,
        Vec::new(),
        None,
    )
}
