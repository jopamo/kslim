use crate::cli::{
    BaseCommands, Cli, Commands, CompareArgs, ExplainAbiArgs, ExplainEditArgs, ExplainFeatureArgs,
    ExplainSymbolArgs, FeatureImpactArgs, FuzzFixturesArgs, GenerateArgs, InitArgs, MatrixArgs,
    PlanArgs, PublishArgs, ReduceTreeArgs, ReportArgs, ResolveArgs, SelftestArgs, UpstreamCommands,
    ValidateConfigArgs,
};
use crate::command_render;
use crate::config;
use crate::feature::{FeatureConflictReport, FeatureImpactReport};
use crate::generate::{self, GenerateOptions};
use crate::lockfile::{self, ResolvedBase};
use crate::manifest;
use crate::model::KconfigSymbol;
use crate::output_repo;
use crate::paths::{KernelSourceRoot, LockfilePath, OutputRepoPath, RelativeKernelPath};
use crate::publish::{self, PublishOptions};
use crate::reducer;
use crate::upstream;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;
const ATTEMPT_METADATA_DIR: &str = ".kslim/attempt";
const GENERATE_FAILURE_FILE: &str = "generate-failure.toml";
const FAILURE_REPORT_FILE: &str = "report.txt";
// ── Router ───────────────────────────────────────────────────────────────────
pub(crate) fn run(cli: Cli) -> Result<()> {
    crate::network_policy::configure_cli(cli.no_network, cli.offline);
    if let Some(query) = cli.explain.as_deref() {
        if cli.command.is_some() {
            anyhow::bail!("--explain cannot be combined with a subcommand");
        }
        return cmd_explain(query);
    }
    match cli.command {
        Some(command) => match command {
            Commands::Init(args) => cmd_init(args),
            Commands::ValidateConfig(args) => cmd_validate_config(args),
            Commands::Plan(args) => cmd_plan(args),
            Commands::FeatureImpact(args) => cmd_feature_impact(args),
            Commands::ReduceTree(args) => cmd_reduce_tree(args),
            Commands::Upstream(sub) => match sub {
                UpstreamCommands::Sync => cmd_upstream_sync(),
            },
            Commands::Base(sub) => match sub {
                BaseCommands::Resolve(args) => cmd_base_resolve(args),
            },
            Commands::Generate(args) => cmd_generate(args),
            Commands::Publish(args) => cmd_publish(args),
            Commands::Report(args) => cmd_report(args),
            Commands::Status => cmd_status(),
            Commands::Repair => cmd_repair(),
            Commands::ExplainEdit(args) => cmd_explain_edit(args),
            Commands::ExplainSymbol(args) => cmd_explain_symbol(args),
            Commands::ExplainFeature(args) => cmd_explain_feature(args),
            Commands::ExplainAbi(args) => cmd_explain_abi(args),
            Commands::Matrix(args) => cmd_matrix(args),
            Commands::Selftest(args) => cmd_selftest(args),
            Commands::FuzzFixtures(args) => cmd_fuzz_fixtures(args),
            Commands::Compare(args) => cmd_compare(args),
        },
        None => anyhow::bail!("no command specified; run `kslim --help`"),
    }
}
// ── Command implementations ───────────────────────────────────────────────────
fn cmd_init(args: InitArgs) -> Result<()> {
    let root = std::env::current_dir()?;
    let root: camino::Utf8PathBuf =
        camino::Utf8PathBuf::from_path_buf(root).map_err(|_| anyhow::anyhow!("non-utf8 path"))?;
    if root.join("kslim.toml").exists() {
        anyhow::bail!("kslim project already exists here (kslim.toml found)");
    }
    let mut config = config::default_kslim_config(&args.name, &args.output);
    config.upstream.name = args.upstream_name.clone();
    config.upstream.url = args.upstream_url.clone();
    if let Some(remote) = &args.publish_remote {
        config.publish = Some(config::default_publish_config(remote));
    }
    let config_toml = toml::to_string_pretty(&config)?;
    std::fs::write(root.join("kslim.toml"), config_toml)?;
    crate::fsutil::ensure_dir(std::path::Path::new(&root.join("profiles")))?;
    let profile = config::default_profile_config(&args.base_ref);
    let profile_toml = toml::to_string_pretty(&profile)?;
    std::fs::write(root.join("profiles/default.toml"), profile_toml)?;
    std::fs::write(
        root.join("profiles/amdgpu-prune.toml.example"),
        config::amdgpu_prune_profile_template(&args.base_ref),
    )?;
    crate::fsutil::ensure_dir(std::path::Path::new(&root.join("manifests")))?;
    std::fs::write(root.join("manifests/.gitkeep"), "")?;
    crate::fsutil::ensure_dir(std::path::Path::new(&root.join("docs")))?;
    std::fs::write(
        root.join("docs/kernel-build-iteration.md"),
        config::kernel_build_iteration_guide(),
    )?;
    println!("Initialized kslim project at {}", root);
    println!("  config:   kslim.toml");
    println!("  profile:  profiles/default.toml");
    println!("  example:  profiles/amdgpu-prune.toml.example");
    println!("  manifests: manifests/");
    println!("  docs:     docs/kernel-build-iteration.md");
    Ok(())
}
fn cmd_validate_config(args: ValidateConfigArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let config_path = root.as_std_path().join("kslim.toml");
    let config::LoadedKslimConfig { config, .. } =
        config::load_kslim_config_file_with_source_map(config_path.as_path())
            .with_context(|| format!("failed to validate {}", config_path.display()))?;
    let profiles = if let Some(profile) = args.profile {
        let profile = config::normalize_profile_name(&profile)?;
        config::require_known_profile(root.as_std_path(), &profile)?;
        vec![profile]
    } else {
        config::list_profiles(root.as_std_path())?
    };
    if profiles.is_empty() {
        anyhow::bail!("no profiles found under profiles/");
    }
    println!("config: ok");
    println!("project: {}", config.project.name);
    println!("profiles:");
    for profile_name in profiles {
        let profile = config::load_profile_with_source_map(root.as_std_path(), &profile_name)
            .with_context(|| format!("failed to validate profile '{}'", profile_name))?
            .profile;
        println!("  - {}: ok (base: {})", profile_name, profile.base.r#ref);
    }
    Ok(())
}
fn cmd_plan(args: PlanArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let profile_name = config::normalize_profile_name(&args.profile)?;
    config::require_known_profile(root.as_std_path(), &profile_name)?;
    let frozen_plan = args.frozen_plan.clone();
    let opts = GenerateOptions {
        dry_run: false,
        deep_dry_run: false,
        report_only: false,
        keep_temp: false,
        max_fixup_passes: args.max_fixup_passes,
        matrix: args.matrix,
        offline: crate::network_policy::cli_offline(),
        frozen_plan: None,
        force: false,
        base_ref: args.base,
        feature: args.feature,
        remove_feature: args.remove_feature,
        preserve_feature: args.preserve_feature,
        arch: args.arch,
        primary_arch: args.primary_arch,
        secondary_arch: args.secondary_arch,
        safety: args.safety,
        strict: args.strict,
        no_strict: args.no_strict,
        run_selftests: true,
    };
    let summary = if let Some(path) = frozen_plan {
        generate::write_frozen_plan_for_request(
            root.as_std_path(),
            &profile_name,
            &opts,
            Path::new(&path),
        )?
    } else {
        generate::resolve_plan_summary(root.as_std_path(), &profile_name, opts)?
    };
    command_render::print_plan_summary(&summary);
    Ok(())
}
fn cmd_feature_impact(args: FeatureImpactArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let _config = config::load_kslim_config(root.as_std_path())?;
    let profile_name = config::normalize_profile_name(&args.profile)?;
    config::require_known_profile(root.as_std_path(), &profile_name)?;
    let selection = config::ProfileFeatureSelection::new(
        args.feature.as_deref(),
        args.remove_feature.as_deref(),
        args.preserve_feature.as_deref(),
        args.arch.as_deref(),
        args.primary_arch.as_deref(),
        args.secondary_arch.as_deref(),
        args.safety.as_deref(),
    );
    let feature_filter = selection.selected_feature_name()?;
    let profile = config::select_profile_features(
        config::load_profile(root.as_std_path(), &profile_name)?,
        selection,
    )?;
    let mut profile = profile;
    apply_reducer_strictness_override(&mut profile.reducer, args.strict, args.no_strict)?;
    print_feature_impact(&profile_name, &profile, feature_filter.as_deref())?;
    Ok(())
}
fn apply_reducer_strictness_override(
    reducer: &mut config::ReducerConfig,
    strict: bool,
    no_strict: bool,
) -> Result<()> {
    if strict && no_strict {
        anyhow::bail!("cli --strict and --no-strict are mutually exclusive");
    }
    if strict {
        reducer.enable_strict_mode();
    } else if no_strict {
        reducer.disable_strict_mode();
    }
    Ok(())
}
fn ensure_no_reduce_tree_frozen_overrides(args: &ReduceTreeArgs) -> Result<()> {
    if args.feature.is_some()
        || args.remove_feature.is_some()
        || args.preserve_feature.is_some()
        || args.arch.is_some()
        || args.primary_arch.is_some()
        || args.secondary_arch.is_some()
        || args.safety.is_some()
        || args.max_fixup_passes.is_some()
        || args.matrix.is_some()
        || args.strict
        || args.no_strict
    {
        anyhow::bail!("--frozen-plan cannot be combined with profile-shaping reducer overrides");
    }
    Ok(())
}
fn ensure_no_generate_frozen_overrides(args: &GenerateArgs) -> Result<()> {
    if args.feature.is_some()
        || args.remove_feature.is_some()
        || args.preserve_feature.is_some()
        || args.arch.is_some()
        || args.primary_arch.is_some()
        || args.secondary_arch.is_some()
        || args.safety.is_some()
        || args.max_fixup_passes.is_some()
        || args.matrix.is_some()
        || args.base.is_some()
        || args.strict
        || args.no_strict
        || args.dry_run
        || args.deep_dry_run
        || args.report_only
        || args.reducer_report_only
        || args.force
        || args.no_selftests
    {
        anyhow::bail!("--frozen-plan cannot be combined with generate plan overrides");
    }
    Ok(())
}
fn cmd_reduce_tree(args: ReduceTreeArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let (profile_name, profile, frozen_inputs) = if let Some(path) = args.frozen_plan.as_deref() {
        ensure_no_reduce_tree_frozen_overrides(&args)?;
        let frozen = generate::load_frozen_plan(Path::new(path))?;
        (
            frozen.profile.profile.name.clone(),
            frozen.profile,
            Some(frozen.inputs),
        )
    } else {
        let _config = config::load_kslim_config(root.as_std_path())?;
        let profile_name = config::normalize_profile_name(&args.profile)?;
        config::require_known_profile(root.as_std_path(), &profile_name)?;
        let mut profile = config::select_profile_features(
            config::load_profile(root.as_std_path(), &profile_name)?,
            config::ProfileFeatureSelection::new(
                args.feature.as_deref(),
                args.remove_feature.as_deref(),
                args.preserve_feature.as_deref(),
                args.arch.as_deref(),
                args.primary_arch.as_deref(),
                args.secondary_arch.as_deref(),
                args.safety.as_deref(),
            ),
        )?;
        apply_reducer_strictness_override(&mut profile.reducer, args.strict, args.no_strict)?;
        if let Some(max_fixup_passes) = args.max_fixup_passes {
            profile.reducer.max_fixup_passes = max_fixup_passes;
        }
        crate::matrix::apply_cli_matrix_override(&mut profile, args.matrix.as_deref())?;
        (profile_name, profile, None)
    };
    let tree = required_path_arg("tree", &args.tree)?;
    let kernel_root = KernelSourceRoot::new_existing_dir(tree)?;
    if let Some(inputs) = frozen_inputs.as_ref() {
        generate::ensure_tree_matches_frozen_base(kernel_root.as_path(), inputs)?;
        if let Some(path) = args.frozen_plan.as_deref() {
            command_render::print_frozen_plan_verification(Path::new(path), inputs);
        }
    }
    reject_profile_feature_conflicts_in_strict_mode(&profile)?;
    let result = reducer::run_reducer_for_profile(&kernel_root, &profile)?;
    command_render::print_reduce_tree_result(&profile_name, &kernel_root, &result);
    if !result.publishable {
        anyhow::bail!(
            "reduce-tree stopped with status {}",
            result.status.stable_name()
        );
    }
    Ok(())
}
fn cmd_matrix(args: MatrixArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let _config = config::load_kslim_config(root.as_std_path())?;
    let profile_name = config::normalize_profile_name(&args.profile)?;
    config::require_known_profile(root.as_std_path(), &profile_name)?;
    let mut profile = config::load_profile(root.as_std_path(), &profile_name)?;
    crate::matrix::apply_cli_matrix_override(&mut profile, args.matrix.as_deref())?;
    crate::matrix::print_summary(&profile_name, &profile, args.matrix.as_deref());
    Ok(())
}
fn cmd_selftest(args: SelftestArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let _config = config::load_kslim_config(root.as_std_path())?;
    let profile_name = config::normalize_profile_name(&args.profile)?;
    config::require_known_profile(root.as_std_path(), &profile_name)?;
    let mut profile = config::load_profile(root.as_std_path(), &profile_name)?;
    crate::matrix::apply_cli_matrix_override(&mut profile, args.matrix.as_deref())?;
    let tree = required_path_arg("tree", &args.tree)?;
    let kernel_root = KernelSourceRoot::new_existing_dir(tree)?;
    let tree = kernel_root.as_path().to_string_lossy().into_owned();
    let result = crate::selftest::run_capture(&tree, &profile.selftests)
        .map_err(|err| anyhow::anyhow!("selftest failed: {}", err))?;
    println!("selftest: ok");
    println!("profile: {}", profile_name);
    println!("tree: {}", kernel_root.as_path().display());
    println!("enabled: {}", result.enabled);
    println!("built-in checks: {}", result.built_in_checks);
    println!("kernel builds: {}", result.kernel_builds_run);
    println!("commands: {}", result.commands_run);
    Ok(())
}
fn cmd_fuzz_fixtures(args: FuzzFixturesArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let config = config::load_kslim_config(root.as_std_path())?;
    let result = crate::fuzz_fixtures::write_fixtures(root.as_std_path(), &config, &args.out)?;
    println!("fuzz-fixtures: written");
    println!("dir: {}", result.output_dir.display());
    println!("files: {}", result.files.len());
    for file in result.files {
        println!("  - {}", file);
    }
    Ok(())
}
fn required_path_arg(name: &str, value: &str) -> Result<std::path::PathBuf> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{name} path must not be empty");
    }
    Ok(std::path::PathBuf::from(value))
}
fn require_declared_feature(profile: &config::ProfileConfig, feature: &str) -> Result<()> {
    if profile.has_named_feature(feature) {
        return Ok(());
    }
    anyhow::bail!(
        "feature '{}' is not declared in features.remove or features.preserve",
        feature
    );
}
fn print_feature_impact(
    profile_name: &str,
    profile: &config::ProfileConfig,
    feature_filter: Option<&str>,
) -> Result<()> {
    println!("profile: {}", profile_name);
    println!("feature filter: {}", feature_filter.unwrap_or("<all>"));
    print_direct_slim_impact(profile);
    print_effective_feature_impact(profile);
    print_effective_feature_conflicts(profile)?;
    print_named_features(profile, feature_filter);
    Ok(())
}
fn print_direct_slim_impact(profile: &config::ProfileConfig) {
    let direct = profile.removal_input();
    println!("direct slim:");
    println!(
        "  remove paths: {}",
        direct.map_or(0, |slim| slim.remove_paths.len())
    );
    println!(
        "  remove configs: {}",
        direct.map_or(0, |slim| slim.remove_configs.len())
    );
    println!(
        "  default overrides: {}",
        direct.map_or(0, |slim| slim.set_defaults.len())
    );
}
fn print_effective_feature_impact(profile: &config::ProfileConfig) {
    let impact = FeatureImpactReport::from_profile(profile);
    println!("effective impact:");
    println!("  remove paths: {}", impact.remove_paths());
    println!("  remove configs: {}", impact.remove_configs());
    println!("  default overrides: {}", impact.default_overrides());
    println!("  preserve paths: {}", impact.preserve_paths());
    println!("  preserve configs: {}", impact.preserve_configs());
}
fn print_effective_feature_conflicts(profile: &config::ProfileConfig) -> Result<()> {
    let conflicts = FeatureConflictReport::from_profile(profile)?;
    command_render::print_feature_conflicts(&conflicts);
    Ok(())
}
fn reject_profile_feature_conflicts_in_strict_mode(profile: &config::ProfileConfig) -> Result<()> {
    FeatureConflictReport::from_profile(profile)?
        .reject_blocking_conflicts_in_strict_mode(profile.reducer.strict_mode())
}
fn print_named_features(profile: &config::ProfileConfig, feature_filter: Option<&str>) {
    println!("named features:");
    let mut printed = false;
    for (name, intent) in &profile.features.remove {
        if should_print_feature(feature_filter, name) {
            print_feature_intent(name, "remove", intent);
            printed = true;
        }
    }
    for (name, intent) in &profile.features.preserve {
        if should_print_feature(feature_filter, name) {
            print_feature_intent(name, "preserve", intent);
            printed = true;
        }
    }

    if !printed {
        println!("  (none)");
    }
}

