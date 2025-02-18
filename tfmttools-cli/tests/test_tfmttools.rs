// use color_eyre::Result;
// use tfmttools_test::TestCase;

// #[test]
// fn test_all_cases() -> Result<()> {
//     let cases = TestCase::load_all()?;

//     let results = cases.iter().map(|c| c.run_test()).collect::<Vec<_>>();

//     let mut failed_test_names = Vec::new();

//     for result in results {
//         println!("{result}");

//         if result.is_failure() {
//             failed_test_names.push(result.test_case_name);
//             println!();
//         }
//     }

//     if !failed_test_names.is_empty() {
//         println!(
//             "The following tests have failed:\n  {}",
//             failed_test_names.join("\n  ")
//         );

//         panic!()
//     }

//     Ok(())
// }
