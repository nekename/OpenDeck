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

use futures::SinkExt;
use serde::Serialize;

#[derive(Serialize)]
struct Coordinates {
	row: u8,
	column: u8,
}

impl Coordinates {
	fn from_context(context: &crate::shared::ActionContext) -> Self {
		if context.controller == "Encoder" {
			return Coordinates { row: 0, column: context.position };
		}
		match crate::shared::DEVICES.get(&context.device) {
			Some(device) if device.columns > 0 => Coordinates {
				row: context.position / device.columns,
				column: context.position % device.columns,
			},
			_ => Coordinates { row: 0, column: 0 },
		}
	}
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
		Self {
			settings: instance.settings.clone(),
			coordinates: Coordinates::from_context(&instance.context),
			controller: instance.context.controller.clone(),
			state: instance.current_state,
			isInMultiAction: instance.context.index != 0,
		}
	}

	fn empty(context: &crate::shared::ActionContext) -> Self {
		Self {
			settings: serde_json::Value::Object(serde_json::Map::new()),
			coordinates: Coordinates::from_context(context),
			controller: context.controller.clone(),
			state: 0,
			isInMultiAction: context.index != 0,
		}
	}
}

async fn send_to_plugin(plugin: &str, data: &impl Serialize) -> Result<(), anyhow::Error> {
	let message = tokio_tungstenite::tungstenite::Message::Text(serde_json::to_string(data)?.into());
	let mut sockets = super::PLUGIN_SOCKETS.lock().await;

	if let Some(socket) = sockets.get_mut(plugin) {
		socket.send(message).await?;
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
	let mut sockets = super::PROPERTY_INSPECTOR_SOCKETS.lock().await;

	if let Some(socket) = sockets.get_mut(&context.to_string()) {
		socket.send(message).await?;
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