fn should_print_feature(feature_filter: Option<&str>, name: &str) -> bool {
    match feature_filter {
        Some(feature) => feature == name,
        None => true,
    }
}

fn print_feature_intent(name: &str, mode: &str, intent: &config::FeatureIntentConfig) {
    println!("  - {} ({})", name, mode);
    println!(
        "    kind: {}",
        intent.kind.as_deref().unwrap_or("(unspecified)")
    );
    println!("    roots: {}", comma_list(&intent.roots));
    println!("    configs: {}", comma_list(&intent.configs));
    println!("    remove paths: {}", comma_list(&intent.remove_paths));
    println!("    remove configs: {}", comma_list(&intent.remove_configs));
    println!("    arch scope: {}", comma_list(&intent.arch_scope));
    println!("    safety: {}", intent.safety.unwrap_or_default().as_str());
    println!(
        "    allow public headers: {}",
        intent.allow_public_header_removal
    );
    println!(
        "    allow uapi headers: {}",
        intent.allow_uapi_header_removal
    );
    println!("    require clean boot: {}", intent.require_clean_boot);
    println!("    report only: {}", intent.report_only);
}

fn comma_list(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".to_string()
    } else {
        values.join(", ")
    }
}

fn cmd_upstream_sync() -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let config = config::load_kslim_config(root.as_std_path())?;

    upstream::sync(&config.upstream.name, &config.upstream.url)?;
    println!(
        "Upstream '{}' verified successfully in direct read-only mode",
        config.upstream.name
    );
    Ok(())
}

