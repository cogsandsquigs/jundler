mod builder;
mod cli;
mod js_config;

use anyhow::{Context, Result};
use builder::Builder;
use clap::Parser;
use cli::Args;
use std::{env, fs, path::PathBuf};

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
        get_cache_dir(),
        args.bundle,
    )?;

    builder.build()?;

    Ok(())
}

/// Get the user's cache directory.
/// TODO: Error handling
fn get_cache_dir() -> PathBuf {
    let cache_dir = dirs::cache_dir().unwrap().join("jundler");

    // If the dir doesn't exist, make it
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).unwrap();
    }

    cache_dir
}
