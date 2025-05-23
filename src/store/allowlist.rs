use regex::Regex;

pub struct Allowlist {
	pub allow: Vec<Regex>,
	pub deny: Vec<Regex>,
}

impl Allowlist {
	pub fn is_allowed(&self, value: &str) -> bool {
		for regex in &self.deny {
			if regex.is_match(value) {
				return false;
			}
		}
		if self.allow.is_empty() {
			return true;
		}
		for regex in &self.allow {
			if regex.is_match(value) {
				return true;
			}
		}
		false
	}
}