fn cmd_base_resolve(args: ResolveArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let config = config::load_kslim_config(root.as_std_path())?;
    let profile_name = config::normalize_profile_name(&args.profile)?;
    config::require_known_profile(root.as_std_path(), &profile_name)?;
    let profile = config::load_profile(root.as_std_path(), &profile_name)?;
    let lockfile_path = LockfilePath::new_in_project_root(root.as_std_path())?;
    if crate::network_policy::cli_offline() {
        crate::network_policy::require_cli_no_network_endpoint(
            "upstream.url",
            &config.upstream.url,
        )?;
        let resolved = lockfile::load_resolved_base_for_request(
            &lockfile_path,
            &config.upstream.name,
            &config.upstream.url,
            &profile.base.r#ref,
        )?;
        println!("profile:      {}", profile_name);
        println!("upstream:     {}", resolved.url);
        println!("base ref:     {}", resolved.r#ref);
        println!("base commit:  {}", resolved.commit);
        println!("resolved at:  {}", resolved.resolved_at);
        return Ok(());
    }

    let upstream_path = upstream::check_access(&config.upstream.url)
        .context("upstream is not accessible. Fix upstream.url or run `kslim upstream sync`.")?;

    let commit = upstream::resolve_ref(upstream_path.as_str(), &profile.base.r#ref)?;
    let resolved_at = upstream::ref_timestamp(upstream_path.as_str(), &profile.base.r#ref)
        .with_context(|| {
            format!(
                "failed to read reproducible timestamp for {}",
                profile.base.r#ref
            )
        })?;

    let resolved = ResolvedBase {
        upstream: config.upstream.name.clone(),
        url: config.upstream.url.clone(),
        r#ref: profile.base.r#ref.clone(),
        commit: commit.clone(),
        resolved_at: resolved_at.clone(),
    };

    let lockfile_update = lockfile::ResolvedBaseLockfileUpdate::new(resolved);
    lockfile::write_resolved_base_lockfile(&lockfile_path, &lockfile_update)?;

    println!("profile:      {}", profile_name);
    println!("upstream:     {}", config.upstream.url);
    println!("base ref:     {}", profile.base.r#ref);
    println!("base commit:  {}", commit);
    println!("resolved at:  {}", resolved_at);

    Ok(())
}

fn cmd_generate(args: GenerateArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    if let Some(path) = args.frozen_plan.as_deref() {
        ensure_no_generate_frozen_overrides(&args)?;
        let frozen = generate::load_frozen_plan(Path::new(path))?;
        command_render::print_frozen_plan_verification(Path::new(path), &frozen.inputs);
        let source_maps = frozen.inputs.source_maps.clone();
        let opts = frozen.inputs.to_generate_options(args.keep_temp);
        let result = generate::generate_with_source_maps(
            &frozen.config,
            &frozen.profile,
            &opts,
            Some(source_maps),
        )?;
        return command_render::print_generate_result(&opts, result);
    }
    let config_path = root.as_std_path().join("kslim.toml");
    let config::LoadedKslimConfig {
        config,
        source_map: config_source_map,
    } = config::load_kslim_config_file_with_source_map(config_path.as_path())?;
    let profile_name = config::normalize_profile_name(&args.profile)?;
    config::require_known_profile(root.as_std_path(), &profile_name)?;
    let config::LoadedProfileConfig {
        profile,
        source_map: profile_source_map,
    } = config::load_profile_with_source_map(root.as_std_path(), &profile_name)?;

    let opts = GenerateOptions {
        dry_run: args.dry_run,
        deep_dry_run: args.deep_dry_run,
        report_only: args.report_only || args.reducer_report_only,
        keep_temp: args.keep_temp,
        max_fixup_passes: args.max_fixup_passes,
        matrix: args.matrix,
        offline: crate::network_policy::cli_offline(),
        frozen_plan: None,
        force: args.force,
        base_ref: args.base,
        feature: args.feature,
        remove_feature: args.remove_feature,
        preserve_feature: args.preserve_feature,
        arch: args.arch,
        primary_arch: args.primary_arch,
        secondary_arch: args.secondary_arch,
        safety: args.safety,
        strict: args.strict,
        no_strict: args.no_strict,
        run_selftests: !args.no_selftests,
    };
    let mut override_source_map = config::ConfigSourceMap::default();
    if opts.base_ref.is_some() {
        override_source_map.insert_cli_override("base.ref", "cli --base");
    }
    if opts.max_fixup_passes.is_some() {
        override_source_map
            .insert_cli_override("reducer.max_fixup_passes", "cli --max-fixup-passes");
    }
    if opts.matrix.is_some() {
        override_source_map.insert_cli_override("selftests.matrix", "cli --matrix");
    }
    config::insert_profile_feature_selection_cli_overrides(
        &mut override_source_map,
        config::ProfileFeatureSelection::new(
            opts.feature.as_deref(),
            opts.remove_feature.as_deref(),
            opts.preserve_feature.as_deref(),
            opts.arch.as_deref(),
            opts.primary_arch.as_deref(),
            opts.secondary_arch.as_deref(),
            opts.safety.as_deref(),
        ),
    );
    if opts.strict {
        config::insert_profile_strictness_cli_overrides(&mut override_source_map, "cli --strict");
    } else if opts.no_strict {
        config::insert_profile_strictness_cli_overrides(
            &mut override_source_map,
            "cli --no-strict",
        );
    }
    let source_maps = generate::GeneratePlanSourceMaps::new(
        config_source_map,
        profile_source_map,
        override_source_map,
    );

    let result = generate::generate_with_source_maps(&config, &profile, &opts, Some(source_maps))?;
    command_render::print_generate_result(&opts, result)
}

fn cmd_publish(args: PublishArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let request = publish::load_publish_request(root.as_std_path())?;

    let opts = PublishOptions {
        dry_run: args.dry_run,
        force: args.force,
        no_network: crate::network_policy::cli_no_network(),
    };

    publish::publish(&request, &opts)?;

    if args.dry_run {
        println!("Dry run complete");
    } else {
        println!("Published successfully");
    }

    Ok(())
}

fn cmd_report(args: ReportArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let output_path = load_report_output_path(root.as_std_path())?;
    let published_path =
        output_repo::reducer_artifact_path(Path::new(&output_path), &args.artifact)?;
    if published_path.exists() {
        return print_report_file("published", true, &args.artifact, &published_path);
    }

    let attempt_path = root
        .as_std_path()
        .join(ATTEMPT_METADATA_DIR)
        .join(&args.artifact);
    if attempt_path.exists() {
        return print_report_file("attempt", false, &args.artifact, &attempt_path);
    }

    anyhow::bail!(
        "report artifact '{}' not found (published: {}, attempt: {})",
        args.artifact,
        published_path.display(),
        attempt_path.display()
    );
}

#[derive(Debug, serde::Deserialize)]
struct ReportOnlyConfig {
    output: ReportOnlyOutputConfig,
}

#[derive(Debug, serde::Deserialize)]
struct ReportOnlyOutputConfig {
    path: String,
}

fn load_report_output_path(project_root: &Path) -> Result<String> {
    let path = project_root.join("kslim.toml");
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let config: ReportOnlyConfig =
        toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))?;
    if config.output.path.trim().is_empty() {
        anyhow::bail!("output.path must not be empty");
    }
    Ok(config.output.path)
}

fn print_report_file(scope: &str, authoritative: bool, artifact: &str, path: &Path) -> Result<()> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read report artifact {}", path.display()))?;
    println!("report scope: {}", scope);
    println!("authoritative: {}", authoritative);
    println!("artifact: {}", artifact);
    println!("path: {}", path.display());
    println!();
    print!("{}", contents);
    if !contents.ends_with('\n') {
        println!();
    }
    Ok(())
}

fn cmd_explain(query: &str) -> Result<()> {
    let query = query.trim();
    if query.is_empty() {
        anyhow::bail!("--explain query must not be empty");
    }
    if parse_explain_edit_query(query).is_ok() {
        return cmd_explain_edit(ExplainEditArgs {
            query: query.to_string(),
        });
    }
    if explain_query_looks_like_path(query) {
        return cmd_explain_path(query);
    }
    cmd_explain_symbol(ExplainSymbolArgs {
        symbol: query.to_string(),
    })
}

fn cmd_explain_edit(args: ExplainEditArgs) -> Result<()> {
    let query = parse_explain_edit_query(&args.query)?;
    let root = crate::fsutil::find_kslim_root()?;
    let artifact = load_explain_edit_summary(root.as_std_path())?;
    let matches = artifact
        .summary
        .edit_record_details
        .iter()
        .filter(|edit| edit_matches_explain_query(edit, &query))
        .collect::<Vec<_>>();

    if matches.is_empty() {
        anyhow::bail!(
            "no edit record found for {}:{} in {} edit summary {}",
            query.path.as_path().display(),
            query.line,
            artifact.scope,
            artifact.path.display()
        );
    }

    println!(
        "explain-edit: {}:{}",
        query.path.as_path().display(),
        query.line
    );
    println!("report scope: {}", artifact.scope);
    println!("authoritative: {}", artifact.authoritative);
    println!("edit summary: {}", artifact.path.display());
    println!("matches: {}", matches.len());

    for (index, edit) in matches.iter().enumerate() {
        print_explain_edit_match(index + 1, edit);
    }

    print_explain_edit_related_reports(&artifact.path);
    Ok(())
}

fn cmd_explain_path(query: &str) -> Result<()> {
    let path = RelativeKernelPath::new(query.trim())?;
    let root = crate::fsutil::find_kslim_root()?;
    let artifact = load_explain_edit_summary(root.as_std_path())?;
    let matches = artifact
        .summary
        .edit_record_details
        .iter()
        .filter(|edit| Path::new(&edit.file) == path.as_path())
        .collect::<Vec<_>>();
    if matches.is_empty() {
        anyhow::bail!(
            "no edit record found for path {} in {} edit summary {}",
            path.as_path().display(),
            artifact.scope,
            artifact.path.display()
        );
    }
    println!("explain-path: {}", path.as_path().display());
    println!("report scope: {}", artifact.scope);
    println!("authoritative: {}", artifact.authoritative);
    println!("edit summary: {}", artifact.path.display());
    println!("matches: {}", matches.len());
    for (index, edit) in matches.iter().enumerate() {
        print_explain_edit_match(index + 1, edit);
    }
    print_explain_edit_related_reports(&artifact.path);
    Ok(())
}

fn cmd_explain_symbol(args: ExplainSymbolArgs) -> Result<()> {
    let symbol = normalize_explain_symbol_query(&args.symbol)?;
    let root = crate::fsutil::find_kslim_root()?;
    let artifact = load_explain_edit_summary(root.as_std_path())?;
    let reducer_report = load_explain_reducer_report_for_edit_summary(&artifact.path)?;
    let matches = artifact
        .summary
        .edit_record_details
        .iter()
        .filter(|edit| edit_mentions_symbol(edit, &symbol))
        .collect::<Vec<_>>();
    let decision = explain_symbol_decision(&reducer_report.report, &symbol, matches.len());

    if matches.is_empty() && decision == "not found" {
        anyhow::bail!(
            "no symbol decision or edit record found for CONFIG_{} in {} reports {}",
            symbol,
            artifact.scope,
            artifact.path.display()
        );
    }

    println!("explain-symbol: CONFIG_{}", symbol);
    println!("normalized symbol: {}", symbol);
    println!("report scope: {}", artifact.scope);
    println!("authoritative: {}", artifact.authoritative);
    println!("reducer report: {}", reducer_report.path.display());
    println!("decision: {}", decision);
    println!(
        "owner: {}",
        explain_symbol_owner(&reducer_report.report, &symbol, matches.len())
    );
    println!(
        "proof source: {}",
        explain_symbol_proof_source(&reducer_report.report, &symbol, matches.len())
    );
    println!("matching edits: {}", matches.len());

    for (index, edit) in matches.iter().enumerate() {
        print_explain_edit_match(index + 1, edit);
    }

    print_explain_edit_related_reports(&artifact.path);
    Ok(())
}

fn cmd_explain_feature(args: ExplainFeatureArgs) -> Result<()> {
    let feature = normalize_feature_name_query(&args.feature)?;
    let root = crate::fsutil::find_kslim_root()?;
    let _config = config::load_kslim_config(root.as_std_path())?;
    let profile_name = config::normalize_profile_name(&args.profile)?;
    config::require_known_profile(root.as_std_path(), &profile_name)?;
    let profile = config::load_profile(root.as_std_path(), &profile_name)?;
    require_declared_feature(&profile, &feature)?;

    println!("explain-feature: {}", feature);
    println!("profile: {}", profile_name);

    if let Some(intent) = profile.features.remove.get(&feature) {
        command_render::print_explain_feature_intent(&feature, "remove", intent, &profile)?;
    }
    if let Some(intent) = profile.features.preserve.get(&feature) {
        command_render::print_explain_feature_intent(&feature, "preserve", intent, &profile)?;
    }

    Ok(())
}

fn cmd_explain_abi(args: ExplainAbiArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let _config = config::load_kslim_config(root.as_std_path())?;
    let profile_name = config::normalize_profile_name(&args.profile)?;
    config::require_known_profile(root.as_std_path(), &profile_name)?;
    let config::LoadedProfileConfig {
        profile,
        source_map,
    } = config::load_profile_with_source_map(root.as_std_path(), &profile_name)?;
    let effective_policy = profile.effective_abi_policy();
    let surfaces = collect_explain_abi_surfaces(&profile);

    println!("explain-abi");
    println!("profile: {}", profile_name);
    println!("decision: {}", explain_abi_decision(&surfaces));
    println!("owner: profile {}", profile_name);
    println!("proof source: profile ABI/UAPI policy");
    println!("policy:");
    print_explain_abi_policy_value(
        "abi.allow_public_header_removal",
        profile.abi.allow_public_header_removal,
        &source_map,
    );
    print_explain_abi_policy_value(
        "abi.allow_uapi_header_removal",
        profile.abi.allow_uapi_header_removal,
        &source_map,
    );
    println!("effective policy:");
    println!(
        "  allow public header removal: {}",
        effective_policy.allow_public_header_removal
    );
    println!(
        "  allow uapi header removal: {}",
        effective_policy.allow_uapi_header_removal
    );
    println!("  fail closed: {}", effective_policy.is_fail_closed());
    print_explain_abi_feature_approvals(&profile, &source_map);
    print_explain_abi_surfaces(&surfaces);
    println!(
        "fail-closed behavior: ABI-sensitive removals require explicit profile [abi] approval or scoped features.remove approval"
    );

    Ok(())
}

#[derive(Debug)]
struct ExplainAbiSurface {
    path: String,
    kind: &'static str,
    owner: String,
    proof_source: String,
    approved: bool,
}

struct ExplainEditQuery {
    path: RelativeKernelPath,
    line: usize,
}

#[derive(Debug, serde::Deserialize)]
struct ExplainEditSummary {
    edit_record_details: Vec<ExplainEditRecord>,
}

#[derive(Debug, serde::Deserialize)]
struct ExplainEditRecord {
    file: String,
    pass_name: String,
    edit_kind: String,
    edit_reason: ExplainEditTruth,
    proof_source: ExplainEditTruth,
    old: ExplainEditOld,
    #[serde(rename = "new")]
    new_value: ExplainEditNew,
    idempotence_marker: String,
}

#[derive(Debug, serde::Deserialize)]
struct ExplainEditTruth {
    kind: String,
    payload: String,
}

#[derive(Debug, serde::Deserialize)]
struct ExplainEditOld {
    line_start: Option<usize>,
    line_end: Option<usize>,
    logical_item: String,
    byte_len: usize,
    sha256: String,
}

#[derive(Debug, serde::Deserialize)]
struct ExplainEditNew {
    logical_item: String,
    byte_len: usize,
    sha256: String,
}

struct ExplainEditArtifact {
    scope: &'static str,
    authoritative: bool,
    path: std::path::PathBuf,
    summary: ExplainEditSummary,
}

#[derive(Debug, serde::Deserialize)]
struct ExplainReducerReport {
    normalized_removal_manifest: ExplainRemovalManifest,
}

#[derive(Debug, serde::Deserialize)]
struct ExplainRemovalManifest {
    #[serde(default)]
    removed_config_symbols: Vec<String>,
    #[serde(default)]
    preserved_config_symbols: Vec<String>,
    #[serde(default)]
    default_overrides: BTreeMap<String, String>,
}

struct ExplainReducerReportArtifact {
    path: std::path::PathBuf,
    report: ExplainReducerReport,
}

fn parse_explain_edit_query(query: &str) -> Result<ExplainEditQuery> {
    let query = query.trim();
    if query.is_empty() {
        anyhow::bail!("explain-edit query must use PATH:LINE form");
    }
    let (path, line) = query
        .rsplit_once(':')
        .ok_or_else(|| anyhow::anyhow!("explain-edit query must use PATH:LINE form"))?;
    let path = path.trim();
    if path.is_empty() {
        anyhow::bail!("explain-edit path must not be empty");
    }
    let line = line
        .trim()
        .parse::<usize>()
        .with_context(|| format!("invalid explain-edit line in query '{query}'"))?;
    if line == 0 {
        anyhow::bail!("explain-edit line must be greater than zero");
    }

    Ok(ExplainEditQuery {
        path: RelativeKernelPath::new(path)?,
        line,
    })
}

fn explain_query_looks_like_path(query: &str) -> bool {
    query.contains('/') || query.contains('.')
}

fn normalize_explain_symbol_query(symbol: &str) -> Result<String> {
    let symbol = symbol.trim();
    if symbol.is_empty() {
        anyhow::bail!("explain-symbol query must not be empty");
    }
    let normalized = symbol.strip_prefix("CONFIG_").unwrap_or(symbol);
    let symbol = KconfigSymbol::new(normalized)?;
    Ok(symbol.as_str().to_string())
}

fn normalize_feature_name_query(feature: &str) -> Result<String> {
    let feature = feature.trim();
    if feature.is_empty() {
        anyhow::bail!("explain-feature query must not be empty");
    }
    Ok(feature.to_string())
}

fn load_explain_edit_summary(project_root: &Path) -> Result<ExplainEditArtifact> {
    let output_path = load_report_output_path(project_root)?;
    let published_path = output_repo::reducer_artifact_path(
        Path::new(&output_path),
        output_repo::REDUCER_EDIT_SUMMARY_JSON,
    )?;
    if published_path.exists() {
        return read_explain_edit_summary("published", true, published_path);
    }

    let attempt_path = project_root
        .join(ATTEMPT_METADATA_DIR)
        .join(output_repo::REDUCER_EDIT_SUMMARY_JSON);
    if attempt_path.exists() {
        return read_explain_edit_summary("attempt", false, attempt_path);
    }

    anyhow::bail!(
        "edit summary not found (published: {}, attempt: {})",
        published_path.display(),
        attempt_path.display()
    )
}

fn load_explain_reducer_report_for_edit_summary(
    edit_summary_path: &Path,
) -> Result<ExplainReducerReportArtifact> {
    let metadata_dir = edit_summary_path.parent().unwrap_or_else(|| Path::new("."));
    let path = metadata_dir.join(output_repo::REDUCER_REPORT_JSON);
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read reducer report {}", path.display()))?;
    let report = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse reducer report {}", path.display()))?;
    Ok(ExplainReducerReportArtifact { path, report })
}

fn read_explain_edit_summary(
    scope: &'static str,
    authoritative: bool,
    path: std::path::PathBuf,
) -> Result<ExplainEditArtifact> {
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read edit summary {}", path.display()))?;
    let summary = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse edit summary {}", path.display()))?;
    Ok(ExplainEditArtifact {
        scope,
        authoritative,
        path,
        summary,
    })
}

fn edit_mentions_symbol(edit: &ExplainEditRecord, symbol: &str) -> bool {
    text_mentions_symbol(&edit.edit_reason.payload, symbol)
        || text_mentions_symbol(&edit.proof_source.payload, symbol)
        || text_mentions_symbol(&edit.old.logical_item, symbol)
        || text_mentions_symbol(&edit.new_value.logical_item, symbol)
}

fn text_mentions_symbol(text: &str, symbol: &str) -> bool {
    contains_symbol_token(text, symbol)
        || contains_symbol_token(text, format!("CONFIG_{symbol}").as_str())
}

fn contains_symbol_token(text: &str, needle: &str) -> bool {
    text.match_indices(needle).any(|(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + needle.len()..].chars().next();
        !is_symbol_char(before) && !is_symbol_char(after)
    })
}

