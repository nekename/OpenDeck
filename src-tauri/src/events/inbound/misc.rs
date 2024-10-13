use super::{ContextAndPayloadEvent, ContextEvent, PayloadEvent};

use tauri::{Emitter, Manager};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenUrlEvent {
	pub url: String,
}

#[derive(Deserialize)]
pub struct LogMessageEvent {
	pub message: String,
}

pub async fn open_url(event: PayloadEvent<OpenUrlEvent>) -> Result<(), anyhow::Error> {
	log::debug!("Opening URL {}", event.payload.url);
	open::that_detached(event.payload.url)?;
	Ok(())
}

pub async fn log_message(event: PayloadEvent<LogMessageEvent>) -> Result<(), anyhow::Error> {
	log::info!("{}", event.payload.message.trim());
	Ok(())
}

pub async fn send_to_property_inspector(event: ContextAndPayloadEvent<serde_json::Value>) -> Result<(), anyhow::Error> {
	crate::events::outbound::property_inspector::send_to_property_inspector(event.context, event.payload).await?;
	Ok(())
}

pub async fn send_to_plugin(event: ContextAndPayloadEvent<serde_json::Value>) -> Result<(), anyhow::Error> {
	crate::events::outbound::property_inspector::send_to_plugin(event.context, event.payload).await?;
	Ok(())
}

pub async fn show_alert(event: ContextEvent) -> Result<(), anyhow::Error> {
	let app = crate::APP_HANDLE.get().unwrap();
	app.get_webview_window("main").unwrap().emit("show_alert", event.context)?;
	Ok(())
}

pub async fn show_ok(event: ContextEvent) -> Result<(), anyhow::Error> {
	let app = crate::APP_HANDLE.get().unwrap();
	app.get_webview_window("main").unwrap().emit("show_ok", event.context)?;
	Ok(())
}

#[derive(Clone, serde::Serialize, Deserialize)]
pub struct SwitchProfileEvent {
	device: String,
	profile: String,
}

pub async fn switch_profile(event: SwitchProfileEvent) -> Result<(), anyhow::Error> {
	let app_handle = crate::APP_HANDLE.get().unwrap();
	app_handle.get_webview_window("main").unwrap().emit("switch_profile", event)?;
	Ok(())
}
