//! Stand-in for the subset of `tauri-plugin-aptabase` used by OpenDeck, which does not compile on FreeBSD
//! (it has no `ENGINE_NAME` for the platform). Usage statistics are not collected on FreeBSD.

use std::time::Duration;

use serde_json::Value;
use tauri::plugin::TauriPlugin;
use tauri::{App, AppHandle, Runtime};

#[derive(Default)]
#[expect(dead_code, reason = "mirrors tauri-plugin-aptabase's API; the options are not used")]
pub struct InitOptions {
	pub host: Option<String>,
	pub flush_interval: Option<Duration>,
}

pub struct Builder;

impl Builder {
	pub fn new(_app_key: &str) -> Self {
		Self
	}

	pub fn with_options(self, _options: InitOptions) -> Self {
		self
	}

	pub fn build<R: Runtime>(self) -> TauriPlugin<R> {
		log::info!("Usage statistics are unavailable on FreeBSD (tauri-plugin-aptabase does not support it)");
		tauri::plugin::Builder::new("aptabase").build()
	}
}

pub trait EventTracker {
	fn track_event(&self, name: &str, props: Option<Value>) -> Result<(), String>;
	fn flush_events_blocking(&self);
}

impl EventTracker for App {
	fn track_event(&self, _name: &str, _props: Option<Value>) -> Result<(), String> {
		Ok(())
	}
	fn flush_events_blocking(&self) {}
}

impl EventTracker for AppHandle {
	fn track_event(&self, _name: &str, _props: Option<Value>) -> Result<(), String> {
		Ok(())
	}
	fn flush_events_blocking(&self) {}
}
