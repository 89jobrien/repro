//! Reproducible container image builder.
//!
//! Organized as hexagonal architecture:
//! - **Ports**: `CommandRunner`, `IdempotentRunner`, `RuntimeResolver`, `RuntimeStrategy`
//! - **Adapters**: `ProcessRunner`, `DryRunner`, `MockRunner`, `WhichResolver`, `MockResolver`
//! - **Domain**: `BuildConfig`, `BuildParams`, `resolve_config`
//! - **Strategies**: `DockerStrategy`, `PodmanStrategy`

pub mod config;
pub mod resolver;
pub mod runner;
pub mod strategy;

// Re-export public API
pub use config::{BuildConfig, BuildParams};
pub use resolver::{ResolvedRuntime, RuntimeResolver, WhichResolver};
pub use runner::{CommandRunner, DryRunner, IdempotentRunner, ProcessRunner};
pub use strategy::{CommandSpec, DockerStrategy, PodmanStrategy, RuntimeStrategy};

#[cfg(any(test, feature = "testutil"))]
pub use resolver::MockResolver;
#[cfg(any(test, feature = "testutil"))]
pub use runner::MockRunner;

use anyhow::{Result, bail};
use tracing::info;

/// Orchestrates a reproducible build using injected dependencies.
pub struct Builder<R: CommandRunner> {
    pub config: BuildConfig,
    pub runner: R,
}

impl Builder<ProcessRunner> {
    /// Create a builder that executes real commands.
    pub fn new(params: BuildParams) -> Result<Self> {
        let config = config::resolve_config(params, &WhichResolver)?;
        Ok(Self {
            config,
            runner: ProcessRunner,
        })
    }

    /// Create a builder that logs commands without executing.
    pub fn dry(params: BuildParams) -> Result<Builder<DryRunner>> {
        let config = config::resolve_config(params, &WhichResolver)?;
        Ok(Builder {
            config,
            runner: DryRunner,
        })
    }
}

impl<R: CommandRunner + IdempotentRunner> Builder<R> {
    /// Create a builder with a custom runner and resolver (for testing).
    pub fn with_deps(
        params: BuildParams,
        resolver: &dyn RuntimeResolver,
        runner: R,
    ) -> Result<Self> {
        let config = config::resolve_config(params, resolver)?;
        Ok(Self { config, runner })
    }

    /// Execute the reproducible build.
    pub fn build(&self) -> Result<()> {
        log_config(&self.config);

        let strategy: Box<dyn RuntimeStrategy> = match self.config.runtime.as_str() {
            "docker" => Box::new(DockerStrategy),
            "podman" => Box::new(PodmanStrategy),
            other => bail!("unsupported runtime: {other}"),
        };

        let commands = strategy.build_commands(&self.config);
        for spec in &commands {
            if spec.allow_failure {
                self.runner.run_no_check(&spec.args);
            } else {
                self.runner.run(&spec.args)?;
            }
        }
        Ok(())
    }
}

fn log_config(c: &BuildConfig) {
    info!(
        runtime = %c.runtime,
        buildkit_image = %c.buildkit_image,
        rootless = c.rootless,
        use_cache = c.use_cache,
        context = %c.context.display(),
        dockerfile = %c.file.as_deref().map(|p| p.display().to_string())
            .unwrap_or_else(|| "(not provided)".into()),
        output = %c.output.display(),
        sde = c.source_date_epoch,
        tag = %c.tag.as_deref().unwrap_or("(not provided)"),
        build_args = %if c.build_args.is_empty() {
            "(not provided)".into()
        } else {
            c.build_args.join(",")
        },
        platform = %c.platform.as_deref().unwrap_or("(default)"),
        "build: configuration resolved"
    );
}