fn is_symbol_char(ch: Option<char>) -> bool {
    ch.is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn edit_matches_explain_query(edit: &ExplainEditRecord, query: &ExplainEditQuery) -> bool {
    if Path::new(&edit.file) != query.path.as_path() {
        return false;
    }

    match (edit.old.line_start, edit.old.line_end) {
        (Some(start), Some(end)) => start <= query.line && query.line <= end,
        (Some(start), None) => start == query.line,
        (None, Some(end)) => query.line <= end,
        (None, None) => true,
    }
}

fn print_explain_edit_match(index: usize, edit: &ExplainEditRecord) {
    println!("match {}:", index);
    println!("  file: {}", edit.file);
    println!("  decision: {}", edit_decision(&edit.edit_kind));
    println!(
        "  owner: {} {}",
        edit.edit_reason.kind, edit.edit_reason.payload
    );
    println!("  pass: {}", edit.pass_name);
    println!("  edit kind: {}", edit.edit_kind);
    println!("  line range: {}", edit_line_range(edit));
    println!(
        "  proof source: {} {}",
        edit.proof_source.kind, edit.proof_source.payload
    );
    println!("  old: {}", one_line_logical_item(&edit.old.logical_item));
    println!(
        "  new: {}",
        one_line_logical_item(&edit.new_value.logical_item)
    );
    println!(
        "  old bytes: {} sha256: {}",
        edit.old.byte_len, edit.old.sha256
    );
    println!(
        "  new bytes: {} sha256: {}",
        edit.new_value.byte_len, edit.new_value.sha256
    );
    println!("  idempotence marker: {}", edit.idempotence_marker);
}

fn collect_explain_abi_surfaces(profile: &config::ProfileConfig) -> Vec<ExplainAbiSurface> {
    let mut surfaces = Vec::new();

    if let Some(slim) = profile.removal_input() {
        for path in &slim.remove_paths {
            if let Some(kind) = explain_abi_surface_kind(path) {
                surfaces.push(ExplainAbiSurface {
                    path: path.clone(),
                    kind,
                    owner: String::from("profile slim.remove_paths"),
                    proof_source: explain_abi_profile_policy_key(kind).to_string(),
                    approved: explain_abi_policy_allows(kind, &profile.abi),
                });
            }
        }
    }

    for (feature, intent) in &profile.features.remove {
        let policy = explain_abi_feature_policy(&profile.abi, intent);
        for path in intent.roots.iter().chain(intent.remove_paths.iter()) {
            if let Some(kind) = explain_abi_surface_kind(path) {
                surfaces.push(ExplainAbiSurface {
                    path: path.clone(),
                    kind,
                    owner: format!("features.remove.{feature}"),
                    proof_source: explain_abi_feature_policy_source(
                        kind,
                        feature,
                        &profile.abi,
                        intent,
                    ),
                    approved: explain_abi_policy_allows(kind, &policy),
                });
            }
        }
    }

    surfaces.sort_by(|a, b| {
        a.owner
            .cmp(&b.owner)
            .then(a.path.cmp(&b.path))
            .then(a.kind.cmp(b.kind))
    });
    surfaces
}

fn explain_abi_surface_kind(path: &str) -> Option<&'static str> {
    let path = Path::new(path.trim());
    if crate::abi::is_uapi_path(path) {
        return Some("uapi");
    }
    if path_has_header_extension(path) && crate::abi::is_public_header_path(path) {
        return Some("public_header");
    }
    None
}

