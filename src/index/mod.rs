mod builder;
mod calculator;
pub mod model;
mod store;

// Size of buffer to compare files, optimized for an 8 KiB average file-size.
// Dinneen, Jesse & Nguyen, Ba. (2021). How Big Are Peoples' Computer Files? File Size Distributions
// Among User-managed Collections.
const BUF_SIZE: usize = 1024 * 8;

pub use calculator::Allowlist;
pub use calculator::Diff;
pub use store::RootIndex;
pub use store::SubIndex;
