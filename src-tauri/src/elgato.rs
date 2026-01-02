use crate::events::outbound::{encoder, keypad};

use std::collections::HashMap;

use base64::Engine as _;
use elgato_streamdeck::{
	AsyncStreamDeck, DeviceStateUpdate,
	images::{ImageRect, convert_image_with_format_async},
	info::Kind,
};
use image::GenericImageView as _;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

static ELGATO_DEVICES: Lazy<RwLock<HashMap<String, AsyncStreamDeck>>> = Lazy::new(|| RwLock::new(HashMap::new()));

/// Extract the average colour from an image.
fn extract_average_colour(img: &image::DynamicImage) -> (u8, u8, u8) {
	let (r_sum, g_sum, b_sum) = img
		.pixels()
		.fold((0u64, 0u64, 0u64), |(r, g, b), (_, _, pixel)| (r + pixel[0] as u64, g + pixel[1] as u64, b + pixel[2] as u64));
	let count = (img.width() * img.height()).min(1) as u64;
	((r_sum / count) as u8, (g_sum / count) as u8, (b_sum / count) as u8)
}

pub async fn update_image(context: &crate::shared::Context, image: Option<&str>) -> Result<(), anyhow::Error> {
	if let Some(device) = ELGATO_DEVICES.read().await.get(&context.device) {
		let key_count = device.kind().key_count();
		let is_touch_point = context.controller == "Keypad" && context.position >= key_count;

		if let Some(image) = image {
			let data = image.split_once(',').unwrap().1;
			let bytes = base64::engine::general_purpose::STANDARD.decode(data)?;
			if context.controller == "Encoder" {
				device
					.write_lcd(
						(context.position as u16 * 200) + 64,
						14,
						&ImageRect::from_image_async(image::load_from_memory(&bytes)?.resize(72, 72, image::imageops::FilterType::Nearest))?,
					)
					.await?;
			} else if is_touch_point {
				let (r, g, b) = extract_average_colour(&image::load_from_memory(&bytes)?);
				device.set_touchpoint_color(context.position - key_count, r, g, b).await?;
			} else {
				device.set_button_image(context.position, image::load_from_memory(&bytes)?).await?;
			}
		} else if context.controller == "Encoder" {
			device
				.write_lcd(context.position as u16 * 200, 0, &ImageRect::from_image_async(image::DynamicImage::new_rgb8(200, 100))?)
				.await?;
		} else if is_touch_point {
			device.set_touchpoint_color(context.position - key_count, 0, 0, 0).await?;
		} else {
			device.clear_button_image(context.position).await?;
		}
		device.flush().await?;
	}
	Ok(())
}

/// Clear all touchpoint LEDs on a device by setting them to black.
async fn clear_all_touchpoints(device: &AsyncStreamDeck) {
	for i in 0..device.kind().touchpoint_count() {
		let _ = device.set_touchpoint_color(i, 0, 0, 0).await;
	}
}

pub async fn clear_screen(id: &str) -> Result<(), anyhow::Error> {
	if let Some(device) = ELGATO_DEVICES.read().await.get(id) {
		device.clear_all_button_images().await?;
		if device.kind() == Kind::Plus {
			device
				.write_lcd_fill(&convert_image_with_format_async(device.kind().lcd_image_format().unwrap(), image::DynamicImage::new_rgb8(800, 100))?)
				.await?;
		}
		clear_all_touchpoints(device).await;
		device.flush().await?;
	}
	Ok(())
}

pub async fn set_brightness(brightness: u8) {
	for (_id, device) in ELGATO_DEVICES.read().await.iter() {
		let _ = device.set_brightness(brightness.clamp(0, 100)).await;
		let _ = device.flush().await;
	}
}

pub async fn reset_devices() {
	for (_id, device) in ELGATO_DEVICES.read().await.iter() {
		let _ = device.reset().await;
		let _ = device.flush().await;
	}
}

