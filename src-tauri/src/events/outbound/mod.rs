pub mod applications;
pub mod deep_link;
pub mod devices;
pub mod encoder;
pub mod keypad;
pub mod misc;
pub mod property_inspector;
pub mod settings;
pub mod states;
pub mod will_appear;

use serde::Serialize;

#[derive(Serialize)]
struct Coordinates {
	row: u8,
	column: u8,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct GenericInstancePayload {
	settings: serde_json::Value,
	coordinates: Coordinates,
	controller: String,
	state: u16,
	isInMultiAction: bool,
}

impl GenericInstancePayload {
	fn new(instance: &crate::shared::ActionInstance) -> Self {
		let coordinates = match &instance.context.controller[..] {
			"Encoder" => Coordinates {
				row: 0,
				column: instance.context.position,
			},
			_ => {
				let columns = crate::shared::DEVICES.get(&instance.context.device).unwrap().columns;
				Coordinates {
					row: instance.context.position / columns,
					column: instance.context.position % columns,
				}
			}
		};

		Self {
			settings: instance.settings.clone(),
			coordinates,
			controller: instance.context.controller.clone(),
			state: instance.current_state,
			isInMultiAction: instance.context.index != 0,
		}
	}
}

async fn send_to_plugin(plugin: &str, data: &impl Serialize) -> Result<(), anyhow::Error> {
	let message = tokio_tungstenite::tungstenite::Message::Text(serde_json::to_string(data)?.into());
	let sockets = super::PLUGIN_SOCKETS.lock().await;

	if let Some(socket) = sockets.get(plugin) {
		socket.send(message)?;
	} else {
		let mut queues = super::PLUGIN_QUEUES.write().await;
		if queues.contains_key(plugin) {
			queues.get_mut(plugin).unwrap().push(message);
		} else {
			queues.insert(plugin.to_owned(), vec![message]);
		}
	}

	Ok(())
}

async fn send_to_all_plugins(data: &impl Serialize) -> Result<(), anyhow::Error> {
	let mut entries = tokio::fs::read_dir(crate::shared::config_dir().join("plugins")).await?;
	while let Ok(Some(entry)) = entries.next_entry().await {
		let path = match entry.metadata().await?.is_symlink() {
			true => tokio::fs::read_link(entry.path()).await?,
			false => entry.path(),
		};
		let metadata = tokio::fs::metadata(&path).await?;
		if metadata.is_dir() {
			let _ = send_to_plugin(entry.file_name().to_str().unwrap(), data).await;
		}
	}
	Ok(())
}

#[allow(clippy::map_entry)]
async fn send_to_property_inspector(context: &crate::shared::ActionContext, data: &impl Serialize) -> Result<(), anyhow::Error> {
	let message = tokio_tungstenite::tungstenite::Message::Text(serde_json::to_string(data)?.into());
	let sockets = super::PROPERTY_INSPECTOR_SOCKETS.lock().await;

	if let Some(socket) = sockets.get(&context.to_string()) {
		socket.send(message)?;
	} else {
		let mut queues = super::PROPERTY_INSPECTOR_QUEUES.write().await;
		if queues.contains_key(&context.to_string()) {
			queues.get_mut(&context.to_string()).unwrap().push(message);
		} else {
			queues.insert(context.to_string(), vec![message]);
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::send_to_plugin;
	use crate::events::{inbound::RegisterEvent, register_plugin};

	use std::time::Duration;

	use tokio::net::{TcpListener, TcpStream};
	use tokio_tungstenite::WebSocketStream;

	/// Connect a WebSocket client and register the server side of it as the plugin `uuid`.
	async fn connect_plugin(uuid: &str) -> WebSocketStream<TcpStream> {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let address = listener.local_addr().unwrap();
		let client = async { tokio_tungstenite::client_async(format!("ws://{address}/"), TcpStream::connect(address).await.unwrap()).await.unwrap().0 };
		let server = async { tokio_tungstenite::accept_async(listener.accept().await.unwrap().0).await.unwrap() };
		let (client, server) = tokio::join!(client, server);
		register_plugin(RegisterEvent::RegisterPlugin { uuid: uuid.to_owned() }, server).await;
		client
	}

	#[tokio::test]
	async fn plugin_that_stops_reading_does_not_block_senders() {
		// The client is never read from, so the server's TCP send buffer fills up after a few megabytes.
		let _client = connect_plugin("test.stalled").await;
		let payload = serde_json::json!({ "event": "test", "data": "x".repeat(256 * 1024) });

		let send_all = async {
			for _ in 0..256 {
				send_to_plugin("test.stalled", &payload).await.unwrap();
			}
		};
		tokio::time::timeout(Duration::from_secs(10), send_all).await.expect("sending to a plugin that stopped reading blocked");
	}

	/// Read the next text message, or `None` if nothing arrives within the timeout.
	async fn next_text(client: &mut WebSocketStream<TcpStream>) -> Option<String> {
		use futures::StreamExt;
		match tokio::time::timeout(Duration::from_millis(500), client.next()).await {
			Ok(Some(Ok(message))) => message.into_text().ok().map(|text| text.to_string()),
			_ => None,
		}
	}

	#[tokio::test]
	async fn queued_events_are_delivered_only_once() {
		let first = serde_json::json!({ "event": "first" });
		send_to_plugin("test.queue", &first).await.unwrap();
		let mut client = connect_plugin("test.queue").await;
		assert_eq!(next_text(&mut client).await, Some(first.to_string()));

		drop(client);
		let disconnected = async {
			while crate::events::registered_plugins().await.iter().any(|uuid| uuid == "test.queue") {
				tokio::time::sleep(Duration::from_millis(10)).await;
			}
		};
		tokio::time::timeout(Duration::from_secs(5), disconnected).await.unwrap();

		let second = serde_json::json!({ "event": "second" });
		send_to_plugin("test.queue", &second).await.unwrap();
		let mut client = connect_plugin("test.queue").await;
		assert_eq!(next_text(&mut client).await, Some(second.to_string()));
		assert_eq!(next_text(&mut client).await, None);
	}
}
