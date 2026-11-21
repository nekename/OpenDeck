#[cfg(not(target_os = "freebsd"))]
use psp::monitor::{PowerMonitor, PowerState};

/// FreeBSD has no systemd-logind, which `psp` requires. Resume events come from devd(8) ACPI notifications, the same
/// ones that run `/etc/rc.resume`. There is no compositor-independent screen lock notification, so "sleep when
/// computer locked" is unavailable; a screen locker can run `opendeck --sleep-device <id>` and `--wake-device <id>`.
#[cfg(target_os = "freebsd")]
pub fn init_power_events() {
	use std::io::{BufRead, BufReader};
	use std::os::unix::net::UnixStream;

	const DEVD_PIPE: &str = "/var/run/devd.pipe";

	log::warn!("Screen lock events are unavailable on FreeBSD; use --sleep-device and --wake-device from your screen locker instead");

	std::thread::spawn(|| {
		let mut reported_failure = false;
		loop {
			match UnixStream::connect(DEVD_PIPE) {
				Ok(stream) => {
					reported_failure = false;
					for line in BufReader::new(stream).lines() {
						let Ok(line) = line else { break };
						// Notify events look like: !system=ACPI subsystem=Resume type=1
						if line.starts_with('!') && line.contains("system=ACPI") && line.contains("subsystem=Resume") {
							tauri::async_runtime::spawn(async {
								crate::elgato::wake_deeply_sleeping_devices().await;
							});
							tauri::async_runtime::spawn(async {
								if let Err(error) = crate::events::outbound::misc::system_did_wake_up().await {
									log::error!("Failed to send the systemDidWakeUp event: {error}");
								}
							});
						}
					}
					log::warn!("Lost connection to devd at {DEVD_PIPE}; reconnecting");
				}
				Err(error) => {
					if !reported_failure {
						log::error!("Failed to connect to devd at {DEVD_PIPE}, resume events will not be handled: {error}");
						reported_failure = true;
					}
				}
			}
			std::thread::sleep(std::time::Duration::from_secs(5));
		}
	});
}

#[cfg(not(target_os = "freebsd"))]
pub fn init_power_events() {
	let power_monitor = Box::leak(Box::new(PowerMonitor::new()));
	let receiver = power_monitor.event_receiver();
	if let Err(error) = power_monitor.start_listening() {
		log::error!("Failed to start listening for power events: {error}");
		return;
	}

	std::thread::spawn(move || {
		while let Ok(event) = receiver.recv() {
			match event {
				PowerState::ScreenLocked => {
					tauri::async_runtime::spawn(async {
						if let Err(error) = crate::device_sleep::sleep_for_computer_lock().await {
							log::error!("Failed to sleep devices due to screen lock: {error}");
						}
					});
				}
				PowerState::ScreenUnlocked => {
					tauri::async_runtime::spawn(async {
						if let Err(error) = crate::device_sleep::wake_from_computer_lock().await {
							log::error!("Failed to wake devices due to screen unlock: {error}");
						}
					});
				}
				PowerState::Resume => {
					tauri::async_runtime::spawn(async {
						crate::elgato::wake_deeply_sleeping_devices().await;
					});
					tauri::async_runtime::spawn(async {
						if let Err(error) = crate::events::outbound::misc::system_did_wake_up().await {
							log::error!("Failed to send the systemDidWakeUp event: {error}");
						}
					});
				}
				PowerState::Suspend | PowerState::Shutdown | PowerState::Unknown => {}
			}
		}
	});
}
