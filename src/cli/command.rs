//! Top-level clap parser and command enum shape.

use clap::{Parser, Subcommand};

use super::args::{
    CompareArgs, ExplainAbiArgs, ExplainEditArgs, ExplainFeatureArgs, ExplainSymbolArgs,
    FeatureImpactArgs, FuzzFixturesArgs, GenerateArgs, InitArgs, MatrixArgs, PlanArgs,
    PublishArgs, ReduceTreeArgs, ReportArgs, ResolveArgs, SelftestArgs, ValidateConfigArgs,
};

#[derive(Parser)]
#[command(name = "kslim", version, about = "Linux kernel slimdown tool")]
pub struct Cli {
    /// Reject network-backed endpoints for this invocation
    #[arg(long = "no-network", global = true)]
    pub no_network: bool,

    /// Use lockfile-resolved inputs without refreshing upstream refs
    #[arg(long = "offline", global = true)]
    pub offline: bool,

    /// Explain a changed PATH[:LINE] or Kconfig symbol from the latest reports
    #[arg(long = "explain")]
    pub explain: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new kslim project
    Init(InitArgs),

    /// Validate project config and profile files
    ValidateConfig(ValidateConfigArgs),

    /// Resolve and print the generate plan without mutating output
    Plan(PlanArgs),

    /// Explain profile feature removal/preservation impact
    FeatureImpact(FeatureImpactArgs),

    /// Apply the reducer directly to an existing kernel tree
    ReduceTree(ReduceTreeArgs),

    /// Manage direct upstream access
    #[command(subcommand)]
    Upstream(UpstreamCommands),

    /// Resolve base ref
    #[command(subcommand)]
    Base(BaseCommands),

    /// Generate output repo from upstream
    Generate(GenerateArgs),

    /// Publish output repo
    Publish(PublishArgs),

    /// Print a generated report artifact
    Report(ReportArgs),

    /// Show project status
    Status,

    /// Clear stale non-authoritative attempt metadata
    Repair,

    /// Explain why an edit touched PATH:LINE
    ExplainEdit(ExplainEditArgs),

    /// Explain how a Kconfig symbol was treated
    ExplainSymbol(ExplainSymbolArgs),

    /// Explain named feature intent and resolved impact
    ExplainFeature(ExplainFeatureArgs),

    /// Explain ABI/UAPI policy and sensitive removal decisions
    ExplainAbi(ExplainAbiArgs),

    /// Show selected build/runtime verification matrix
    Matrix(MatrixArgs),

    /// Run selected selftests against an existing kernel tree
    Selftest(SelftestArgs),

    /// Write deterministic fuzz fixture seed corpus
    FuzzFixtures(FuzzFixturesArgs),

    /// Compare two upstream snapshots
    Compare(CompareArgs),
}

// ── Upstream ─────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum UpstreamCommands {
    /// Verify direct read-only upstream access
    Sync,
}

// ── Base ─────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum BaseCommands {
    /// Resolve base ref from profile
    Resolve(ResolveArgs),
}
