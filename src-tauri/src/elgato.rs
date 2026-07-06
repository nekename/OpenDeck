use crate::events::inbound;
use crate::shared::{ActionInstance, Encoder, config_dir};
use crate::store::profiles::{acquire_locks_mut, get_slot_mut};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::LazyLock;
use anyhow::Context;
use base64::Engine as _;
use elgato_streamdeck::{
	AsyncStreamDeck, DeviceStateUpdate,
	images::{ImageRect, convert_image_with_format_async},
	info::Kind,
};
use image::imageops::overlay;
use image::{DynamicImage, GenericImageView as _, Rgba, RgbaImage};
use log::{trace, warn};
use serde_json::{Map, Value};
use streamdeck_strip_render::layout::{LayoutItem, PixmapSource};
use tokio::sync::RwLock;

static ELGATO_DEVICES: LazyLock<RwLock<HashMap<String, AsyncStreamDeck>>> = LazyLock::new(|| RwLock::new(HashMap::new()));
static HIDAPI: LazyLock<RwLock<Option<Arc<hidapi::HidApi>>>> = LazyLock::new(|| RwLock::new(None));

/// Extract the average colour from an image.
fn extract_average_colour(img: &image::DynamicImage) -> (u8, u8, u8) {
	let (r_sum, g_sum, b_sum) = img
		.pixels()
		.fold((0u64, 0u64, 0u64), |(r, g, b), (_, _, pixel)| (r + pixel[0] as u64, g + pixel[1] as u64, b + pixel[2] as u64));
	let count = (img.width() * img.height()).max(1) as u64;
	((r_sum / count) as u8, (g_sum / count) as u8, (b_sum / count) as u8)
}

// Honestly, this probably needs moving, not sure where though, given the scope and size of this it
// might be worth just making an new encoder.rs for it
pub async fn generate_encoder_image(context: &crate::shared::Context, fallback: &[u8]) -> Result<DynamicImage, anyhow::Error> {
	let mut locks = acquire_locks_mut().await;
	let slot = get_slot_mut(context, &mut locks).await?;

	// We need to borrow the encoder instance to render the image, so we'll take it then give it
	// back when we're done.
	let img = if let Some(instance) = slot {
		if let Some(mut encoder) = instance.action.encoder.take() {
			let result = get_encoder_image(&mut encoder, instance).context("Failed to render Encoder Image");
			instance.action.encoder = Some(encoder);
			Some(result?)
		} else {
			None
		}
	} else {
		None
	};
	drop(locks);

	match img {
		Some(img) => Ok(img),
		None => {
			// If we get here, this is either an Encoder action that doesn't have an Encoder config in the manifest, or we were
			// unable to locate the instance for this action. This realistically shouldn't happen, but if it does, we'll fall back
			// to rendering what was provided to this function call.
			trace!("No encoder instance / config found for action; using fallback image");

			let mut fallback_canvas = RgbaImage::from_pixel(200, 100, Rgba([0, 0, 0, 255]));
			let fallback_img = image::load_from_memory(fallback)
				.context("Failed to decode fallback image")?
				.resize(72, 72, image::imageops::FilterType::Nearest);

			overlay(&mut fallback_canvas, &fallback_img.to_rgba8(), 64, 14);

			Ok(DynamicImage::ImageRgba8(fallback_canvas))
		}
	}
}