fn path_has_header_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext == "h")
}

fn explain_abi_feature_policy(
    profile_policy: &config::AbiPolicyConfig,
    intent: &config::FeatureIntentConfig,
) -> config::AbiPolicyConfig {
    let mut policy = profile_policy.clone();
    policy.allow_public_header_removal |= intent.allow_public_header_removal;
    policy.allow_uapi_header_removal |= intent.allow_uapi_header_removal;
    policy
}

fn explain_abi_policy_allows(kind: &str, policy: &config::AbiPolicyConfig) -> bool {
    match kind {
        "uapi" => policy.allow_uapi_header_removal,
        "public_header" => policy.allow_public_header_removal,
        _ => false,
    }
}

fn explain_abi_profile_policy_key(kind: &str) -> &'static str {
    match kind {
        "uapi" => "abi.allow_uapi_header_removal",
        "public_header" => "abi.allow_public_header_removal",
        _ => "abi",
    }
}

fn explain_abi_feature_policy_source(
    kind: &str,
    feature: &str,
    profile_policy: &config::AbiPolicyConfig,
    intent: &config::FeatureIntentConfig,
) -> String {
    match kind {
        "uapi" if profile_policy.allow_uapi_header_removal => {
            String::from("abi.allow_uapi_header_removal")
        }
        "uapi" if intent.allow_uapi_header_removal => {
            format!("features.remove.{feature}.allow_uapi_header_removal")
        }
        "uapi" => String::from("missing abi.allow_uapi_header_removal"),
        "public_header" if profile_policy.allow_public_header_removal => {
            String::from("abi.allow_public_header_removal")
        }
        "public_header" if intent.allow_public_header_removal => {
            format!("features.remove.{feature}.allow_public_header_removal")
        }
        "public_header" => String::from("missing abi.allow_public_header_removal"),
        _ => String::from("unknown ABI policy"),
    }
}

