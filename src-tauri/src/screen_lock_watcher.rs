//! Screen lock watcher module for detecting when the screen is locked/unlocked.
//!
//! When the screen is locked, the Stream Deck should go into standby mode
//! (buttons disabled, screen blanked) to prevent unauthorized access.

use std::sync::atomic::{AtomicBool, Ordering};

use once_cell::sync::Lazy;

/// Global state tracking whether the screen is currently locked.
pub static SCREEN_LOCKED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

/// Check if the screen is currently locked.
pub fn is_screen_locked() -> bool {
	SCREEN_LOCKED.load(Ordering::Relaxed)
}

/// Initialize the screen lock watcher.
/// This spawns a background task that monitors for screen lock/unlock events.
pub fn init_screen_lock_watcher() {
	tokio::spawn(async move {
		platform::watch_screen_lock().await;
	});
}

#[cfg(target_os = "linux")]
mod platform {
	use super::*;

	use std::time::Duration;

	use dbus::blocking::Connection;

	/// Watch for screen lock/unlock events on Linux using D-Bus.
	/// Monitors both org.freedesktop.login1 (for session lock) and
	/// org.freedesktop.ScreenSaver / org.gnome.ScreenSaver for screensaver.
	pub async fn watch_screen_lock() {
		// Run the D-Bus monitoring in a blocking task since dbus crate is synchronous
		let handle = tokio::task::spawn_blocking(|| {
			if let Err(e) = run_dbus_monitor() {
				log::warn!("D-Bus screen lock monitor failed: {e}, falling back to polling");
				// Fall back to polling if signal monitoring fails
				poll_screen_lock_status();
			}
		});

		if let Err(e) = handle.await {
			log::error!("Screen lock watcher task failed: {e}");
		}
	}

	/// Run the D-Bus signal monitor for screen lock events.
	fn run_dbus_monitor() -> Result<(), dbus::Error> {
		let conn = Connection::new_system()?;

		// Get the session path for the current user
		let session_path = get_session_path(&conn)?;

		// Add match rules for Lock and Unlock signals from login1
		conn.add_match(dbus::message::MatchRule::new_signal("org.freedesktop.login1.Session", "Lock"), |_: (), _, _| true)?;
		conn.add_match(dbus::message::MatchRule::new_signal("org.freedesktop.login1.Session", "Unlock"), |_: (), _, _| true)?;

		log::info!("Screen lock watcher initialized (D-Bus signals)");

		loop {
			// Process D-Bus messages with a timeout
			match conn.process(Duration::from_secs(1)) {
				Ok(true) => {
					// Check the current lock state after receiving any message
					if let Ok(locked) = check_session_locked(&conn, &session_path) {
						update_lock_state(locked);
					}
				}
				Ok(false) => {
					// Timeout, continue waiting
				}
				Err(e) => {
					log::warn!("D-Bus process error: {e}");
					std::thread::sleep(Duration::from_secs(5));
				}
			}
		}
	}

	/// Get the D-Bus session path for the current user.
	fn get_session_path(conn: &Connection) -> Result<dbus::Path<'static>, dbus::Error> {
		let proxy = conn.with_proxy("org.freedesktop.login1", "/org/freedesktop/login1", Duration::from_secs(5));

		// Get the user's session by PID
		let (session_path,): (dbus::Path<'static>,) = proxy.method_call("org.freedesktop.login1.Manager", "GetSessionByPID", (std::process::id(),))?;

		Ok(session_path)
	}

	/// Check if the session is currently locked via D-Bus properties.
	fn check_session_locked(conn: &Connection, session_path: &dbus::Path) -> Result<bool, dbus::Error> {
		use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

		let proxy = conn.with_proxy("org.freedesktop.login1", session_path, Duration::from_secs(5));

		let locked: bool = proxy.get("org.freedesktop.login1.Session", "LockedHint")?;
		Ok(locked)
	}

	/// Fall back to polling the session lock status if signal monitoring fails.
	fn poll_screen_lock_status() {
		log::info!("Screen lock watcher using polling fallback");

		loop {
			if let Some(locked) = poll_lock_state_once() {
				update_lock_state(locked);
			}
			std::thread::sleep(Duration::from_secs(2));
		}
	}

	/// Poll the lock state once, returning None if any step fails.
	fn poll_lock_state_once() -> Option<bool> {
		let conn = Connection::new_system().ok()?;
		let session_path = get_session_path(&conn).ok()?;
		check_session_locked(&conn, &session_path).ok()
	}

