//! Clap argument structs for command-line shape.

// ── Init ──────────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct InitArgs {
    /// Project name
    #[arg(long, default_value = "linux-kslim")]
    pub name: String,

    /// Output repo path
    #[arg(long, default_value = "/home/me/projects/linux-kslim")]
    pub output: String,

    /// Upstream local git path
    #[arg(long, default_value = "/path/to/linux/.git")]
    pub upstream_url: String,

    /// Upstream name
    #[arg(long, default_value = "linux")]
    pub upstream_name: String,

    /// Publish remote URL (optional)
    #[arg(long)]
    pub publish_remote: Option<String>,

    /// Base ref for default profile
    #[arg(long, default_value = "v6.15")]
    pub base_ref: String,
}

// ── Validate config ──────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct ValidateConfigArgs {
    /// Validate only this profile instead of all profiles
    #[arg(long)]
    pub profile: Option<String>,
}

// ── Plan ─────────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct PlanArgs {
    /// Profile name
    #[arg(long, default_value = "default")]
    pub profile: String,

    /// Restrict named feature intent to this profile feature
    #[arg(long)]
    pub feature: Option<String>,

    /// Restrict named removal feature intent to this profile removal feature
    #[arg(long = "remove-feature", conflicts_with = "feature")]
    pub remove_feature: Option<String>,

    /// Restrict named preservation feature intent to this profile preservation feature
    #[arg(long = "preserve-feature", conflicts_with_all = ["feature", "remove_feature"])]
    pub preserve_feature: Option<String>,

    /// Restrict named feature intent to this kernel architecture
    #[arg(long)]
    pub arch: Option<String>,

    /// Restrict named feature intent to this primary kernel architecture
    #[arg(long = "primary-arch", conflicts_with = "arch")]
    pub primary_arch: Option<String>,

    /// Restrict named feature intent to this secondary kernel architecture
    #[arg(long = "secondary-arch", conflicts_with = "arch")]
    pub secondary_arch: Option<String>,

    /// Override active named removal feature safety level
    #[arg(long)]
    pub safety: Option<String>,

    /// Override maximum deterministic fixup retry passes
    #[arg(long = "max-fixup-passes")]
    pub max_fixup_passes: Option<usize>,

    /// Select verification matrix preset
    #[arg(long = "matrix")]
    pub matrix: Option<String>,

    /// Write the resolved immutable plan to this frozen plan file
    #[arg(long = "frozen-plan")]
    pub frozen_plan: Option<String>,

    /// Force strict reducer publication gates on
    #[arg(long, conflicts_with = "no_strict")]
    pub strict: bool,

    /// Force strict reducer publication gates off
    #[arg(long = "no-strict", conflicts_with = "strict")]
    pub no_strict: bool,

    /// Override base ref
    #[arg(long)]
    pub base: Option<String>,
}

// ── Feature impact ───────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct FeatureImpactArgs {
    /// Profile name
    #[arg(long, default_value = "default")]
    pub profile: String,

    /// Show only this named feature
    #[arg(long)]
    pub feature: Option<String>,

    /// Show only this named removal feature
    #[arg(long = "remove-feature", conflicts_with = "feature")]
    pub remove_feature: Option<String>,

    /// Show only this named preservation feature
    #[arg(long = "preserve-feature", conflicts_with_all = ["feature", "remove_feature"])]
    pub preserve_feature: Option<String>,

    /// Show only named feature intent for this kernel architecture
    #[arg(long)]
    pub arch: Option<String>,

    /// Show only named feature intent for this primary kernel architecture
    #[arg(long = "primary-arch", conflicts_with = "arch")]
    pub primary_arch: Option<String>,

    /// Show only named feature intent for this secondary kernel architecture
    #[arg(long = "secondary-arch", conflicts_with = "arch")]
    pub secondary_arch: Option<String>,

    /// Override active named removal feature safety level
    #[arg(long)]
    pub safety: Option<String>,

    /// Show impact with strict reducer publication gates forced on
    #[arg(long, conflicts_with = "no_strict")]
    pub strict: bool,

    /// Show impact with strict reducer publication gates forced off
    #[arg(long = "no-strict", conflicts_with = "strict")]
    pub no_strict: bool,
}

// ── Reduce tree ──────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct ReduceTreeArgs {
    /// Profile name
    #[arg(long, default_value = "default")]
    pub profile: String,

    /// Restrict named feature intent to this profile feature
    #[arg(long)]
    pub feature: Option<String>,

    /// Restrict named removal feature intent to this profile removal feature
    #[arg(long = "remove-feature", conflicts_with = "feature")]
    pub remove_feature: Option<String>,

    /// Restrict named preservation feature intent to this profile preservation feature
    #[arg(long = "preserve-feature", conflicts_with_all = ["feature", "remove_feature"])]
    pub preserve_feature: Option<String>,

    /// Restrict named feature intent to this kernel architecture
    #[arg(long)]
    pub arch: Option<String>,

    /// Restrict named feature intent to this primary kernel architecture
    #[arg(long = "primary-arch", conflicts_with = "arch")]
    pub primary_arch: Option<String>,

    /// Restrict named feature intent to this secondary kernel architecture
    #[arg(long = "secondary-arch", conflicts_with = "arch")]
    pub secondary_arch: Option<String>,

    /// Override active named removal feature safety level
    #[arg(long)]
    pub safety: Option<String>,

    /// Override maximum deterministic fixup retry passes
    #[arg(long = "max-fixup-passes")]
    pub max_fixup_passes: Option<usize>,

    /// Select verification matrix preset
    #[arg(long = "matrix")]
    pub matrix: Option<String>,

    /// Use this frozen plan file instead of rereading mutable profile intent
    #[arg(long = "frozen-plan")]
    pub frozen_plan: Option<String>,

    /// Force strict reducer publication gates on
    #[arg(long, conflicts_with = "no_strict")]
    pub strict: bool,

    /// Force strict reducer publication gates off
    #[arg(long = "no-strict", conflicts_with = "strict")]
    pub no_strict: bool,

    /// Existing kernel source tree to mutate in place
    #[arg(long)]
    pub tree: String,
}

