use std::sync::LazyLock;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, AtomicU16, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;

static LAST_ACTIVITY: LazyLock<DashMap<String, Instant>> = LazyLock::new(DashMap::new);
static ACTIVE_INPUTS: LazyLock<DashMap<String, u32>> = LazyLock::new(DashMap::new);
static SLEEPING_DEVICES: LazyLock<DashMap<String, ()>> = LazyLock::new(DashMap::new);
static SLEEP_TIMEOUT_MINUTES: AtomicU16 = AtomicU16::new(0);
static AWAKE_BRIGHTNESS: AtomicU8 = AtomicU8::new(50);
static INITIALISED: OnceLock<()> = OnceLock::new();

pub fn init() {
	if INITIALISED.set(()).is_err() {
		return;
	}

	if let Ok(settings) = crate::store::get_settings() {
		SLEEP_TIMEOUT_MINUTES.store(settings.value.sleep_timeout_minutes, Ordering::Relaxed);
		AWAKE_BRIGHTNESS.store(settings.value.brightness, Ordering::Relaxed);
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
	ACTIVE_INPUTS.remove(device);
	SLEEPING_DEVICES.remove(device);
}

pub async fn note_activity(device: &str) -> Result<(), anyhow::Error> {
	LAST_ACTIVITY.insert(device.to_owned(), Instant::now());
	wake_device(device).await
}

pub async fn input_started(device: &str) -> Result<(), anyhow::Error> {
	let mut count = ACTIVE_INPUTS.entry(device.to_owned()).or_insert(0);
	*count += 1;
	drop(count);
	note_activity(device).await
}

pub async fn input_ended(device: &str) -> Result<(), anyhow::Error> {
	if let Some(mut count) = ACTIVE_INPUTS.get_mut(device) {
		*count = count.saturating_sub(1);
	}
	note_activity(device).await
}

pub fn update_settings(settings: &crate::store::Settings) {
	SLEEP_TIMEOUT_MINUTES.store(settings.sleep_timeout_minutes, Ordering::Relaxed);
	AWAKE_BRIGHTNESS.store(settings.brightness, Ordering::Relaxed);
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
		if now.duration_since(last_activity) < idle_after || SLEEPING_DEVICES.contains_key(&device) || ACTIVE_INPUTS.get(&device).is_some_and(|count| *count > 0) {
			continue;
		}

		crate::events::outbound::devices::set_device_brightness(&device, 0).await?;
		SLEEPING_DEVICES.insert(device, ());
	}

	Ok(())
}

async fn wake_device(device: &str) -> Result<(), anyhow::Error> {
	if SLEEPING_DEVICES.remove(device).is_some() {
		crate::events::outbound::devices::set_device_brightness(device, AWAKE_BRIGHTNESS.load(Ordering::Relaxed)).await?;
	}
	Ok(())
}
