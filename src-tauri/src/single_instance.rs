//! Drop-in replacement for the subset of `tauri-plugin-single-instance` used by OpenDeck, which has no FreeBSD backend.
//!
//! The first instance listens on a Unix domain socket in `$XDG_RUNTIME_DIR`; later instances forward their
//! command-line arguments to it and exit, as the Linux implementation does over D-Bus.

use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, RunEvent, Runtime};

type Callback<R> = Box<dyn FnMut(&AppHandle<R>, Vec<String>, String) + Send + Sync + 'static>;

pub struct Builder<R: Runtime> {
	callback: Option<Callback<R>>,
}

impl<R: Runtime> Default for Builder<R> {
	fn default() -> Self {
		Self { callback: None }
	}
}

impl<R: Runtime> Builder<R> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn callback<F: FnMut(&AppHandle<R>, Vec<String>, String) + Send + Sync + 'static>(mut self, callback: F) -> Self {
		self.callback = Some(Box::new(callback));
		self
	}

	/// Accepted for API compatibility; D-Bus is not used.
	pub fn dbus_id(self, _id: impl Into<String>) -> Self {
		self
	}

	pub fn build(self) -> TauriPlugin<R> {
		let mut callback = self.callback;
		tauri::plugin::Builder::new("single-instance")
			.setup(move |app, _api| {
				let path = socket_path(&app.package_info().name);
				forward_to_existing_instance(&path);
				let listener = bind(&path)?;
				let schemes = deep_link_schemes(app);
				let app = app.clone();
				let mut callback = callback.take();
				std::thread::spawn(move || {
					for mut stream in listener.incoming().flatten() {
						let mut buffer = String::new();
						if stream.read_to_string(&mut buffer).is_err() {
							continue;
						}
						let (args, cwd) = match serde_json::from_str::<(Vec<String>, String)>(&buffer) {
							Ok(message) => message,
							Err(error) => {
								log::warn!("Ignoring malformed arguments from another instance: {error}");
								continue;
							}
						};
						emit_deep_link(&app, &schemes, &args);
						if let Some(callback) = callback.as_mut() {
							callback(&app, args, cwd);
						}
					}
				});
				Ok(())
			})
			.on_event(|app, event| {
				if let RunEvent::Exit = event {
					let _ = std::fs::remove_file(socket_path(&app.package_info().name));
				}
			})
			.build()
	}
}

fn socket_path(app_name: &str) -> PathBuf {
	let dir = std::env::var("XDG_RUNTIME_DIR").map_or_else(
		// SAFETY: getuid has no preconditions.
		|_| std::env::temp_dir().join(format!("{app_name}-{}", unsafe { libc::getuid() })),
		PathBuf::from,
	);
	dir.join(format!("{app_name}-single-instance.sock"))
}

/// If another instance is listening, send it this process's arguments and exit.
/// Returns only if no instance is listening.
fn forward_to_existing_instance(path: &Path) {
	let Ok(mut stream) = UnixStream::connect(path) else {
		return;
	};
	let args: Vec<String> = std::env::args().collect();
	let cwd = std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
	let message = serde_json::to_string(&(args, cwd)).expect("a tuple of strings always serialises");
	match stream.write_all(message.as_bytes()).and_then(|()| stream.shutdown(std::net::Shutdown::Write)) {
		Ok(()) => std::process::exit(0),
		Err(error) => {
			eprintln!("Another instance is running at {} but could not be contacted: {error}", path.display());
			std::process::exit(1);
		}
	}
}

fn bind(path: &Path) -> std::io::Result<UnixListener> {
	// Nothing is listening, so any existing socket file is stale.
	let _ = std::fs::remove_file(path);
	if let Some(parent) = path.parent()
		&& !parent.exists()
	{
		std::fs::create_dir_all(parent)?;
		std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
	}
	let listener = UnixListener::bind(path)?;
	std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
	Ok(listener)
}

/// The desktop schemes configured for `tauri-plugin-deep-link` (`plugins.deep-link.desktop.schemes`).
fn deep_link_schemes<R: Runtime>(app: &AppHandle<R>) -> Vec<String> {
	app.config()
		.plugins
		.0
		.get("deep-link")
		.and_then(|config| config.pointer("/desktop/schemes"))
		.and_then(|schemes| serde_json::from_value(schemes.clone()).ok())
		.unwrap_or_default()
}

/// Equivalent of the `deep-link` feature of `tauri-plugin-single-instance`: a launch whose only argument is a URL
/// with one of the configured schemes is forwarded to `onOpenUrl` listeners.
fn emit_deep_link<R: Runtime>(app: &AppHandle<R>, schemes: &[String], args: &[String]) {
	if let [_, arg] = args
		&& let Ok(url) = reqwest::Url::parse(arg)
		&& schemes.iter().any(|scheme| scheme == url.scheme())
	{
		let _ = app.emit("deep-link://new-url", vec![url.to_string()]);
	}
}
