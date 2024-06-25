use core::fmt;
use std::{
    default,
    env::consts::{ARCH, OS},
};

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    #[clap(alias = "darwin")]
    MacOS,
    Linux,
    Windows,
}

impl Default for Os {
    fn default() -> Self {
        match OS {
            "macos" | "darwin" => Os::MacOS,
            "linux" => Os::Linux,
            "windows" => Os::Windows,
            _ => panic!("Building for unsupported os target!"),
        }
    }
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Os::MacOS => write!(f, "darwin"),
            Os::Linux => write!(f, "linux"),
            Os::Windows => write!(f, "win"),
        }
    }
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    #[clap(alias = "x64")]
    #[clap(alias = "x86_64")]
    X64,

    #[clap(alias = "x86")]
    X86,

    Arm64,
}

impl default::Default for Arch {
    fn default() -> Self {
        match ARCH {
            "x86" => Arch::X86, // "x86" is not a valid value for ARCH, but we'll include it for completeness
            "x64" | "x86_64" => Arch::X64,
            "arm" | "aarch64" => Arch::Arm64,
            _ => panic!("Building for unsupported architecture target!"),
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Arch::X64 => write!(f, "x64"),
            Arch::X86 => write!(f, "x86"),
            Arch::Arm64 => write!(f, "arm64"),
        }
    }
}
