//! Sources of "which application is focused" events.
//!
//! A source pushes the focused application's name into a channel whenever it changes (deduplicated, plus one initial value).
//! An empty string means no window is focused or it could not be determined.

mod polling;

use tokio::sync::mpsc;

pub fn start() -> mpsc::Receiver<String> {
	let (tx, rx) = mpsc::channel(16);
	tokio::spawn(polling::run(tx));
	rx
}
