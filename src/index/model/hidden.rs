use std::fs::Metadata;
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;

#[cfg(target_os = "windows")]
pub fn is_hidden_windows_metadata(metadata: &Metadata) -> bool {
	// https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants
	const FILE_ATTRIBUTE_HIDDEN: u32 = 2;
	metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
}

#[cfg(not(target_os = "windows"))]
pub fn is_hidden_windows_metadata(metadata: &Metadata) -> bool {
	false
}