fn explain_abi_decision(surfaces: &[ExplainAbiSurface]) -> &'static str {
    if surfaces.is_empty() {
        "no ABI-sensitive removals"
    } else if surfaces.iter().all(|surface| surface.approved) {
        "approved"
    } else {
        "refused"
    }
}

fn print_explain_abi_policy_value(key: &str, value: bool, source_map: &config::ConfigSourceMap) {
    println!(
        "  {}: {} ({})",
        key,
        value,
        config_source_summary(source_map, key)
    );
}

fn print_explain_abi_feature_approvals(
    profile: &config::ProfileConfig,
    source_map: &config::ConfigSourceMap,
) {
    println!("feature-scoped approvals:");
    let mut printed = false;
    for (feature, intent) in &profile.features.remove {
        if intent.allow_public_header_removal {
            let key = format!("features.remove.{feature}.allow_public_header_removal");
            println!(
                "  {}: true ({})",
                key,
                config_source_summary(source_map, &key)
            );
            printed = true;
        }
        if intent.allow_uapi_header_removal {
            let key = format!("features.remove.{feature}.allow_uapi_header_removal");
            println!(
                "  {}: true ({})",
                key,
                config_source_summary(source_map, &key)
            );
            printed = true;
        }
    }
    if !printed {
        println!("  (none)");
    }
}

fn print_explain_abi_surfaces(surfaces: &[ExplainAbiSurface]) {
    println!("ABI-sensitive removal candidates:");
    if surfaces.is_empty() {
        println!("  (none)");
        return;
    }

    for surface in surfaces {
        println!("  - {} ({})", surface.path, surface.kind);
        println!(
            "    decision: {}",
            if surface.approved {
                "approved"
            } else {
                "refused"
            }
        );
        println!("    owner: {}", surface.owner);
        println!("    proof source: {}", surface.proof_source);
    }
}

fn config_source_summary(source_map: &config::ConfigSourceMap, key: &str) -> String {
    source_map
        .get(key)
        .map(|source| format!("{} from {}", source.kind.as_str(), source.source))
        .unwrap_or_else(|| String::from("unknown source"))
}

fn explain_symbol_decision(
    report: &ExplainReducerReport,
    symbol: &str,
    matching_edits: usize,
) -> &'static str {
    if report
        .normalized_removal_manifest
        .removed_config_symbols
        .iter()
        .any(|removed| removed == symbol)
    {
        "removed"
    } else if report
        .normalized_removal_manifest
        .preserved_config_symbols
        .iter()
        .any(|preserved| preserved == symbol)
    {
        "preserved"
    } else if report
        .normalized_removal_manifest
        .default_overrides
        .contains_key(symbol)
    {
        "default overridden"
    } else if matching_edits > 0 {
        "affected"
    } else {
        "not found"
    }
}

