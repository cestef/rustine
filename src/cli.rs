use std::path::PathBuf;

use facet::Facet;
use facet_args as args;

#[derive(Facet)]
pub struct Opts {
    #[facet(args::subcommand)]
    pub cmd: Command,
}

#[derive(Facet, Debug)]
#[repr(C)]
pub enum Command {
    Help {
        #[facet(default, args::positional)]
        command: Option<String>,
    },
    Generate {
        #[facet(args::positional)]
        base: PathBuf,

        #[facet(args::positional)]
        patched: PathBuf,

        #[facet(default, args::named, args::short = 'o')]
        output: Option<PathBuf>,

        /// Enable verbose output
        #[facet(default, args::named, args::short = 'v')]
        verbose: bool,

        /// Suppress all output except errors
        #[facet(default, args::named, args::short = 'q')]
        quiet: bool,

        /// Overwrite output file if it exists
        #[facet(default, args::named, args::short = 'f')]
        force: bool,

        /// Embed checksums for verification
        #[facet(default, args::named)]
        checksum: bool,

        /// Include reverse patch for bidirectional patching
        #[facet(default, args::named, args::short = 'r')]
        reverse: bool,
    },
    Apply {
        #[facet(args::positional)]
        base: PathBuf,

        #[facet(args::positional)]
        patch: PathBuf,

        #[facet(default, args::named, args::short = 'o')]
        output: Option<PathBuf>,

        /// Verify patch can be applied without writing output
        #[facet(default, args::named)]
        dry_run: bool,

        /// Apply patch in reverse
        #[facet(default, args::named, args::short = 'R')]
        reverse: bool,

        /// Enable verbose output
        #[facet(default, args::named, args::short = 'v')]
        verbose: bool,

        /// Suppress all output except errors
        #[facet(default, args::named, args::short = 'q')]
        quiet: bool,

        /// Overwrite output file if it exists
        #[facet(default, args::named, args::short = 'f')]
        force: bool,

        /// Verify checksums if present
        #[facet(default, args::named)]
        verify: bool,
    },
    Inspect {
        #[facet(args::positional)]
        patch: PathBuf,

        /// Enable verbose output
        #[facet(default, args::named, args::short = 'v')]
        verbose: bool,
    },
}
