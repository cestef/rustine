pub mod check;
pub mod fs;

pub use check::{can_write, exists};
pub use fs::{read, read_streaming, should_stream, write};