// ── Base resolve args ───────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct ResolveArgs {
    /// Profile name
    #[arg(long, default_value = "default")]
    pub profile: String,
}

// ── Generate ─────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct GenerateArgs {
    /// Profile name
    #[arg(long, default_value = "default")]
    pub profile: String,

    /// Restrict named feature intent to this profile feature
    #[arg(long)]
    pub feature: Option<String>,

    /// Restrict named removal feature intent to this profile removal feature
    #[arg(long = "remove-feature", conflicts_with = "feature")]
    pub remove_feature: Option<String>,

    /// Restrict named preservation feature intent to this profile preservation feature
    #[arg(long = "preserve-feature", conflicts_with_all = ["feature", "remove_feature"])]
    pub preserve_feature: Option<String>,

    /// Restrict named feature intent to this kernel architecture
    #[arg(long)]
    pub arch: Option<String>,

    /// Restrict named feature intent to this primary kernel architecture
    #[arg(long = "primary-arch", conflicts_with = "arch")]
    pub primary_arch: Option<String>,

    /// Restrict named feature intent to this secondary kernel architecture
    #[arg(long = "secondary-arch", conflicts_with = "arch")]
    pub secondary_arch: Option<String>,

    /// Override active named removal feature safety level
    #[arg(long)]
    pub safety: Option<String>,

    /// Override maximum deterministic fixup retry passes
    #[arg(long = "max-fixup-passes")]
    pub max_fixup_passes: Option<usize>,

    /// Select verification matrix preset
    #[arg(long = "matrix")]
    pub matrix: Option<String>,

    /// Use this frozen plan file instead of rereading mutable profile intent
    #[arg(long = "frozen-plan")]
    pub frozen_plan: Option<String>,

    /// Force strict reducer publication gates on
    #[arg(long, conflicts_with = "no_strict")]
    pub strict: bool,

    /// Force strict reducer publication gates off
    #[arg(long = "no-strict", conflicts_with = "strict")]
    pub no_strict: bool,

    /// Override base ref
    #[arg(long)]
    pub base: Option<String>,

    /// Dry run: show what would happen without mutating output repo
    #[arg(long, conflicts_with = "deep_dry_run")]
    pub dry_run: bool,

    /// Deep dry run: materialize and verify the candidate without publishing output
    #[arg(
        long = "deep-dry-run",
        conflicts_with_all = ["dry_run", "report_only", "reducer_report_only"]
    )]
    pub deep_dry_run: bool,

    /// Resolve the generate plan and write non-authoritative attempt metadata
    #[arg(long = "report-only", conflicts_with = "deep_dry_run")]
    pub report_only: bool,

    /// Legacy alias for --report-only
    #[arg(long = "reducer-report-only", conflicts_with = "deep_dry_run")]
    pub reducer_report_only: bool,

    /// Keep private temporary candidate trees for debugging
    #[arg(long = "keep-temp")]
    pub keep_temp: bool,

    /// Force: bypass safety checks (dirty repo, detached HEAD)
    #[arg(long)]
    pub force: bool,

    /// Skip automatic selftests for this generate run
    #[arg(long)]
    pub no_selftests: bool,
}

// ── Publish ──────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct PublishArgs {
    /// Dry run (don't actually push)
    #[arg(long)]
    pub dry_run: bool,

    /// Force: bypass dirty check and remote URL mismatch
    #[arg(long)]
    pub force: bool,
}

// ── Report ───────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct ReportArgs {
    /// Report artifact file name
    #[arg(long, default_value = "report.txt")]
    pub artifact: String,
}

// ── Compare ──────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct CompareArgs {
    /// Source base ref
    #[arg(long)]
    pub from: String,

    /// Target base ref
    #[arg(long)]
    pub to: String,
}

// ── Explain ──────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct ExplainEditArgs {
    /// Edit location in PATH:LINE form
    pub query: String,
}

#[derive(clap::Args)]
pub struct ExplainSymbolArgs {
    /// Kconfig symbol, with or without CONFIG_ prefix
    pub symbol: String,
}

#[derive(clap::Args)]
pub struct ExplainFeatureArgs {
    /// Profile name
    #[arg(long, default_value = "default")]
    pub profile: String,

    /// Named feature from features.remove or features.preserve
    pub feature: String,
}

#[derive(clap::Args)]
pub struct ExplainAbiArgs {
    /// Profile name
    #[arg(long, default_value = "default")]
    pub profile: String,
}

#[derive(clap::Args)]
pub struct MatrixArgs {
    /// Profile name
    #[arg(long, default_value = "default")]
    pub profile: String,

    /// Select verification matrix preset
    #[arg(long = "matrix")]
    pub matrix: Option<String>,
}

#[derive(clap::Args)]
pub struct SelftestArgs {
    /// Profile name
    #[arg(long, default_value = "default")]
    pub profile: String,

    /// Existing kernel source tree to test
    #[arg(long)]
    pub tree: String,

    /// Select verification matrix preset
    #[arg(long = "matrix")]
    pub matrix: Option<String>,
}

#[derive(clap::Args)]
pub struct FuzzFixturesArgs {
    /// Fixture output directory
    #[arg(long, default_value = "fuzz-fixtures")]
    pub out: String,
}
