use tfmttools_core::history::history_schema_sql;
#[test]
fn schema_matches_snapshot() {
    assert_eq!(
        history_schema_sql(),
        include_str!("../../../docs/history/schema-v1.sql")
    );
}
