pub enum MatchKind {
	/// The size and the name match. Metadata or checksums may not match.
	Size,
	/// The metadata of the file match. Nothing else was compared.
	Metadata,
	/// The metadata and available checksums match. Content was not compared.
	Checksums,
}
