//! Sources of "which application is focused" events.
//!
//! A source pushes the focused application's name into a channel whenever it changes (deduplicated, plus one initial value).
//! An empty string means no window is focused or it could not be determined.
//!
//! Event-driven sources are preferred as they cost nothing while idle. Polling is the fallback, and is also used when an
//! event-driven source fails to start or dies.

#[cfg(target_os = "linux")]
mod kwin;
mod polling;
#[cfg(target_os = "linux")]
mod x11;

use tokio::sync::mpsc;

pub fn start() -> mpsc::Receiver<String> {
	let (tx, rx) = mpsc::channel(16);
	tokio::spawn(async move {
		#[cfg(target_os = "linux")]
		{
			if kwin::available() {
				match kwin::run(tx.clone()).await {
					Ok(()) => return,
					Err(error) => log::warn!("KWin active window source failed, falling back: {error:#}"),
				}
			}
			if x11::available() {
				match x11::run(tx.clone()).await {
					Ok(()) => return,
					Err(error) => log::warn!("X11 active window source failed, falling back: {error:#}"),
				}
			}
		}
		polling::run(tx).await;
	});
	rx
}
