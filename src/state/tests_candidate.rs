use super::*;

#[test]
fn test_candidate_tree_state_tracks_candidate_lifecycle_flags() {
    let mut state = CandidateTreeState::from_materialized_tree("/tmp/kslim-candidate").unwrap();

    assert_eq!(state.tree.as_path(), Path::new("/tmp/kslim-candidate"));
    assert_eq!(
        state.metadata_dir.as_path(),
        Path::new("/tmp/kslim-candidate/.kslim")
    );
    assert!(state.materialized);
    assert!(!state.integrated);
    assert!(!state.pruned);
    assert!(!state.reduced);
    assert!(!state.selftested);

    state.mark_integrated().unwrap();
    state.mark_pruned().unwrap();
    state.mark_reduced().unwrap();
    state.mark_selftested().unwrap();

    assert!(state.integrated);
    assert!(state.pruned);
    assert!(state.reduced);
    assert!(state.selftested);
}

#[test]
fn test_candidate_tree_state_rejects_unmaterialized_progress() {
    let err = CandidateTreeState::new(
        CandidateTreePath::new("/tmp/candidate").unwrap(),
        CandidateMetadataDir::new("/tmp/candidate/.kslim").unwrap(),
        false,
        true,
        false,
        false,
        false,
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("cannot advance before materialization"));

    let mut state = CandidateTreeState::new(
        CandidateTreePath::new("/tmp/candidate").unwrap(),
        CandidateMetadataDir::new("/tmp/candidate/.kslim").unwrap(),
        false,
        false,
        false,
        false,
        false,
    )
    .unwrap();
    let err = state.mark_reduced().unwrap_err().to_string();
    assert!(err.contains("cannot be marked reduced before materialization"));
}

#[test]
fn test_candidate_tree_state_rejects_metadata_outside_tree() {
    let err = CandidateTreeState::new(
        CandidateTreePath::new("/tmp/candidate").unwrap(),
        CandidateMetadataDir::new("/tmp/other/.kslim").unwrap(),
        true,
        false,
        false,
        false,
        false,
    )
    .unwrap_err()
    .to_string();

    assert!(err.contains("candidate metadata dir is not the candidate tree metadata dir"));
    assert!(err.contains("/tmp/other/.kslim"));
    assert!(err.contains("/tmp/candidate/.kslim"));
}