fn explain_symbol_owner(
    report: &ExplainReducerReport,
    symbol: &str,
    matching_edits: usize,
) -> String {
    if report
        .normalized_removal_manifest
        .removed_config_symbols
        .iter()
        .any(|removed| removed == symbol)
    {
        format!("removal manifest symbol={symbol}")
    } else if report
        .normalized_removal_manifest
        .preserved_config_symbols
        .iter()
        .any(|preserved| preserved == symbol)
    {
        format!("preservation manifest symbol={symbol}")
    } else if let Some(value) = report
        .normalized_removal_manifest
        .default_overrides
        .get(symbol)
    {
        format!("default override symbol={symbol} value={value}")
    } else if matching_edits > 0 {
        String::from("matching edit records")
    } else {
        String::from("none")
    }
}

fn explain_symbol_proof_source(
    report: &ExplainReducerReport,
    symbol: &str,
    matching_edits: usize,
) -> String {
    if report
        .normalized_removal_manifest
        .removed_config_symbols
        .iter()
        .any(|removed| removed == symbol)
    {
        format!("removal_manifest_entry symbol={symbol}")
    } else if report
        .normalized_removal_manifest
        .preserved_config_symbols
        .iter()
        .any(|preserved| preserved == symbol)
    {
        format!("removal_manifest_entry preserve_symbol={symbol}")
    } else if report
        .normalized_removal_manifest
        .default_overrides
        .contains_key(symbol)
    {
        format!("removal_manifest_entry default_override={symbol}")
    } else if matching_edits > 0 {
        String::from("edit record proof sources")
    } else {
        String::from("none")
    }
}

fn print_explain_edit_related_reports(edit_summary_path: &Path) {
    let metadata_dir = edit_summary_path.parent().unwrap_or_else(|| Path::new("."));
    println!("related reports:");
    println!("  edit summary: {}", edit_summary_path.display());
    println!(
        "  reducer report: {}",
        metadata_dir
            .join(output_repo::REDUCER_REPORT_JSON)
            .display()
    );
    println!(
        "  diagnostics: {}",
        metadata_dir
            .join(output_repo::REDUCER_DIAGNOSTICS_JSON)
            .display()
    );
    println!(
        "  kconfig solver: {}",
        metadata_dir
            .join(output_repo::REDUCER_KCONFIG_SOLVER_REPORT_JSON)
            .display()
    );
    println!(
        "  kconfig rewrite: {}",
        metadata_dir
            .join(output_repo::REDUCER_KCONFIG_REWRITE_REPORT_JSON)
            .display()
    );
}

fn edit_decision(edit_kind: &str) -> &'static str {
    match edit_kind {
        "remove_path" | "remove_line" | "remove_block" => "removed",
        "rewrite_line" | "rewrite_block" => "rewritten",
        _ => "changed",
    }
}

fn edit_line_range(edit: &ExplainEditRecord) -> String {
    match (edit.old.line_start, edit.old.line_end) {
        (Some(start), Some(end)) if start == end => start.to_string(),
        (Some(start), Some(end)) => format!("{start}-{end}"),
        _ => String::from("structural"),
    }
}

fn one_line_logical_item(value: &str) -> String {
    if value.is_empty() {
        String::from("<empty>")
    } else {
        value.replace('\n', "\\n")
    }
}

fn cmd_status() -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let config = config::load_kslim_config(root.as_std_path())?;

    println!("project: {}", config.project.name);
    println!();

    println!("upstream:");
    println!("  name: {}", config.upstream.name);
    println!("  url:  {}", config.upstream.url);
    println!("  mode: direct read-only");
    match upstream::check_access(&config.upstream.url) {
        Ok(path) => println!("  source: {}", path),
        Err(_) => println!("  source: (unavailable)"),
    }
    println!();

    // Lockfile info
    let lockfile_path = LockfilePath::new_in_project_root(root.as_std_path())?;
    if let Ok(Some(lock)) = lockfile::load_lockfile(&lockfile_path) {
        println!("locked base:");
        println!("  ref:     {}", lock.resolved_base.r#ref);
        println!("  commit:  {}", lock.resolved_base.commit);
        println!("  resolved: {}", lock.resolved_base.resolved_at);
        if let Some(published) = lock.published {
            println!("  published branch: {}", published.output_branch);
            println!("  published output commit: {}", published.output_commit);
        }
    } else {
        println!("locked base: (not resolved yet)");
    }
    println!();

    println!("output:");
    println!("  path: {}", config.output.path);
    let out_path = std::path::Path::new(&config.output.path);
    if out_path.exists() {
        if output_repo::is_kslim_managed(&config.output.path) {
            println!("  managed: yes");
        } else {
            println!("  managed: no");
        }
        if out_path.join(".git").exists() {
            match crate::git::current_branch(&config.output.path) {
                Ok(b) if b.is_empty() => println!("  head: (detached)"),
                Ok(b) => println!("  branch: {}", b),
                Err(_) => println!("  branch: (unknown)"),
            }
            match crate::git::head_commit(&config.output.path) {
                Ok(sha) => println!("  last commit: {}", sha),
                Err(_) => println!("  last commit: (none)"),
            }
            match crate::git::is_dirty(&config.output.path) {
                Ok(true) => println!("  dirty: yes"),
                Ok(false) => println!("  dirty: no"),
                Err(_) => println!("  dirty: (unknown)"),
            }
        } else {
            println!("  (not a git repository)");
        }
    } else {
        println!("  (does not exist yet)");
    }
    println!();

    let output_repo_path = OutputRepoPath::new(config.output.path.as_str())?;
    match output_repo::load_authoritative_published_state(&lockfile_path, &output_repo_path) {
        Ok(Some(state)) => {
            println!("published snapshot:");
            println!("  branch: {}", state.lockfile.output_branch);
            println!("  output commit: {}", state.lockfile.output_commit);
            println!("  tag: {}", state.lockfile.tag);
            println!(
                "  base: {} @ {}",
                state.lockfile.base_ref, state.lockfile.base_commit
            );
            println!("  profile: {}", state.lockfile.profile);
            println!("  mode: {}", state.lockfile.mode);
            println!("  generated: {}", state.lockfile.generated_at);
        }
        Ok(None) => println!("published snapshot: (not published yet)"),
        Err(err) => println!("published snapshot: (invalid: {:#})", err),
    }
    println!();

    print_last_attempt_status(root.as_std_path())?;
    println!();

    if let Some(pub_conf) = &config.publish {
        println!("publish:");
        println!("  remote: {}", pub_conf.remote);
    } else {
        println!("publish: not configured");
    }
    println!();

    println!("profiles:");
    let profiles = config::list_profiles(root.as_std_path())?;
    if profiles.is_empty() {
        println!("  (none)");
    } else {
        for p in &profiles {
            match config::load_profile(root.as_std_path(), p) {
                Ok(prof) => println!("  - {} (base: {})", p, prof.base.r#ref),
                Err(_) => println!("  - {} (unparseable)", p),
            }
        }
    }

    Ok(())
}

fn cmd_repair() -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let attempt_dir = root.as_std_path().join(ATTEMPT_METADATA_DIR);

    if remove_non_authoritative_attempt_metadata(root.as_std_path(), &attempt_dir)? {
        println!("repair: cleared non-authoritative attempt metadata");
        println!("removed: {}", ATTEMPT_METADATA_DIR);
    } else {
        println!("repair: nothing to repair");
    }
    println!("authoritative state: unchanged");
    println!("lockfile: unchanged");
    println!("output: unchanged");

    Ok(())
}

