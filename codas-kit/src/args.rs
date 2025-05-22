//! Command-line argument parsing and handling.
use std::{
    io::{BufReader, Read},
    path::PathBuf,
};

use clap::{Parser, Subcommand};

pub mod cryptography;
pub mod inspect;

/// Command-line arguments for the `coda` terminal interface.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Command to execute.
    #[command(subcommand)]
    command: Command,
}

impl Args {
    /// Execute the subcommand in these arguments.
    pub fn execute(self) {
        match self.command {
            Command::Compile { .. } => todo!(),
            Command::Inspect { .. } => todo!(),
            Command::Crypt(cryptography_command) => {
                cryptography::execute_cryptography_command(cryptography_command);
            }
        }
    }
}

/// Subcommand passed to [Args].
#[derive(Subcommand, Debug, Clone)]
#[command()]
pub enum Command {
    /// Compile language-specific bindings for codas.
    Compile(CompileCommand),

    /// Inspect binary coda-encoded data.
    Inspect(InspectCommand),

    /// Cryptography-related utilities.
    #[command(subcommand)]
    Crypt(CryptographyCommand),
}

/// Arguments passed to [Command::Compile].
#[derive(clap::Args, Debug, Clone)]
pub struct CompileCommand {
    /// The path to search for codas in.
    ///
    /// If unspecified, the current working directory is used.
    #[arg(short, long, default_value_os_t = get_working_directory())]
    source: PathBuf,

    /// The path to output compiled code to.
    ///
    /// If unspecified, the `target` directory in
    /// the current working directory is used.
    #[arg(short, long, default_value_os_t = get_working_directory().join("target"))]
    target: PathBuf,
}

/// Arguments passed to [Command::Inspect].
#[derive(clap::Args, Debug, Clone)]
pub struct InspectCommand {
    /// Path to a file containing coda-encoded data.
    ///
    /// If unspecified, data will be read from standard input.
    #[arg(short, long)]
    source: Option<PathBuf>,
}

/// Subcommand passed to [Command::Crypt].
#[derive(Subcommand, Debug, Clone)]
#[command()]
pub enum CryptographyCommand {
    /// Hash data into a [codas::types::cryptography::HashBytes].
    Hash {
        /// Path to a file containing data to hash.
        ///
        /// If unspecified, data will be read from standard input.
        #[arg(short, long)]
        source: Option<PathBuf>,
    },

    /// Generate a cryptographic keypair for signing data.
    Keygen {
        /// Passphrase to encrypt the generated keypair with.
        #[arg(short, long)]
        passphrase: String,
    },

    /// Sign data into a [codas::types::cryptography::SignatureBytes].
    Sign {
        /// Path to a file containing the signing keypair to use.
        #[arg(short, long)]
        keypair: PathBuf,

        /// Passphrase to decrypt the signing keypair with.
        #[arg(short, long)]
        passphrase: String,

        /// Path to a file containing data to sign.
        ///
        /// If unspecified, data will be read from standard input.
        #[arg(short, long)]
        source: Option<PathBuf>,
    },
}

/// Returns the working directory of the current executable.
fn get_working_directory() -> PathBuf {
    std::env::current_dir().unwrap()
}

/// Opens the contents of the file at `path` for reading
///
/// Iff `path` is `None`, the contents of standard
/// input will be opened for reading until there is
/// no more input.
fn open_file_or_stdin(path: Option<PathBuf>) -> std::io::Result<Box<dyn Read>> {
    match path {
        Some(path) => Ok(Box::new(BufReader::new(std::fs::File::open(path)?))),
        None => Ok(Box::new(BufReader::new(std::io::stdin()))),
    }
}
