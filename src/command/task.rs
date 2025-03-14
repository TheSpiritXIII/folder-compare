use std::sync::atomic;
use std::time::Duration;
use std::time::SystemTime;

use crate::progress;

pub struct Delayer {
	start: SystemTime,
	duration: Duration,
}

impl Delayer {
	pub fn new(duration: Duration) -> Self {
		Self {
			start: SystemTime::now(),
			duration,
		}
	}

	pub fn run(&mut self) -> bool {
		let Ok(elapsed) = self.start.elapsed() else {
			self.start = SystemTime::now();
			return true;
		};
		if elapsed >= self.duration {
			self.start = SystemTime::now();
			return true;
		}
		false
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
