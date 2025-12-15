use super::send_to_plugin;

use serde::Serialize;

#[derive(Serialize)]
struct DidReceiveDeepLinkPayload {
	url: String,
}

#[derive(Serialize)]
struct DidReceiveDeepLinkEvent {
	event: &'static str,
	payload: DidReceiveDeepLinkPayload,
}

pub async fn did_receive_deep_link(plugin: &str, url: String) -> Result<(), anyhow::Error> {
	send_to_plugin(
		plugin,
		&DidReceiveDeepLinkEvent {
			event: "didReceiveDeepLink",
			payload: DidReceiveDeepLinkPayload { url },
		},
	)
	.await
}
