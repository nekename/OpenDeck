use std::sync::LazyLock;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;

static LAST_ACTIVITY: LazyLock<DashMap<String, Instant>> = LazyLock::new(DashMap::new);
static SLEEPING_DEVICES: LazyLock<DashMap<String, ()>> = LazyLock::new(DashMap::new);
static SLEEP_TIMEOUT_MINUTES: AtomicU16 = AtomicU16::new(0);
static INITIALISED: OnceLock<()> = OnceLock::new();

pub fn init() {
	if INITIALISED.set(()).is_err() {
		return;
	}

	if let Ok(settings) = crate::store::get_settings() {
		SLEEP_TIMEOUT_MINUTES.store(settings.value.sleep_timeout_minutes, Ordering::Relaxed);
	}

	tokio::spawn(async {
		loop {
			if let Err(error) = sleep_idle_devices().await {
				log::warn!("Failed to update sleeping devices: {error}");
			}
			tokio::time::sleep(Duration::from_secs(1)).await;
		}
	});
}

pub async fn register_device(device: &str) -> Result<(), anyhow::Error> {
	LAST_ACTIVITY.insert(device.to_owned(), Instant::now());
	wake_device(device).await
}

pub fn deregister_device(device: &str) {
	LAST_ACTIVITY.remove(device);
	SLEEPING_DEVICES.remove(device);
}

pub async fn note_activity(device: &str) -> Result<(), anyhow::Error> {
	LAST_ACTIVITY.insert(device.to_owned(), Instant::now());
	wake_device(device).await
}

pub fn update_settings(settings: &crate::store::Settings) {
	SLEEP_TIMEOUT_MINUTES.store(settings.sleep_timeout_minutes, Ordering::Relaxed);
	if settings.sleep_timeout_minutes == 0 {
		SLEEPING_DEVICES.clear();
	}
}

async fn sleep_idle_devices() -> Result<(), anyhow::Error> {
	let timeout = SLEEP_TIMEOUT_MINUTES.load(Ordering::Relaxed);
	if timeout == 0 {
		return Ok(());
	}

	let idle_after = Duration::from_secs(timeout as u64 * 60);
	let now = Instant::now();
	let device_ids = LAST_ACTIVITY.iter().map(|entry| entry.key().clone()).collect::<Vec<_>>();
	for device in device_ids {
		let Some(last_activity) = LAST_ACTIVITY.get(&device).map(|entry| *entry.value()) else { continue };
		if now.duration_since(last_activity) < idle_after || SLEEPING_DEVICES.contains_key(&device) {
			continue;
		}

		crate::events::outbound::devices::set_device_brightness(&device, 0).await?;
		SLEEPING_DEVICES.insert(device, ());
	}

	Ok(())
}

async fn wake_device(device: &str) -> Result<(), anyhow::Error> {
	if SLEEPING_DEVICES.remove(device).is_some() {
		let brightness = crate::store::get_settings().map(|s| s.value.brightness).unwrap_or(50);
		crate::events::outbound::devices::set_device_brightness(device, brightness).await?;
	}
	Ok(())
}
