mod builder;
mod cli;
mod js_config;

use anyhow::{Context, Result};
use builder::Builder;
use clap::Parser;
use cli::Args;
use js_config::{PackageConfig, SEAConfig};
use std::env;
use std::fs::File;
use std::path::Path;

fn main() -> Result<()> {
    // Default the log level to info if it's not set.
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .init();

    let args = Args::parse();

    let project_dir: std::path::PathBuf = args
        .project_dir
        .canonicalize()
        .context("Invalid project directory!")?
        .to_path_buf();

    let (sea_config, package_config) = get_configs(&project_dir)?;

    let mut builder = Builder::new(
        project_dir,
        args.node_version,
        args.os,
        args.arch,
        sea_config,
        package_config,
        args.bundle,
    )?;

    builder.build()?;

    Ok(())
}

/// Gets the `sea-config.json` and `package.json` configurations from the project directory.
fn get_configs(project_dir: &Path) -> Result<(SEAConfig, PackageConfig)> {
    let sea_config = serde_json::from_reader(
        File::open(project_dir.join("sea-config.json"))
            .context("Could not find or open the `sea-config.json` file!")?,
    )
    .context("Could not parse the `sea-config.json` file!")?;

    let package_config = serde_json::from_reader(
        File::open(project_dir.join("package.json"))
            .context("Could not find or open the `sea-config.json` file!")?,
    )
    .context("Could not parse the `package.json` file!")?;

    Ok((sea_config, package_config))
}
