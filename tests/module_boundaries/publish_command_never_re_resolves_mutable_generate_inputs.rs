use super::common::*;

#[test]
fn publish_command_never_re_resolves_mutable_generate_inputs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cli = cli_sources(root);
    let commands = commands_source(root);
    let publish = production_source(&root.join("src/publish.rs"));

    let publish_args = cli
        .split("pub struct PublishArgs")
        .nth(1)
        .and_then(|rest| rest.split("// ── Report").next())
        .expect("src/cli/* should define PublishArgs");
    for required_publish_arg in ["pub dry_run: bool", "pub force: bool"] {
        assert!(
            publish_args.contains(required_publish_arg),
            "publish CLI should expose only publish-local controls; missing {required_publish_arg}"
        );
    }
    for forbidden_override in [
        "profile", "base", "upstream", "feature", "remove", "preserve", "arch", "matrix",
        "strict", "candidate",
    ] {
        assert!(
            !publish_args.contains(forbidden_override),
            "publish CLI must not accept generate/profile/candidate override {forbidden_override}"
        );
    }

    let cmd_publish = commands
        .split("fn cmd_publish")
        .nth(1)
        .and_then(|rest| rest.split("fn cmd_report").next())
        .expect("src/commands/* should define cmd_publish");
    for required_publish_wiring in [
        "publish::load_publish_request(root.as_std_path())?",
        "PublishOptions",
        "publish::publish(&request, &opts)?",
    ] {
        assert!(
            cmd_publish.contains(required_publish_wiring),
            "cmd_publish should delegate to publish-only request loading; missing {required_publish_wiring}"
        );
    }

    let publish_request_fields = publish
        .split("pub struct PublishRequest {")
        .nth(1)
        .and_then(|rest| rest.split("}\n").next())
        .expect("publish.rs should define PublishRequest");
    for required_field in [
        "pub project_root: PathBuf",
        "pub output_path: String",
        "pub remote_name: String",
        "pub remote: String",
    ] {
        assert!(
            publish_request_fields.contains(required_field),
            "PublishRequest should contain only committed-publish inputs; missing {required_field}"
        );
    }
    for forbidden_field in [
        "upstream",
        "profile",
        "base",
        "feature",
        "candidate",
        "cli_override",
        "selected_profile",
        "mode",
        "branch_prefix",
    ] {
        assert!(
            !publish_request_fields.contains(forbidden_field),
            "PublishRequest must not carry mutable generate input {forbidden_field}"
        );
    }

    let publish_only_config = publish
        .split("struct PublishOnlyConfig")
        .nth(1)
        .and_then(|rest| rest.split("fn default_publish_remote_name").next())
        .expect("publish.rs should define publish-only config shapes");
    for required_config_field in [
        "output: PublishOnlyOutputConfig",
        "git: PublishOnlyGitConfig",
        "publish: Option<PublishOnlyRemoteConfig>",
    ] {
        assert!(
            publish_only_config.contains(required_config_field),
            "publish-only config should parse only output/git/publish fields; missing {required_config_field}"
        );
    }
    for forbidden_config_field in [
        "upstream",
        "profile",
        "base",
        "features",
        "slim",
        "reducer",
        "selftests",
        "matrix",
        "branch_prefix",
    ] {
        assert!(
            !publish_only_config.contains(forbidden_config_field),
            "publish-only config must not parse generate/profile state field {forbidden_config_field}"
        );
    }

    let publish_path = format!("{cmd_publish}\n{publish}");
    for forbidden_re_resolution in [
        "load_kslim_config",
        "load_kslim_config_file",
        "load_profile",
        "require_known_profile",
        "config::",
        "GenerateOptions",
        "GeneratePlan",
        "resolve_candidate_plan",
        "ResolvedCandidateState",
        "CandidateMetadata",
        "CandidateTree",
        "cli_overrides",
        "selected_profile",
        "crate::upstream",
        "upstream::",
        "resolve_ref",
        "check_access",
        "ref_timestamp",
        "require_local_upstream_url",
        "git::fetch",
        "\"fetch\"",
        "\"ls-remote\"",
        "ls_remote",
    ] {
        assert!(
            !publish_path.contains(forbidden_re_resolution),
            "publish path must not re-resolve upstream, network, profile, CLI override, or candidate state; found {forbidden_re_resolution}"
        );
    }
}
