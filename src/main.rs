mod builder;
mod cli;
mod js_config;
mod ui;

use anyhow::{Context, Result};
use builder::Builder;
use clap::Parser;
use cli::Args;
use std::{env, fs, path::PathBuf, thread::sleep, time::Duration};
use ui::Interface;

fn main() -> Result<()> {
    // Default the log level to info if it's not set.
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .init();

    let args = Args::parse();

    let mut builder = Builder::new(get_cache_dir())?;

    let mut interface = Interface::new();

    let mut spinner = interface.spawn_spinner("Building project...".to_string());

    spinner.start();

    sleep(Duration::from_secs(5));

    spinner.close();

    todo!();

    match args.action {
        cli::Action::Build {
            project_dir,
            node_version,
            os,
            arch,
            bundle,
        } => {
            let project_dir: std::path::PathBuf = project_dir
                .canonicalize()
                .context("Invalid project directory!")?
                .to_path_buf();

            builder.build(&project_dir, node_version, os, arch, bundle)?;
        }
        cli::Action::Clean => builder.clean_cache()?,
    };

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