fn get_encoder_image(encoder: &mut Encoder, instance: &ActionInstance) -> Result<DynamicImage, anyhow::Error> {
	// Clone the layout so we can mutate it for rendering without persisting
	let Some(ref mut renderer) = encoder.layout_parsed else {
		// Something's gone horribly wrong here; we should have a layout. Render a blank image.
		return Ok(DynamicImage::new_rgb8(200, 100));
	};

	let path = config_dir().join("plugins").join(&instance.action.plugin);
	let path_canonical = path.canonicalize()?;

	let state_idx = instance.current_state as usize;

	// Override the title if it's set in the State
	let override_title = {
		let t = instance.states[state_idx].text.trim();
		(!t.is_empty()).then(|| t.to_string())
	};

	// Similar to the title, except for the rendered icon (We check whether the state matches
	// the default, and if it doesn't, we use it)
	let state_image = &instance.states[state_idx].image.trim();

	let override_icon = {
		let default_image = &instance.action.states[state_idx].image.trim();
		(!state_image.is_empty() && state_image != default_image).then(|| state_image.to_string())
	};

	// We need to do small item corrections here
	let mut feedback = Map::new();
	let layout = renderer.layout();

	// If the layout doesn't have a title, fall back to the action title
	if let Some(title) = layout.item("title")
		&& let LayoutItem::Text(title) = title
		&& title.value.as_deref().is_none_or(str::is_empty)
	{
		feedback.insert("title".to_string(), Value::String(instance.action.name.clone()));
	}

	// If the layout doesn't have an icon, we'll use the state image.
	if let Some(icon) = layout.item("icon")
		&& let LayoutItem::Pixmap(icon) = icon
		&& icon.value == PixmapSource::None
	{
		feedback.insert("icon".to_string(), Value::String(state_image.to_string()));
	}

	// Expand + sandbox every pixmap item's relative file path.
	for item in &layout.items {
		let LayoutItem::Pixmap(p) = item else { continue };
		let PixmapSource::File(v) = &p.value else { continue };
		if v.is_empty() {
			continue;
		}

		let resolved = {
			// We need to make sure this path isn't already canonical
			let candidate = if Path::new(v).is_absolute() { PathBuf::from(v) } else { path.join(v) };
			match candidate.canonicalize() {
				Ok(resolved) if resolved.starts_with(&path_canonical) => resolved.to_string_lossy().into_owned(),
				Ok(resolved) => {
					warn!("Attempted to load image outside of base path: {resolved:?}");
					String::new()
				}
				Err(_) => {
					warn!("Unable to canonicalize path: {candidate:?}");
					String::new()
				}
			}
		};

		feedback.insert(item.key().to_string(), Value::String(resolved));
	}

	// Send changes to the renderer, note that the overrides are NOOP if they haven't changed.
	renderer.set_icon_override(override_icon);
	renderer.set_title_override(override_title);
	renderer.set_feedback(Value::Object(feedback))?;
	Ok(DynamicImage::ImageRgba8(renderer.get_image()))
}

