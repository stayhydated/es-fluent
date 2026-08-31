use super::*;
use crate::test_fixtures as tempfile;
use std::fs;

fn inspect_fixture(
    files: &[(&str, &str)],
    entry: &str,
    target: SourceTarget<'_>,
) -> InspectionOutcome {
    let temp = tempfile::tempdir().expect("tempdir");
    for (path, source) in files {
        let path = temp.path().join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(path, source).expect("write source");
    }
    inspect(&temp.path().join(entry), temp.path(), target)
}

fn inspect_fixture_with_roots(
    files: &[(&str, &str)],
    entry: &str,
    target: &'static str,
    expected_roots: &[&str],
) -> InspectionOutcome {
    let expected_roots = expected_roots
        .iter()
        .map(|root| (*root).to_string())
        .collect::<Vec<_>>();
    inspect_fixture(
        files,
        entry,
        SourceTarget::Macro(target, Some(&expected_roots)),
    )
}

mod calls;
mod graph;
mod macros;
mod reachability;
