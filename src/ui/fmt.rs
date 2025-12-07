use owo_colors::OwoColorize;

/// Format bytes with human-readable units
pub fn bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = n as f64;
    let mut idx = 0;

    while size >= 1024.0 && idx < UNITS.len() - 1 {
        size /= 1024.0;
        idx += 1;
    }

    if idx == 0 {
        format!("{} {}", n.cyan(), UNITS[0].dimmed())
    } else {
        format!("{:.2} {}", size.cyan(), UNITS[idx].dimmed())
    }
}

/// Calculate reduction percentage
pub fn reduce(orig: u64, comp: u64) -> f64 {
    if orig > 0 {
        100.0 - (comp as f64 / orig as f64 * 100.0)
    } else {
        0.0
    }
}

/// Format reduction percentage with color
pub fn reduction(percent: f64) -> String {
    if percent > 80.0 {
        format!("{:.2}%", percent).bright_green().to_string()
    } else if percent > 50.0 {
        format!("{:.2}%", percent).green().to_string()
    } else if percent > 20.0 {
        format!("{:.2}%", percent).yellow().to_string()
    } else {
        format!("{:.2}%", percent).dimmed().to_string()
    }
}

/// Success marker (✓ green + bold)
pub fn ok() -> String {
    "✓".green().bold().to_string()
}

/// Info marker (● blue)
pub fn info() -> String {
    "●".blue().to_string()
}

/// Path formatter with subtle styling
pub fn path(p: impl std::fmt::Display) -> String {
    format!("{}", p.cyan().bold())
}
