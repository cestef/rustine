/// Represents a change in the binary data
#[derive(Debug)]
pub struct ByteChange {
    pub offset: usize,
    pub old_bytes: Vec<u8>,
    pub new_bytes: Vec<u8>,
}

/// Find regions where bytes differ between old and new data
pub fn find_changes(old: &[u8], new: &[u8]) -> Vec<ByteChange> {
    let mut changes = Vec::new();
    let min_len = old.len().min(new.len());

    let mut i = 0;
    while i < min_len {
        // Find start of difference
        if old[i] != new[i] {
            let start = i;
            let mut end = i;

            // Find end of continuous difference (with small gap tolerance)
            while end < min_len
                && (old[end] != new[end] || (end + 1 < min_len && old[end + 1] != new[end + 1]))
            {
                end += 1;
            }

            changes.push(ByteChange {
                offset: start,
                old_bytes: old[start..end].to_vec(),
                new_bytes: new[start..end].to_vec(),
            });

            i = end;
        } else {
            i += 1;
        }
    }

    // Handle size differences
    if old.len() != new.len() {
        if new.len() > old.len() {
            changes.push(ByteChange {
                offset: old.len(),
                old_bytes: vec![],
                new_bytes: new[old.len()..].to_vec(),
            });
        } else {
            changes.push(ByteChange {
                offset: new.len(),
                old_bytes: old[new.len()..].to_vec(),
                new_bytes: vec![],
            });
        }
    }

    changes
}

/// Format bytes as hex with ASCII preview using pretty-hex
pub fn format_hex_dump(bytes: &[u8], max_bytes: usize) -> String {
    use pretty_hex::{HexConfig, PrettyHex};

    let display_bytes = &bytes[..bytes.len().min(max_bytes)];

    let cfg = HexConfig {
        title: false,
        width: 16,
        group: 2,
        chunk: 8,
        ..HexConfig::default()
    };

    let mut hex_str = format!("{}", display_bytes.hex_conf(cfg));

    // Remove trailing newline if present
    if hex_str.ends_with('\n') {
        hex_str.pop();
    }

    if bytes.len() > max_bytes {
        format!("{} ... ({} more bytes)", hex_str, bytes.len() - max_bytes)
    } else {
        hex_str
    }
}

/// Generate a preview summary of changes
pub fn preview_summary(changes: &[ByteChange]) -> String {
    if changes.is_empty() {
        return "No changes detected".to_string();
    }

    let total_old_bytes: usize = changes.iter().map(|c| c.old_bytes.len()).sum();
    let total_new_bytes: usize = changes.iter().map(|c| c.new_bytes.len()).sum();
    let regions = changes.len();

    format!(
        "{} change region{}, {} bytes → {} bytes",
        regions,
        if regions == 1 { "" } else { "s" },
        total_old_bytes,
        total_new_bytes
    )
}
