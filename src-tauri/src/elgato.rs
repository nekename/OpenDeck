use crate::events::inbound;

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::LazyLock;

use base64::Engine as _;
use elgato_streamdeck::{
	AsyncStreamDeck, DeviceStateUpdate, StreamDeck,
	images::{ImageRect, convert_image_with_format_async},
	info::Kind,
};
use image::GenericImageView as _;
use tokio::sync::{RwLock, mpsc, oneshot};

static ELGATO_DEVICES: LazyLock<RwLock<HashMap<String, AsyncStreamDeck>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

/// A request to the dedicated HID thread: enumerate connected Stream Decks and
/// open any whose device ID isn't already in `known`, replying with the opened
/// (device_id, StreamDeck) pairs.
struct HidScan {
	known: HashSet<String>,
	reply: oneshot::Sender<Vec<(String, StreamDeck)>>,
}

/// Sender to the dedicated HID thread (see [hid_thread_main]).
///
/// On macOS, hidapi's `IOHIDManager` is implicitly bound to the run loop of the
/// thread that created the `HidApi`, and enumerating/opening devices from any
/// other thread schedules IOKit sources on a foreign run loop — which traps in
/// CoreFoundation (a PAC / `EXC_BREAKPOINT` crash), most reliably right after a
/// sleep/wake when the device set changes. The previous code shared one
/// `HidApi` across arbitrary tokio worker threads (elgato-streamdeck's
/// `block_in_place` does NOT pin to a single thread), so it was exposed to this.
///
/// We instead create the `HidApi` once on a single dedicated OS thread and run
/// every enumerate/open on that same thread via the synchronous API (which,
/// unlike the async wrappers, never calls `block_in_place`). Opened devices are
/// `Send`, so they're handed back and wrapped for async use afterwards.
static HID_TX: LazyLock<mpsc::UnboundedSender<HidScan>> = LazyLock::new(|| {
	let (tx, rx) = mpsc::unbounded_channel::<HidScan>();
	std::thread::Builder::new()
		.name("opendeck-hid".to_owned())
		.spawn(move || hid_thread_main(rx))
		.expect("failed to spawn HID thread");
	tx
});

/// Body of the dedicated HID thread. Owns the `HidApi` for its entire lifetime
/// and services scan requests one at a time on this thread.
fn hid_thread_main(mut rx: mpsc::UnboundedReceiver<HidScan>) {
	let mut hid: Option<hidapi::HidApi> = None;
	while let Some(scan) = rx.blocking_recv() {
		let api = match hid {
			Some(ref mut api) => {
				if let Err(error) = elgato_streamdeck::refresh_device_list(api) {
					log::warn!("Failed to refresh HID device list: {error}");
				}
				api
			}
			None => match elgato_streamdeck::new_hidapi() {
				Ok(api) => hid.insert(api),
				Err(error) => {
					log::warn!("Failed to initialise hidapi: {error}");
					let _ = scan.reply.send(Vec::new());
					continue;
				}
			},
		};

		let mut opened = Vec::new();
		for (kind, serial) in elgato_streamdeck::list_devices(api) {
			let device_id = format!("sd-{serial}");
			if scan.known.contains(&device_id) {
				continue;
			}
			match StreamDeck::connect(api, kind, &serial) {
				Ok(device) => opened.push((device_id, device)),
				Err(error) => log::warn!("Failed to connect to Elgato device: {error}"),
			}
		}
		let _ = scan.reply.send(opened);
	}
}

/// Extract the average colour from an image.
fn extract_average_colour(img: &image::DynamicImage) -> (u8, u8, u8) {
	let (r_sum, g_sum, b_sum) = img
		.pixels()
		.fold((0u64, 0u64, 0u64), |(r, g, b), (_, _, pixel)| (r + pixel[0] as u64, g + pixel[1] as u64, b + pixel[2] as u64));
	let count = (img.width() * img.height()).max(1) as u64;
	((r_sum / count) as u8, (g_sum / count) as u8, (b_sum / count) as u8)
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

	// Enumerate and open devices on the dedicated HID thread (see HID_TX) so
	// hidapi's IOHIDManager is only ever driven from its owning thread.
	let known: HashSet<String> = ELGATO_DEVICES.read().await.keys().cloned().collect();
	let (reply, rx) = oneshot::channel();
	if HID_TX.send(HidScan { known, reply }).is_err() {
		log::warn!("HID thread is gone; cannot enumerate devices");
		return;
	}
	let opened = match rx.await {
		Ok(opened) => opened,
		Err(_) => {
			log::warn!("HID thread dropped the scan request");
			return;
		}
	};
	for (device_id, device) in opened {
		// The device may have been registered since the scan; init() re-checks.
		tokio::spawn(init(AsyncStreamDeck::from(device), device_id));
	}
}
