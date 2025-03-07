use std::sync::atomic;
use std::thread;
use std::time::SystemTime;

use crate::progress;

/// Runs the given `run_fn` every second, as long as `is_done_fn` is false. It is possible that
/// `run_fn` never runs if `is_done_fn` is true.
pub fn interval(is_done_fn: impl Fn() -> bool, run_fn: impl Fn()) {
	loop {
		let now = SystemTime::now();
		loop {
			if is_done_fn() {
				return;
			}
			// thread::sleep(Duration::from_secs(1));
			if now.elapsed().unwrap().as_secs() >= 1 {
				break;
			}
			thread::yield_now();
		}
		run_fn();
	}
}

pub struct Task {
	pub counter: progress::AtomicProgressCounter,
	done: atomic::AtomicBool,
}

impl Task {
	pub fn new() -> Self {
		Self {
			counter: progress::AtomicProgressCounter::new(),
			done: atomic::AtomicBool::new(false),
		}
	}

	pub fn done(&self) -> bool {
		self.done.load(atomic::Ordering::Relaxed)
	}

	pub fn set_done(&self) {
		self.done.store(true, atomic::Ordering::SeqCst);
	}
}
