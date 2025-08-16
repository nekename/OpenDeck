use super::send_to_plugin;

use serde::Serialize;

#[derive(Serialize)]
struct ApplicationPayload {
	application: String,
}

#[derive(Serialize)]
struct ApplicationEvent {
	event: &'static str,
	payload: ApplicationPayload,
}

pub async fn application_did_launch(plugin: &str, application: String) -> Result<(), anyhow::Error> {
	send_to_plugin(
		plugin,
		&ApplicationEvent {
			event: "applicationDidLaunch",
			payload: ApplicationPayload { application },
		},
	)
	.await
}

pub async fn application_did_terminate(plugin: &str, application: String) -> Result<(), anyhow::Error> {
	send_to_plugin(
		plugin,
		&ApplicationEvent {
			event: "applicationDidTerminate",
			payload: ApplicationPayload { application },
		},
	)
	.await
}
