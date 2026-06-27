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

#[test]
fn kconfig_files_skips_generator_template_directories() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    std::fs::write(root.join("Kconfig"), "config ROOT\n\tbool \"Root\"\n").unwrap();
    std::fs::create_dir_all(root.join("tools/verification/rvgen/rvgen/templates")).unwrap();
    std::fs::write(
        root.join("tools/verification/rvgen/rvgen/templates/Kconfig"),
        "config RV_MON_%%MODEL_NAME_UP%%\n\tbool \"Template\"\n",
    )
    .unwrap();

    let files = kconfig_files(root)
        .into_iter()
        .map(|path| path.strip_prefix(root).unwrap().to_path_buf())
        .collect::<Vec<_>>();

    assert_eq!(files, vec![PathBuf::from("Kconfig")]);
}

#[path = "tests_rewrite.rs"]
mod rewrite;
#[path = "tests_solver.rs"]
mod solver;
#[path = "tests_report.rs"]
mod report;
#[path = "tests_root_facade.rs"]
mod root_facade;
