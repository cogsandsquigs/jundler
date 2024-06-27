use super::lock::{Checksum, NodeExecutableMeta};
use super::Error;
use crate::builder::platforms::{Arch, Os};
use nom::branch::alt;
use nom::character::complete::one_of;
use nom::combinator::recognize;
use nom::multi::{many0, many1};
use nom::sequence::{terminated, tuple};
use nom::{
    bytes::complete::{tag, take},
    character::complete::char,
    IResult,
};
use semver::Version;

// TODO: Consider replacing nom with winnow: https://docs.rs/winnow/latest/winnow/

/// Parses a singular checksum file. Note that if there are any errors parsing a checksum file line, that line will be
/// skipped and not included in the final result. If there is an error parsing every line, an error will be returned.
pub fn parse_checksum_file(input: &str) -> Result<Vec<(Checksum, NodeExecutableMeta)>, Error> {
    let mut entries = Vec::new();

    for line in input.lines() {
        match parse_checksum_file_entry(line) {
            Ok((_, entry)) => entries.push(entry),
            Err(err) => {
                println!("Failed to parse checksum file entry: {}: {}", line, err);
                continue;
            }
        }
    }

    // If there are no entries, return an error
    if entries.is_empty() {
        return Err(Error::UnparseableChecksumFile);
    }

    Ok(entries)
}

/// A function to parse a single line in a checksum file using Nom
fn parse_checksum_file_entry(input: &str) -> IResult<&str, (Checksum, NodeExecutableMeta)> {
    let (input, checksum) = parse_checksum(input)?;
    let (input, _) = tag("  node-v")(input)?;
    let (input, version) = parse_version(input)?;
    let (input, _) = char('-')(input)?;
    let (input, os) = parse_os(input)?;
    let (input, _) = char('-')(input)?;
    let (input, arch) = parse_arch(input)?;
    let (input, _) = alt((tag(".tar.gz"), tag(".zip")))(input)?;

    Ok((input, (checksum, NodeExecutableMeta { version, arch, os })))
}

/// Parses a checksum
fn parse_checksum(input: &str) -> IResult<&str, Checksum> {
    // Get the first 64 characters of the input --- these are the hex characters of the checksum
    let (input, checksum_str) = take(64usize)(input)?;

    let mut checksum: Checksum = [0u8; 32];

    // Convert the hex characters to a [u8; 32]
    hex::decode_to_slice(checksum_str, &mut checksum as &mut [u8])
        .expect("Node.js checksums should always be 64 characters!");

    Ok((input, checksum))
}

/// Parses a semver version
fn parse_version(input: &str) -> IResult<&str, semver::Version> {
    let (input, version_str) = tuple((
        parse_decimal_number,
        char('.'),
        parse_decimal_number,
        char('.'),
        parse_decimal_number,
    ))(input)?;

    let version_str = version_str.0.to_owned() + "." + version_str.2 + "." + version_str.4;

    let version = semver::Version::parse(&version_str)
        .expect("Node.js versions should always conform to semver!");

    Ok((input, version))
}

/// Parses an operating system
fn parse_os(input: &str) -> IResult<&str, Os> {
    let (input, os_str) = alt((tag("win"), tag("darwin"), tag("linux")))(input)?;

    let os = match os_str {
        "win" => Os::Windows,
        "darwin" => Os::MacOS,
        "linux" => Os::Linux,
        _ => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    };

    Ok((input, os))
}

/// Parses an architecture
fn parse_arch(input: &str) -> IResult<&str, Arch> {
    let (input, arch_str) = alt((
        tag("arm64"),
        tag("aarch64"),
        tag("x64"),
        tag("x86"),
        tag("x86_64"),
    ))(input)?;

    let arch = match arch_str {
        "arm64" | "aarch64" => Arch::Arm64,
        "x64" | "x86_64" => Arch::X64,
        "x86" => Arch::X86,
        _ => {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )))
        }
    };

    Ok((input, arch))
}

/// Parse a decimal number
fn parse_decimal_number(input: &str) -> IResult<&str, &str> {
    recognize(many1(terminated(one_of("0123456789"), many0(char('_')))))(input)
}