async fn init(device: AsyncStreamDeck, device_id: String) {
	if ELGATO_DEVICES.read().await.contains_key(&device_id) {
		return;
	}

	let kind = device.kind();
	let device_type = match kind {
		Kind::Original | Kind::OriginalV2 | Kind::Mk2 | Kind::Mk2Scissor | Kind::Mk2Module => 0,
		Kind::Mini | Kind::MiniMk2 | Kind::MiniDiscord | Kind::MiniMk2Module => 1,
		Kind::Xl | Kind::XlV2 | Kind::XlV2Module => 2,
		Kind::Pedal => 5,
		Kind::Plus => 7,
		Kind::Neo => 9,
	};
	let _ = device.clear_all_button_images().await;
	clear_all_touchpoints(&device).await;
	if let Ok(settings) = crate::store::get_settings() {
		let _ = device.set_brightness(settings.value.brightness).await;
	}
	let _ = device.flush().await;
	crate::events::inbound::devices::register_device(
		"",
		crate::events::inbound::PayloadEvent {
			payload: crate::shared::DeviceInfo {
				id: device_id.clone(),
				plugin: String::new(),
				name: device.product().await.unwrap(),
				rows: kind.row_count(),
				columns: kind.column_count(),
				encoders: kind.encoder_count(),
				touchpoints: kind.touchpoint_count(),
				r#type: device_type,
			},
		},
	)
	.await
	.unwrap();

	let reader = device.get_reader();
	ELGATO_DEVICES.write().await.insert(device_id.clone(), device);
	loop {
		let updates = match reader.read(100.0).await {
			Ok(updates) => updates,
			Err(_) => break,
		};
		for update in updates {
			match match update {
				DeviceStateUpdate::ButtonDown(key) => keypad::key_down(&device_id, key).await,
				DeviceStateUpdate::ButtonUp(key) => keypad::key_up(&device_id, key).await,
				DeviceStateUpdate::TouchPointDown(point) => keypad::key_down(&device_id, kind.key_count() + point).await,
				DeviceStateUpdate::TouchPointUp(point) => keypad::key_up(&device_id, kind.key_count() + point).await,
				DeviceStateUpdate::EncoderTwist(dial, ticks) => encoder::dial_rotate(&device_id, dial, ticks.into()).await,
				DeviceStateUpdate::EncoderDown(dial) => encoder::dial_press(&device_id, "dialDown", dial).await,
				DeviceStateUpdate::EncoderUp(dial) => encoder::dial_press(&device_id, "dialUp", dial).await,
				_ => Ok(()),
			} {
				Ok(_) => (),
				Err(error) => log::warn!("Failed to process device event {update:?}: {error}"),
			}
		}
	}

	ELGATO_DEVICES.write().await.remove(&device_id);
	crate::events::inbound::devices::deregister_device("", crate::events::inbound::PayloadEvent { payload: device_id })
		.await
		.unwrap();
}

/// Attempt to initialise all connected devices.
pub async fn initialise_devices() {
	if let Ok(settings) = crate::store::get_settings() {
		if settings.value.disableelgato {
			crate::plugins::DEVICE_NAMESPACES
				.write()
				.await
				.insert("sd".to_owned(), "opendeck_alternative_elgato_implementation".to_owned());
			return;
		} else {
			crate::plugins::DEVICE_NAMESPACES.write().await.remove("sd");
		}
	}

	// Iterate through detected Elgato devices and attempt to register them.
	match elgato_streamdeck::new_hidapi() {
		Ok(hid) => {
			for (kind, serial) in elgato_streamdeck::asynchronous::list_devices_async(&hid) {
				let device_id = format!("sd-{serial}");
				if ELGATO_DEVICES.read().await.contains_key(&device_id) {
					continue;
				}
				match elgato_streamdeck::AsyncStreamDeck::connect(&hid, kind, &serial) {
					Ok(device) => {
						tokio::spawn(init(device, device_id));
					}
					Err(error) => log::warn!("Failed to connect to Elgato device: {error}"),
				}
			}
		}
		Err(error) => log::warn!("Failed to initialise hidapi: {error}"),
	}
}
