pub fn percentage(current: usize, total: usize) -> String {
	// TODO: Fix this.
	#[allow(clippy::cast_precision_loss)]
	let percent = current as f64 / total as f64 * 100_f64;
	format!("{percent:04.1}%")
}
