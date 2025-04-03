mod calculator;
mod checksum;
mod entry;
mod metadata;
mod root_index;
mod sub_index;
#[cfg(test)]
mod sub_index_test;
#[cfg(test)]
mod test;

pub use calculator::Diff;
pub use root_index::RootIndex;

// Size of buffer to compare files, optimized for an 8 KiB average file-size.
// Dinneen, Jesse & Nguyen, Ba. (2021). How Big Are Peoples' Computer Files? File Size Distributions
// Among User-managed Collections.
const BUF_SIZE: usize = 1024 * 8;