fn remove_non_authoritative_attempt_metadata(
    project_root: &Path,
    attempt_dir: &Path,
) -> Result<bool> {
    ensure_repair_attempt_metadata_path(project_root, attempt_dir)?;

    let metadata = match std::fs::symlink_metadata(attempt_dir) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(err).with_context(|| {
                format!(
                    "failed to inspect attempt metadata {}",
                    attempt_dir.display()
                )
            })
        }
    };

    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        std::fs::remove_dir_all(attempt_dir).with_context(|| {
            format!(
                "failed to remove non-authoritative attempt metadata {}",
                attempt_dir.display()
            )
        })?;
    } else {
        std::fs::remove_file(attempt_dir).with_context(|| {
            format!(
                "failed to remove non-authoritative attempt metadata {}",
                attempt_dir.display()
            )
        })?;
    }

    Ok(true)
}

fn ensure_repair_attempt_metadata_path(project_root: &Path, attempt_dir: &Path) -> Result<()> {
    let expected = project_root.join(ATTEMPT_METADATA_DIR);
    if attempt_dir != expected.as_path() {
        anyhow::bail!(
            "repair may remove only non-authoritative attempt metadata: {} != {}",
            attempt_dir.display(),
            expected.display()
        );
    }
    Ok(())
}

fn print_last_attempt_status(project_root: &std::path::Path) -> Result<()> {
    let attempt_dir = project_root.join(ATTEMPT_METADATA_DIR);
    let generate_failure_path = attempt_dir.join(GENERATE_FAILURE_FILE);
    let last_attempt_path = attempt_dir.join(output_repo::LAST_ATTEMPT_JSON);
    let failure_report_path = attempt_dir.join(FAILURE_REPORT_FILE);

    if !generate_failure_path.exists()
        && !last_attempt_path.exists()
        && !failure_report_path.exists()
    {
        println!("last attempt: (none recorded)");
        return Ok(());
    }

    println!("last attempt:");
    println!("  authoritative: false");
    println!(
        "  metadata scope: {}",
        output_repo::NON_AUTHORITATIVE_ATTEMPT_SCOPE
    );
    println!("  metadata dir: {}", ATTEMPT_METADATA_DIR);

    let mut printed_report_reference = false;
    if generate_failure_path.exists() {
        match print_generate_failure_status(&generate_failure_path) {
            Ok(printed) => printed_report_reference = printed,
            Err(err) => println!("  structured failure: (invalid: {:#})", err),
        }
    } else if last_attempt_path.exists() {
        match print_last_attempt_json_status(&last_attempt_path) {
            Ok(printed) => printed_report_reference = printed,
            Err(err) => println!("  last-attempt metadata: (invalid: {:#})", err),
        }
    }

    if failure_report_path.exists() && !printed_report_reference {
        println!(
            "  failure report: {}/{}",
            ATTEMPT_METADATA_DIR, FAILURE_REPORT_FILE
        );
    }

    Ok(())
}

fn print_generate_failure_status(path: &std::path::Path) -> Result<bool> {
    let contents = std::fs::read_to_string(path)?;
    let failure: toml::Value = toml::from_str(&contents)?;

    print_optional_status_str("stage", failure.get("stage").and_then(toml::Value::as_str));
    print_optional_status_str(
        "error kind",
        failure.get("error_kind").and_then(toml::Value::as_str),
    );
    print_optional_status_str(
        "failure",
        failure
            .get("message")
            .and_then(toml::Value::as_str)
            .and_then(first_nonempty_line),
    );

    let mut printed_report_reference = false;
    if let Some(paths) = failure.get("report_paths").and_then(toml::Value::as_array) {
        let paths = paths
            .iter()
            .filter_map(toml::Value::as_str)
            .collect::<Vec<_>>();
        if paths.iter().any(|path| *path == FAILURE_REPORT_FILE) {
            println!(
                "  failure report: {}/{}",
                ATTEMPT_METADATA_DIR, FAILURE_REPORT_FILE
            );
            printed_report_reference = true;
        }
        let other_paths = paths
            .iter()
            .copied()
            .filter(|path| *path != FAILURE_REPORT_FILE)
            .collect::<Vec<_>>();
        if !other_paths.is_empty() {
            println!("  report paths:");
        }
        for path in other_paths {
            println!("    - {}/{}", ATTEMPT_METADATA_DIR, path);
            printed_report_reference = true;
        }
    }

    Ok(printed_report_reference)
}

fn print_last_attempt_json_status(path: &std::path::Path) -> Result<bool> {
    let contents = std::fs::read_to_string(path)?;
    let attempt: serde_json::Value = serde_json::from_str(&contents)?;

    print_optional_status_str(
        "stage",
        attempt.get("stage").and_then(serde_json::Value::as_str),
    );
    print_optional_status_str(
        "error kind",
        attempt
            .get("error_kind")
            .and_then(serde_json::Value::as_str),
    );
    print_optional_status_str(
        "failure",
        attempt
            .get("failure")
            .and_then(serde_json::Value::as_str)
            .and_then(first_nonempty_line),
    );
    if let Some(path) = attempt
        .get("failure_report")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.trim().is_empty())
    {
        println!("  failure report: {}/{}", ATTEMPT_METADATA_DIR, path);
        return Ok(true);
    }

    Ok(false)
}

fn print_optional_status_str(label: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        println!("  {}: {}", label, value);
    }
}

fn first_nonempty_line(value: &str) -> Option<&str> {
    value.lines().find(|line| !line.trim().is_empty())
}

fn cmd_compare(args: CompareArgs) -> Result<()> {
    let root = crate::fsutil::find_kslim_root()?;
    let config = config::load_kslim_config(root.as_std_path())?;

    let upstream_path = upstream::check_access(&config.upstream.url)
        .context("upstream is not accessible. Fix upstream.url or run `kslim upstream sync`.")?;

    let commit_from = upstream::resolve_ref(upstream_path.as_str(), &args.from)?;
    let commit_to = upstream::resolve_ref(upstream_path.as_str(), &args.to)?;

    println!("comparing upstream snapshots:");
    println!("  from: {} ({})", args.from, commit_from);
    println!("  to:   {} ({})", args.to, commit_to);
    println!();

    // Materialize both trees into temp dirs
    let tmp_from = tempfile::Builder::new()
        .prefix("kslim-cmp-from-")
        .tempdir()?;
    let tmp_to = tempfile::Builder::new().prefix("kslim-cmp-to-").tempdir()?;
    let tmp_from_path = tmp_from.path().to_string_lossy().to_string();
    let tmp_to_path = tmp_to.path().to_string_lossy().to_string();

    upstream::archive_tree(upstream_path.as_str(), &commit_from, &tmp_from_path)?;
    upstream::archive_tree(upstream_path.as_str(), &commit_to, &tmp_to_path)?;

    let from_entries = manifest::generate_manifest(&tmp_from_path)?;
    let to_entries = manifest::generate_manifest(&tmp_to_path)?;

    // Build index by path
    let from_map: std::collections::BTreeMap<&str, &crate::manifest::FileEntry> =
        from_entries.iter().map(|e| (e.path.as_str(), e)).collect();
    let to_map: std::collections::BTreeMap<&str, &crate::manifest::FileEntry> =
        to_entries.iter().map(|e| (e.path.as_str(), e)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (path, entry) in &to_map {
        match from_map.get(path) {
            None => added.push(*path),
            Some(old) if old.sha256 != entry.sha256 => changed.push(*path),
            _ => {}
        }
    }
    for path in from_map.keys() {
        if !to_map.contains_key(path) {
            removed.push(*path);
        }
    }

    println!("files added:   {}", added.len());
    println!("files removed: {}", removed.len());
    println!("files changed: {}", changed.len());
    println!();

    if !added.is_empty() {
        println!("--- Added ---");
        for p in &added {
            println!("  + {}", p);
        }
        println!();
    }
    if !removed.is_empty() {
        println!("--- Removed ---");
        for p in &removed {
            println!("  - {}", p);
        }
        println!();
    }
    if !changed.is_empty() {
        println!("--- Changed ---");
        for p in &changed {
            println!("  ~ {}", p);
        }
        println!();
    }

    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        println!("No changes detected between {} and {}.", args.from, args.to);
    }

    drop(tmp_from);
    drop(tmp_to);

    Ok(())
}