pub async fn update_image(context: &crate::shared::Context, image: Option<&str>) -> Result<(), anyhow::Error> {
	if let Some(device) = ELGATO_DEVICES.read().await.get(&context.device) {
		let kind = device.kind();
		if !kind.is_visual() {
			return Ok(());
		}
		let key_count = kind.key_count();
		let is_touch_point = context.controller == "Keypad" && context.position >= key_count;

		if let Some(image) = image {
			let data = image.split_once(',').unwrap().1;
			let bytes = base64::engine::general_purpose::STANDARD.decode(data)?;
			if context.controller == "Encoder" {
				let img = generate_encoder_image(context, &bytes).await?;
				device.write_lcd(context.position as u16 * 200, 0, &ImageRect::from_image(img)?).await?;
			} else if context.controller == "Infobar" {
				let img = image::load_from_memory(&bytes)?;
				let Some(format) = device.kind().lcd_image_format() else {
					return Err(anyhow::anyhow!("Failed to get LCD image format"));
				};
				let data = convert_image_with_format_async(format, img.resize_exact(248, 58, image::imageops::FilterType::Lanczos3))?;
				device.write_lcd_fill(&data).await?;
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
		} else if context.controller == "Infobar" {
			let Some(format) = device.kind().lcd_image_format() else {
				return Err(anyhow::anyhow!("Failed to get LCD image format"));
			};
			let data = convert_image_with_format_async(format, image::DynamicImage::new_rgb8(248, 58))?;
			device.write_lcd_fill(&data).await?;
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
		} else if device.kind() == Kind::Neo {
			device
				.write_lcd_fill(&convert_image_with_format_async(device.kind().lcd_image_format().unwrap(), image::DynamicImage::new_rgb8(248, 58))?)
				.await?;
		}
		clear_all_touchpoints(device).await;
		device.flush().await?;
	}
	Ok(())
}

pub async fn set_brightness(id: &str, brightness: u8) {
	if let Some(device) = ELGATO_DEVICES.read().await.get(id) {
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

	let device_name = device.product().await.unwrap();
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
	let _ = device.set_brightness(crate::store::get_settings().value.brightness).await;
	let _ = device.flush().await;

	let reader = device.get_reader();
	ELGATO_DEVICES.write().await.insert(device_id.clone(), device);
	let _ = clear_screen(&device_id).await;

	crate::events::inbound::devices::register_device(
		"",
		crate::events::inbound::PayloadEvent {
			payload: crate::shared::DeviceInfo {
				id: device_id.clone(),
				plugin: String::new(),
				name: device_name,
				rows: kind.row_count(),
				columns: kind.column_count(),
				encoders: kind.encoder_count(),
				touchpoints: kind.touchpoint_count(),
				infobars: if kind == Kind::Neo { 1 } else { 0 },
				r#type: device_type,
			},
		},
	)
	.await
	.unwrap();

	let press = |position| inbound::PayloadEvent {
		payload: inbound::devices::PressPayload { device: device_id.clone(), position },
	};
	let encoder = |position, ticks: i8| inbound::PayloadEvent {
		payload: inbound::devices::TicksPayload {
			device: device_id.clone(),
			position,
			ticks: ticks.into(),
		},
	};
	let touchscreen_press = |position, x, y, hold| inbound::PayloadEvent {
		payload: inbound::devices::TouchscreenPressPayload {
			device: device_id.clone(),
			position,
			x,
			y,
			hold,
		},
	};
	loop {
		let updates = match reader.read(100.0).await {
			Ok(updates) => updates,
			Err(_) => break,
		};
		for update in updates {
			match match update {
				DeviceStateUpdate::ButtonDown(key) => inbound::devices::key_down(press(key)).await,
				DeviceStateUpdate::ButtonUp(key) => inbound::devices::key_up(press(key)).await,
				DeviceStateUpdate::TouchPointDown(point) => inbound::devices::key_down(press(kind.key_count() + point)).await,
				DeviceStateUpdate::TouchPointUp(point) => inbound::devices::key_up(press(kind.key_count() + point)).await,
				DeviceStateUpdate::EncoderTwist(dial, ticks) => inbound::devices::encoder_change(encoder(dial, ticks)).await,
				DeviceStateUpdate::EncoderDown(dial) => inbound::devices::encoder_down(press(dial)).await,
				DeviceStateUpdate::EncoderUp(dial) => inbound::devices::encoder_up(press(dial)).await,
				DeviceStateUpdate::TouchScreenPress(x, y) => {
					let (position, x, y) = match kind {
						Kind::Plus => ((x / 200) as u8, x % 200, y),
						_ => continue,
					};
					inbound::devices::touchscreen_press(touchscreen_press(position, x, y, false)).await
				}
				DeviceStateUpdate::TouchScreenLongPress(x, y) => {
					let (position, x, y) = match kind {
						Kind::Plus => ((x / 200) as u8, x % 200, y),
						_ => continue,
					};
					inbound::devices::touchscreen_press(touchscreen_press(position, x, y, true)).await
				}
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
	if crate::store::get_settings().value.disableelgato {
		crate::plugins::DEVICE_NAMESPACES
			.write()
			.await
			.insert("sd".to_owned(), "opendeck_alternative_elgato_implementation".to_owned());
		return;
	} else {
		crate::plugins::DEVICE_NAMESPACES.write().await.remove("sd");
	}

	// Iterate through detected Elgato devices and attempt to register them.
	let current = HIDAPI.read().await.as_ref().cloned();
	let hid = match current {
		Some(arc) => arc,
		None => match elgato_streamdeck::new_hidapi() {
			Ok(hid) => {
				let arc = Arc::new(hid);
				HIDAPI.write().await.replace(arc.clone());
				arc
			}
			Err(error) => {
				log::warn!("Failed to initialise hidapi: {error}");
				return;
			}
		},
	};
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
