use super::send_to_plugin;

#[derive(serde::Serialize)]
struct ApplicationDidLaunchEvent {
	event: &'static str,
	payload: ApplicationPayload,
}

#[derive(serde::Serialize)]
struct ApplicationDidTerminate {
	event: &'static str,
	payload: ApplicationPayload,
}

#[derive(serde::Serialize)]
struct ApplicationPayload {
	application: String,
}

pub async fn application_did_launch(plugin: &str, application: String) -> Result<(), anyhow::Error> {
	send_to_plugin(
		plugin,
		&ApplicationDidLaunchEvent {
			event: "applicationDidLaunch",
			payload: ApplicationPayload { application },
		},
	)
	.await
}

pub async fn application_did_terminate(plugin: &str, application: String) -> Result<(), anyhow::Error> {
	send_to_plugin(
		plugin,
		&ApplicationDidTerminate {
			event: "applicationDidTerminate",
			payload: ApplicationPayload { application },
		},
	)
	.await
}
