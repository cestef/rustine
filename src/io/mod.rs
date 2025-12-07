pub mod check;
pub mod fs;

pub use check::{can_write, exists};
pub use fs::{filename, read, read_streaming, write};
