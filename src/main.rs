mod builder;
mod cli;
mod js_config;
mod ui;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use std::{env, fs, path::PathBuf};

fn main() -> Result<()> {
    amend_panic_with_issue_msg();

    // Default the log level to info if it's not set.
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();

    cli.run()?;

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

/// OVerride panic messages with a message to submit an issue at the git repo.
fn amend_panic_with_issue_msg() {
    let default_panic = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        default_panic(info);

        println!();

        println!("{}", console::style("This panic most likely should not have happened (unless your OS is very weird). However, Jundler is experimental and these types of things can happen.").yellow());
        println!("{}", console::style("If you feel that this panic was unjustified or unreasonable, submit an issue at https://github.com/cogsandsquigs/jundler if you encounter any problems.").yellow());
        println!("{}", console::style("If you aren't sure what to do, submit an issue just in case. Better safe than sorry ;).").yellow());
    }));
}
