use crate::builder::{
    platforms::{Arch, Os},
    Builder,
};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::HumanDuration;
use semver::Version;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The subcommand to run.
    #[clap(subcommand)]
    pub action: Action,
}

impl Cli {
    /// Gets the action that's currently being performed, as a human-readable string.
    pub fn action(&self) -> &str {
        match &self.action {
            Action::Clean => "Cleaning",
            Action::Build { .. } => "Building",
        }
    }

    /// Runs the command-line interface for `dotbak` based on the user's input.
    pub fn run(&self) -> Result<()> {
        let started = Instant::now();

        println!("⏳ {}...", self.action());

        let mut builder = Builder::new(get_cache_dir())?;

        builder
            .interface
            .warn("This is experimental and may not work as expected.");
        builder.interface.warn("Submit an issue at https://github.com/cogsandsquigs/jundler if you encounter any problems.");

        // Run the action.
        match &self.action {
            Action::Build {
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

                builder.build(&project_dir, node_version.clone(), *os, *arch, *bundle)?;
            }

            Action::Clean => builder.clean_cache()?,
        }

        println!(
            "✨ Done! {}",
            console::style(format!("[{}]", HumanDuration(started.elapsed())))
                .bold()
                .dim(),
        );

        Ok(())
    }
}

/// An enum of actions to perform.
#[derive(Subcommand, Debug)]
pub enum Action {
    /// Build the project.
    Build {
        /// The path to the directory where the project to build is located. Note that the output binary will be
        /// placed in this directory as well.
        #[clap(default_value = ".")]
        project_dir: PathBuf,

        /// The version of Node.js you want to bundle with your application. This MUST match your installed/currently
        /// used Node.js version. Note that there should not be any "v" prefix.
        #[arg(short, long, default_value_t = current_node_version())]
        node_version: Version,

        /// The platform you're building for.
        #[arg(short, long, default_value_t = Os::default())]
        os: Os,

        /// The architecture you're building for.
        #[arg(short, long, default_value_t = Arch::default())]
        arch: Arch,

        /// Bundle the project into a single JS file instead of just compiling the `sea-config.json` main entrypoint. This
        /// will also bundle the Node.js runtime.
        #[arg(short, long, default_value_t = false)]
        bundle: bool,
    },

    /// Clean the project.
    Clean,
}

fn current_node_version() -> Version {
    let output = std::process::Command::new("node")
        .arg("--version")
        .output()
        .expect("Failed to get Node.js version!");

    Version::parse(
        String::from_utf8(output.stdout)
            .expect("Failed to parse Node.js version!")
            // Remove any whitespace.
            .trim()
            // Remove the "v" prefix.
            .strip_prefix('v')
            .expect("There should be a 'v' prefix for a nodejs version!"),
    )
    .expect("Failed to parse node version as semver!")
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