	/// Update the global lock state and trigger device updates if state changed.
	fn update_lock_state(locked: bool) {
		let previous = SCREEN_LOCKED.swap(locked, Ordering::SeqCst);
		if previous != locked {
			log::info!("Screen lock state changed: {}", if locked { "locked" } else { "unlocked" });
			// Spawn a task to handle device state changes
			tokio::spawn(async move {
				if locked {
					crate::elgato::on_screen_locked().await;
				} else {
					crate::elgato::on_screen_unlocked().await;
				}
			});
		}
	}
}

#[cfg(target_os = "windows")]
mod platform {
	use super::*;

	use std::time::Duration;

	/// Watch for screen lock/unlock events on Windows.
	/// Uses WTSQuerySessionInformation to check the session state.
	pub async fn watch_screen_lock() {
		log::info!("Screen lock watcher initialized (Windows polling)");

		loop {
			let locked = is_session_locked();
			let previous = SCREEN_LOCKED.swap(locked, Ordering::SeqCst);

			if previous != locked {
				log::info!("Screen lock state changed: {}", if locked { "locked" } else { "unlocked" });
				if locked {
					crate::elgato::on_screen_locked().await;
				} else {
					crate::elgato::on_screen_unlocked().await;
				}
			}

			tokio::time::sleep(Duration::from_secs(2)).await;
		}
	}

	/// Check if the Windows session is locked.
	/// Uses WTSQuerySessionInformationW with WTSSessionInfoEx to get the session state.
	fn is_session_locked() -> bool {
		use windows_sys::Win32::System::RemoteDesktop::{WTS_SESSIONSTATE_LOCK, WTSFreeMemory, WTSINFOEXW, WTSQuerySessionInformationW, WTSSessionInfoEx};

		unsafe {
			let mut buffer: *mut u16 = std::ptr::null_mut();
			let mut bytes_returned: u32 = 0;

			// Query the session info for the current session (WTS_CURRENT_SESSION = 0xFFFFFFFF)
			let result = WTSQuerySessionInformationW(
				std::ptr::null_mut(), // WTS_CURRENT_SERVER_HANDLE
				0xFFFFFFFF,           // WTS_CURRENT_SESSION
				WTSSessionInfoEx,
				&mut buffer,
				&mut bytes_returned,
			);

			if result == 0 || buffer.is_null() {
				return false;
			}

			let info = &*(buffer as *const WTSINFOEXW);
			// Check if Level is 1 and SessionFlags indicates locked state
			let is_locked = info.Level == 1 && info.Data.WTSInfoExLevel1.SessionFlags == WTS_SESSIONSTATE_LOCK as i32;

			WTSFreeMemory(buffer as *mut std::ffi::c_void);

			is_locked
		}
	}
}

#[cfg(target_os = "macos")]
mod platform {
	use super::*;

	use std::process::Command;
	use std::time::Duration;

	/// Watch for screen lock/unlock events on macOS.
	/// Uses ioreg to check the screen lock state.
	pub async fn watch_screen_lock() {
		log::info!("Screen lock watcher initialized (macOS polling)");

		loop {
			let locked = is_session_locked();
			let previous = SCREEN_LOCKED.swap(locked, Ordering::SeqCst);

			if previous != locked {
				log::info!("Screen lock state changed: {}", if locked { "locked" } else { "unlocked" });
				if locked {
					crate::elgato::on_screen_locked().await;
				} else {
					crate::elgato::on_screen_unlocked().await;
				}
			}

			tokio::time::sleep(Duration::from_secs(2)).await;
		}
	}

	/// Check if the macOS session is locked.
	/// Uses ioreg to check the IOHIDSystem's HIDIdleTime and screen saver state.
	fn is_session_locked() -> bool {
		// Primary method: Check if the loginwindow is the frontmost app
		// This is the most reliable cross-version method
		if check_login_window_active() {
			return true;
		}

		// Secondary method: Check screen saver status via defaults
		if check_screen_saver_running() {
			return true;
		}

		false
	}

	/// Check if the loginwindow process has focus (indicates lock screen).
	fn check_login_window_active() -> bool {
		let output = Command::new("osascript")
			.args(["-e", "tell application \"System Events\" to get name of first process whose frontmost is true"])
			.output();

		match output {
			Ok(output) => {
				let stdout = String::from_utf8_lossy(&output.stdout);
				stdout.trim().to_lowercase() == "loginwindow"
			}
			Err(_) => false,
		}
	}

	/// Check if the screen saver is running.
	fn check_screen_saver_running() -> bool {
		// Check if ScreenSaverEngine is running
		let output = Command::new("pgrep").args(["-x", "ScreenSaverEngine"]).output();

		match output {
			Ok(output) => output.status.success(),
			Err(_) => false,
		}
	}
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod platform {
	use super::*;

	/// No-op implementation for unsupported platforms.
	pub async fn watch_screen_lock() {
		log::warn!("Screen lock detection is not supported on this platform");
	}
}
