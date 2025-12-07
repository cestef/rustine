use facet::Facet;
use miette::{Diagnostic, NamedSource, SourceSpan};
use std::{fmt::Display, path::PathBuf, sync::OnceLock};

#[derive(Diagnostic, thiserror::Error, Debug)]
pub enum RustineErrorKind {
    #[error("failed to load config")]
    #[diagnostic(code(rustine::config))]
    Config {
        #[diagnostic_source]
        src: Box<dyn Diagnostic + Send + Sync>,
    },

    #[error("command not found: {name}")]
    #[diagnostic(code(rustine::command_not_found), help("available commands: {}", facet_reflect::peek_enum_variants(crate::cli::Command::SHAPE).expect("this is an enum").iter().map(|e| e.name.to_lowercase()).collect::<Vec<_>>().join(", ")))]
    CommandNotFound { name: String },

    #[error(transparent)]
    #[diagnostic(code(rustine::io))]
    Io(#[from] std::io::Error),

    #[error("output file already exists: {path}")]
    #[diagnostic(code(rustine::file_exists), help("use --force to overwrite"))]
    FileExists { path: String },

    #[error("failed to parse patch file")]
    #[diagnostic(
        code(rustine::invalid_patch),
        help("ensure the patch file is a valid bsdiff4 patch and not corrupted")
    )]
    InvalidPatch {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to apply patch")]
    #[diagnostic(
        code(rustine::patch_failed),
        help("ensure the base file matches the expected input for this patch")
    )]
    PatchFailed {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to generate patch")]
    #[diagnostic(code(rustine::diff_failed))]
    DiffFailed {
        #[source]
        source: std::io::Error,
    },

    #[error("input file not found: {path}")]
    #[diagnostic(
        code(rustine::file_not_found),
        help("ensure the file path is correct and the file exists")
    )]
    FileNotFound { path: String },

    #[error("cannot read input file: {path}")]
    #[diagnostic(code(rustine::file_unreadable))]
    FileUnreadable {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("checksum mismatch")]
    #[diagnostic(
        code(rustine::checksum_mismatch),
        help("the file may have been corrupted or modified")
    )]
    ChecksumMismatch { expected: String, actual: String },
}

#[derive(Debug)]
pub struct RustineError {
    pub kind: RustineErrorKind,
    pub ctx: Box<RustineErrorContext>,
}

impl RustineError {
    pub fn new(kind: RustineErrorKind, ctx: RustineErrorContext) -> Self {
        Self {
            kind,
            ctx: Box::new(ctx),
        }
    }
}

pub type Result<T> = std::result::Result<T, RustineError>;

impl From<RustineErrorKind> for RustineError {
    fn from(val: RustineErrorKind) -> Self {
        RustineError {
            ctx: Box::new(RustineErrorContext::default()),
            kind: val,
        }
    }
}

// Generic macro to bridge From implementations from RustineErrorKind to RustineError
// For any type T that implements Into<RustineErrorKind>, this generates From<T> for RustineError
macro_rules! bridge {
    ($($t:ty),* $(,)?) => {
        $(
            impl From<$t> for RustineError {
                fn from(val: $t) -> Self {
                    Self::from(RustineErrorKind::from(val))
                }
            }
        )*
    };
}

bridge! {
    std::io::Error,
}

#[derive(Debug)]
pub struct RustineErrorContext {
    pub path: Option<PathBuf>,
    pub span: Option<SourceSpan>,
    pub contents: Option<String>,
    named_source: OnceLock<NamedSource<String>>,
}

impl RustineErrorContext {
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_contents(mut self, contents: impl Into<String>) -> Self {
        self.contents = Some(contents.into());
        self
    }
}

impl Default for RustineErrorContext {
    fn default() -> Self {
        Self {
            path: None,
            span: None,
            contents: None,
            named_source: OnceLock::new(),
        }
    }
}

impl std::error::Error for RustineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl Display for RustineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl Diagnostic for RustineError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.kind.code()
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.kind.severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.kind.help()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.kind.url()
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        if let Some(ref contents) = self.ctx.contents {
            if let Some(ref path) = self.ctx.path {
                let named_source = self
                    .ctx
                    .named_source
                    .get_or_init(|| NamedSource::new(path.to_string_lossy(), contents.clone()));
                Some(named_source as &dyn miette::SourceCode)
            } else {
                Some(contents as &dyn miette::SourceCode)
            }
        } else {
            None
        }
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        self.ctx.span.map(|span| {
            Box::new(std::iter::once(miette::LabeledSpan::at(span, "right here")))
                as Box<dyn Iterator<Item = miette::LabeledSpan>>
        })
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        self.kind.related()
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        self.kind.diagnostic_source()
    }
}
