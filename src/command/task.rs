use std::sync::atomic;
use std::thread;
use std::time::SystemTime;

use crate::progress;

pub struct Delayer {
	start: SystemTime,
}

impl Delayer {
	pub fn new() -> Self {
		Self {
			start: SystemTime::now(),
		}
	}

	pub fn run(&mut self) -> bool {
		let elapsed = self.start.elapsed().unwrap();
		if elapsed.as_secs() >= 1 {
			self.start = SystemTime::now();
			return true;
		}
		false
	}
}

// Runs the given condition method with a delay.
pub fn condition_delay(condition_fn: impl Fn() -> bool) -> bool {
	let now = SystemTime::now();
	loop {
		if condition_fn() {
			return true;
		}
		if now.elapsed().unwrap().as_secs() >= 1 {
			break;
		}
		thread::yield_now();
	}
	false
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
