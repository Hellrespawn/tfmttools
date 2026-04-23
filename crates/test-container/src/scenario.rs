use color_eyre::Result;
use color_eyre::eyre::eyre;
use libtest_mimic::Arguments;
use tfmttools_test_harness::FixtureDirs;

use crate::case::{ContainerCase, case_id_from_path};

pub fn discover_cases(args: &Arguments) -> Result<Vec<ContainerCase>> {
    let fixture_dirs = FixtureDirs::container();
    let case_dir = fixture_dirs.case_dir();
    let scenario_dir = fixture_dirs.scenario_dir();

    if !case_dir.exists() {
        return Err(eyre!("container case directory missing at {case_dir}"));
    }

    if !scenario_dir.exists() {
        return Err(eyre!(
            "container scenario directory missing at {scenario_dir}"
        ));
    }

    let mut case_paths = fs_err::read_dir(&case_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter_map(|path| camino::Utf8PathBuf::from_path_buf(path).ok())
        .filter(|path| {
            path.file_name().is_some_and(|name| name.ends_with(".case.json"))
        })
        .collect::<Vec<_>>();
    case_paths.sort();

    if case_paths.is_empty() {
        return Err(eyre!("did not find any container cases at {case_dir}"));
    }

    let mut cases = Vec::new();

    for path in case_paths {
        let case_id = case_id_from_path(&path)?;
        if !matches_filters(&case_id, args) {
            continue;
        }

        let case = ContainerCase::from_file(&path)?;
        let scenario_path =
            scenario_dir.join(format!("{}.scenario.json", case.scenario()));

        if !scenario_path.exists() {
            return Err(eyre!(
                "container case {:?} references missing scenario {}",
                case.id(),
                case.scenario()
            ));
        }

        cases.push(case);
    }

    Ok(cases)
}

fn matches_filters(case_id: &str, args: &Arguments) -> bool {
    let filter_matches = args
        .filter
        .as_ref()
        .is_none_or(|filter| matches_filter(case_id, filter, args.exact));
    let skip_matches =
        args.skip.iter().any(|skip| matches_filter(case_id, skip, args.exact));

    filter_matches && !skip_matches
}

fn matches_filter(case_id: &str, filter: &str, exact: bool) -> bool {
    if exact { case_id == filter } else { case_id.contains(filter) }
}

#[cfg(test)]
mod tests {
    use super::matches_filter;

    #[test]
    fn exact_filters_match_only_full_case_id() {
        assert!(matches_filter("cross-device-nemo", "cross-device-nemo", true));
        assert!(!matches_filter("cross-device-nemo", "cross-device", true));
    }

    #[test]
    fn non_exact_filters_match_substrings() {
        assert!(matches_filter("cross-device-nemo", "device", false));
        assert!(!matches_filter("cross-device-nemo", "redo-only", false));
    }
}
