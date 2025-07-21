pub trait Index {
	/// Returns the total number of filesystem entries.
	fn entry_count(&self) -> usize;

	/// Returns the total number of files.
	fn file_count(&self) -> usize;

	/// Returns the total number of directories.
	fn dir_count(&self) -> usize;
}
