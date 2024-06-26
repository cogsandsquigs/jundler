mod builder;
mod cli;
mod js_config;

use anyhow::{Context, Result};
use builder::Builder;
use clap::Parser;
use cli::Args;
use std::env;

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

    let mut builder = Builder::new(
        project_dir,
        args.node_version,
        args.os,
        args.arch,
        args.bundle,
        args.custom_node,
    )?;

    builder.build()?;

    Ok(())
}
