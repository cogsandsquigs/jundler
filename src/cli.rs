use crate::builder::node::{Arch, Os};
use clap::{Parser, Subcommand};
use semver::Version;
use std::path::PathBuf;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The subcommand to run.
    #[clap(subcommand)]
    pub action: Action,
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
