mod builder;
mod oci;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "repro", about = "Reproducible container image builder")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Perform a reproducible container image build
    Build(BuildArgs),
    /// Analyze an OCI image tarball
    Analyze(AnalyzeArgs),
}

#[derive(Parser)]
struct BuildArgs {
    /// Path to the build context
    context: String,

    /// Container runtime (docker or podman)
    #[arg(long, value_parser = ["docker", "podman"])]
    runtime: Option<String>,

    /// Date/time in ISO format for image layer timestamps
    #[arg(long)]
    datetime: Option<String>,

    /// BuildKit container image (NAME:TAG@DIGEST)
    #[arg(long)]
    buildkit_image: Option<String>,

    /// Unix timestamp for image layer timestamps
    #[arg(long, alias = "sde")]
    source_date_epoch: Option<i64>,

    /// Do not use cached images
    #[arg(long)]
    no_cache: bool,

    /// Run BuildKit in rootless mode (Podman only)
    #[arg(long)]
    rootless: bool,

    /// Pathname of a Dockerfile
    #[arg(short = 'f', long)]
    file: Option<String>,

    /// Path to save OCI tarball
    #[arg(short = 'o', long, default_value = "image.tar")]
    output: Option<String>,

    /// Tag the built image
    #[arg(short = 't', long)]
    tag: Option<String>,

    /// Set build-time variables (ARG=VALUE)
    #[arg(long = "build-arg")]
    build_arg: Vec<String>,

    /// Append annotation to the image (KEY=VALUE)
    #[arg(long)]
    annotation: Vec<String>,

    /// Set platform for the image
    #[arg(long)]
    platform: Option<String>,

    /// Extra arguments for BuildKit (Podman only)
    #[arg(long)]
    buildkit_args: Option<String>,

    /// Extra arguments for Docker Buildx (Docker only)
    #[arg(long)]
    buildx_args: Option<String>,

    /// Print commands without executing
    #[arg(long)]
    dry: bool,
}

#[derive(Parser)]
struct AnalyzeArgs {
    /// Path to OCI image tarball
    tarball: PathBuf,

    /// Expected digest to verify against
    #[arg(long)]
    expected_image_digest: Option<String>,

    /// Show full file contents
    #[arg(long)]
    show_contents: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Build(args) => {
            let params = builder::BuildParams {
                context: args.context,
                runtime: args.runtime,
                source_date_epoch: args.source_date_epoch,
                datetime: args.datetime,
                buildkit_image: args.buildkit_image,
                no_cache: args.no_cache,
                rootless: args.rootless,
                file: args.file,
                output: args.output,
                tag: args.tag,
                build_args: args.build_arg,
                annotations: args.annotation,
                platform: args.platform,
                buildkit_args: args.buildkit_args,
                buildx_args: args.buildx_args,
                dry: args.dry,
            };
            builder::Builder::new(params)?.build()
        }
        Commands::Analyze(args) => {
            let parsed = oci::parse_tarball(&args.tarball)?;
            oci::print_info(&parsed, args.show_contents);
            if let Some(ref expected) = args.expected_image_digest {
                oci::verify_digest(&parsed, expected)?;
            }
            Ok(())
        }
    }
}
