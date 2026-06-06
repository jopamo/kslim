use super::*;

fn removed_set<'a>(removed: &'a [&'a str]) -> HashSet<&'a str> {
    removed.iter().copied().collect()
}

fn source_removal_proof(line: usize, source: &str) -> KconfigSourceRemovalProof {
    KconfigSourceRemovalProof {
        file: PathBuf::from("Kconfig"),
        line,
        source: source.to_string(),
        optional: false,
        relative: false,
        removed_target: PathBuf::from(source),
    }
}

fn selected_profile_values(values: &[(&str, &str)]) -> BTreeMap<String, String> {
    values
        .iter()
        .map(|(symbol, value)| (symbol.to_string(), value.to_string()))
        .collect()
}

#[path = "tests_rewrite.rs"]
mod rewrite;
#[path = "tests_solver.rs"]
mod solver;
#[path = "tests_report.rs"]
mod report;
#[path = "tests_root_facade.rs"]
mod root_facade;
