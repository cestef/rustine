use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

use super::Level;

/// Context for UI operations - eliminates spinner duplication
pub struct Ctx {
    spin: Option<ProgressBar>,
    level: Level,
}

impl Ctx {
    pub fn new(level: Level) -> Self {
        Self {
            spin: if level.quiet() {
                None
            } else {
                Some(make_spin())
            },
            level,
        }
    }

    /// Update spinner message
    pub fn msg(&self, text: &str) {
        if let Some(ref s) = self.spin {
            s.set_message(text.to_string());
        }
    }

    /// Finish spinner with message
    pub fn done(&self, text: &str) {
        if let Some(ref s) = self.spin {
            s.finish_with_message(text.to_string());
        }
    }

    pub fn level(&self) -> Level {
        self.level
    }
}

/// Create spinner with standard style
fn make_spin() -> ProgressBar {
    let s = ProgressBar::new_spinner();
    s.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan.bold} {msg:.dim}")
            .unwrap(),
    );
    s.enable_steady_tick(Duration::from_millis(80));
    s
}
