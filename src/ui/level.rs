#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Level {
    Quiet,
    Normal,
    Verbose,
}

impl Level {
    pub fn from_flags(verbose: bool, quiet: bool) -> Self {
        match (quiet, verbose) {
            (true, _) => Level::Quiet,
            (_, true) => Level::Verbose,
            _ => Level::Normal,
        }
    }

    pub fn quiet(&self) -> bool {
        matches!(self, Level::Quiet)
    }
}
