use std::sync::atomic;

pub trait ProgressCounter {
	fn update(&self, count: usize);
}

pub struct AtomicProgressCounter {
	counter: atomic::AtomicUsize,
}

impl AtomicProgressCounter {
	pub fn new() -> Self {
		Self {
			counter: atomic::AtomicUsize::new(0),
		}
	}

	pub fn value(&self) -> usize {
		self.counter.load(atomic::Ordering::Relaxed)
	}
}

impl ProgressCounter for AtomicProgressCounter {
	fn update(&self, count: usize) {
		self.counter.store(count, atomic::Ordering::Release);
	}
}
