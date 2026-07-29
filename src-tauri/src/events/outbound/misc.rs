use super::send_to_all_plugins;

use serde::Serialize;

#[derive(Serialize)]
struct SystemDidWakeUpEvent {
	event: &'static str,
}

pub async fn system_did_wake_up() -> Result<(), anyhow::Error> {
	send_to_all_plugins(&SystemDidWakeUpEvent { event: "systemDidWakeUp" }).await
}
