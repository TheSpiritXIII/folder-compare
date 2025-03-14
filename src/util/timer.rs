use std::time::Duration;
use std::time::SystemTime;

pub struct CountdownTimer {
	start: SystemTime,
	duration: Duration,
}

impl CountdownTimer {
	pub fn new(duration: Duration) -> Self {
		Self {
			start: SystemTime::now(),
			duration,
		}
	}

	pub fn passed(&mut self) -> bool {
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
