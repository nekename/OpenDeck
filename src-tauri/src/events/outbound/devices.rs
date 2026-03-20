use super::{send_to_all_plugins, send_to_plugin};

use crate::plugins::{DEVICE_NAMESPACES, info_param::DeviceInfo};

use serde::Serialize;

#[derive(Serialize)]
#[allow(non_snake_case)]
struct DeviceDidConnectEvent {
	event: &'static str,
	device: String,
	deviceInfo: DeviceInfo,
}

pub async fn device_did_connect(id: &str, info: DeviceInfo) -> Result<(), anyhow::Error> {
	send_to_all_plugins(&DeviceDidConnectEvent {
		event: "deviceDidConnect",
		device: id.to_owned(),
		deviceInfo: info,
	})
	.await
}

#[derive(Serialize)]
struct DeviceDidDisconnectEvent {
	event: &'static str,
	device: String,
}

pub async fn device_did_disconnect(id: &str) -> Result<(), anyhow::Error> {
	send_to_all_plugins(&DeviceDidDisconnectEvent {
		event: "deviceDidDisconnect",
		device: id.to_owned(),
	})
	.await
}

#[derive(Serialize)]
struct SetImageEvent {
	event: &'static str,
	device: String,
	controller: Option<String>,
	position: Option<u8>,
	image: Option<String>,
}

pub async fn update_image(context: crate::shared::Context, image: Option<String>) -> Result<(), anyhow::Error> {
	if let Some(plugin) = DEVICE_NAMESPACES.read().await.get(&context.device[..2]) {
		send_to_plugin(
			plugin,
			&SetImageEvent {
				event: "setImage",
				device: context.device,
				controller: Some(context.controller),
				position: Some(context.position),
				image,
			},
		)
		.await?;
	} else if context.device.starts_with("sd-") {
		crate::elgato::update_image(&context, image.as_deref()).await?;
	}

	Ok(())
}

pub async fn clear_screen(device: String) -> Result<(), anyhow::Error> {
	if let Some(plugin) = DEVICE_NAMESPACES.read().await.get(&device[..2]) {
		send_to_plugin(
			plugin,
			&SetImageEvent {
				event: "setImage",
				device,
				controller: None,
				position: None,
				image: None,
			},
		)
		.await?;
	} else if device.starts_with("sd-") {
		crate::elgato::clear_screen(&device).await?;
	}

	Ok(())
}

#[derive(Serialize)]
struct SetBrightnessEvent {
	event: &'static str,
	device: String,
	brightness: u8,
}

/// Set brightness for all devices, used by frontend/settings.rs to set global brightness
pub async fn set_brightness(brightness: u8) -> Result<(), anyhow::Error> {
	for device in crate::shared::DEVICES.iter() {
		set_device_brightness(&device.id, brightness).await?;
	}

	Ok(())
}

/// Set brightness for a specific device. Used by the global setter and device wake and sleep functions
pub async fn set_device_brightness(device: &str, brightness: u8) -> Result<(), anyhow::Error> {
	if let Some(plugin) = DEVICE_NAMESPACES.read().await.get(&device[..2]) {
		send_to_plugin(
			plugin,
			&SetBrightnessEvent {
				event: "setBrightness",
				device: device.to_owned(),
				brightness,
			},
		)
		.await?;
	} else {
		crate::elgato::set_brightness(device, brightness).await;
	}

	Ok(())
}
