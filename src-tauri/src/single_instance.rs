//! Drop-in replacement for the subset of `tauri-plugin-single-instance` used by OpenDeck, which has no FreeBSD backend.
//!
//! The Linux implementation relies on D-Bus name ownership. Here, the instance that holds an exclusive `flock` on
//! `~/.cache/<app>/instance/<hostuuid>.lock` owns `<hostuuid>.sock` next to it; later launches forward their
//! command-line arguments over that socket and exit. The home directory comes from the passwd entry and the host ID
//! from `kern.hostuuid`, so every launch by the same user on the same host finds the same files regardless of its
//! environment, while hosts sharing a home directory stay separate. Every directory on the path is checked before
//! use, and an unsafe or unusable path is a startup error rather than a reason to run without protection.

use std::ffi::{CStr, OsString};
use std::fs::{DirBuilder, File, Metadata, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{SocketAddr, UnixListener, UnixStream};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Manager, RunEvent, Runtime};

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
				// Fail closed: without a safe socket and lock, refuse to start rather than run unprotected.
				let (paths, lock) = match InstancePaths::new(&app.package_info().name).and_then(|paths| Ok((try_lock(&paths.lock)?, paths))) {
					Ok((Some(lock), paths)) => (paths, lock),
					Ok((None, paths)) => forward_to_existing_instance(&paths),
					Err(error) => refuse_to_start(&error),
				};
				let listener = listen(&paths).unwrap_or_else(|error| refuse_to_start(&error));
				app.manage(SocketPath(paths.socket));

				let schemes = deep_link_schemes(app);
				let app = app.clone();
				let mut callback = callback.take();
				std::thread::spawn(move || {
					// The lock marks this process as the owner for as long as it runs.
					let _lock = lock;
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
				if let RunEvent::Exit = event
					&& let Some(socket) = app.try_state::<SocketPath>()
				{
					let _ = std::fs::remove_file(&socket.0);
				}
			})
			.build()
	}
}

/// The socket this instance listens on, removed on exit.
struct SocketPath(PathBuf);

struct InstancePaths {
	lock: PathBuf,
	socket: PathBuf,
	address: SocketAddr,
}

impl InstancePaths {
	fn new(app_name: &str) -> io::Result<Self> {
		// SAFETY: getuid has no preconditions.
		let uid = unsafe { libc::getuid() };
		let home = resolve_trusted(&passwd_home(uid)?, uid)?;
		let cache = home.join(".cache");
		create_dir(&cache)?;
		let cache = resolve_trusted(&cache, uid)?;
		let app_cache = cache.join(app_name);
		create_dir(&app_cache)?;
		// The app's cache directory is shared with WebKit and is not private, so the files go in a subdirectory.
		let dir = resolve_trusted(&app_cache, uid)?.join("instance");
		create_dir(&dir)?;
		check_private(&dir, &std::fs::symlink_metadata(&dir)?, uid)?;

		let host = host_uuid()?;
		let lock = dir.join(format!("{host}.lock"));
		let socket = dir.join(format!("{host}.sock"));
		let address = SocketAddr::from_pathname(&socket).map_err(|error| io::Error::new(error.kind(), format!("cannot use socket path {}: {error}", socket.display())))?;
		Ok(Self { lock, socket, address })
	}
}

/// The home directory from the passwd entry, so that it does not depend on `$HOME`.
fn passwd_home(uid: libc::uid_t) -> io::Result<PathBuf> {
	let mut passwd = std::mem::MaybeUninit::<libc::passwd>::uninit();
	let mut buffer = vec![0 as libc::c_char; 16384];
	let mut result = std::ptr::null_mut();
	// SAFETY: `passwd`, `buffer` and `result` are valid for writes, and `buffer.len()` is the buffer's size.
	let status = unsafe { libc::getpwuid_r(uid, passwd.as_mut_ptr(), buffer.as_mut_ptr(), buffer.len(), &raw mut result) };
	if status != 0 {
		return Err(io::Error::from_raw_os_error(status));
	}
	if result.is_null() {
		return Err(io::Error::other(format!("no passwd entry for uid {uid}")));
	}
	// SAFETY: getpwuid_r succeeded, so `passwd` is initialised and `pw_dir` points to a C string inside `buffer`.
	let dir = unsafe { CStr::from_ptr(passwd.assume_init().pw_dir) };
	Ok(PathBuf::from(OsString::from_vec(dir.to_bytes().to_vec())))
}

/// `kern.hostuuid`, which is unique per host and does not change with the hostname.
fn host_uuid() -> io::Result<String> {
	let mut value = [0u8; 64];
	let mut len = value.len();
	// SAFETY: the name is a valid C string, and `value`/`len` describe a writable buffer of `len` bytes.
	if unsafe { libc::sysctlbyname(c"kern.hostuuid".as_ptr(), value.as_mut_ptr().cast(), &raw mut len, std::ptr::null(), 0) } != 0 {
		return Err(io::Error::last_os_error());
	}
	let uuid = CStr::from_bytes_until_nul(&value[..len]).map_err(io::Error::other)?.to_string_lossy().into_owned();
	if uuid.is_empty() || !uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
		return Err(io::Error::other(format!("unexpected kern.hostuuid value {uuid:?}")));
	}
	Ok(uuid)
}

/// Resolves `path` one component at a time, checking each directory with [`check_trusted`] before going into it and
/// each symlink before following it, so no other user can redirect or replace any part of the path.
fn resolve_trusted(path: &Path, uid: libc::uid_t) -> io::Result<PathBuf> {
	let mut resolved = PathBuf::from("/");
	check_trusted(&resolved, &std::fs::symlink_metadata(&resolved)?, uid)?;
	let mut pending: Vec<OsString> = Vec::new();
	push_components(&mut pending, path);
	let mut links = 0;
	while let Some(component) = pending.pop() {
		if component == ".." {
			resolved.pop();
			continue;
		}
		let next = resolved.join(&component);
		let metadata = std::fs::symlink_metadata(&next)?;
		if metadata.file_type().is_symlink() {
			links += 1;
			if links > 32 {
				return Err(io::Error::other(format!("too many symlinks resolving {}", path.display())));
			}
			check_owner(&next, &metadata, uid)?;
			let target = std::fs::read_link(&next)?;
			if target.is_absolute() {
				resolved = PathBuf::from("/");
			}
			push_components(&mut pending, &target);
			continue;
		}
		check_trusted(&next, &metadata, uid)?;
		resolved = next;
	}
	Ok(resolved)
}

/// Queues the components of `path` so that `pending.pop()` yields them in order.
fn push_components(pending: &mut Vec<OsString>, path: &Path) {
	let start = pending.len();
	for component in path.components() {
		match component {
			Component::Normal(name) => pending.push(name.to_owned()),
			Component::ParentDir => pending.push("..".into()),
			Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
		}
	}
	pending[start..].reverse();
}

/// Creates `path` with mode 0700 if it does not exist. Its parent must already be trusted; callers check the result.
fn create_dir(path: &Path) -> io::Result<()> {
	match DirBuilder::new().mode(0o700).create(path) {
		Ok(()) => Ok(()),
		Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
		Err(error) => Err(io::Error::new(error.kind(), format!("cannot create {}: {error}", path.display()))),
	}
}

fn check_owner(path: &Path, metadata: &Metadata, uid: libc::uid_t) -> io::Result<()> {
	if metadata.uid() != 0 && metadata.uid() != uid {
		return Err(io::Error::other(format!("{} is owned by uid {}, not by root or uid {uid}", path.display(), metadata.uid())));
	}
	Ok(())
}

/// A directory no other user can modify: owned by root or `uid`, and not group- or world-writable.
fn check_trusted(path: &Path, metadata: &Metadata, uid: libc::uid_t) -> io::Result<()> {
	if !metadata.is_dir() {
		return Err(io::Error::other(format!("{} is not a directory", path.display())));
	}
	check_owner(path, metadata, uid)?;
	if metadata.mode() & 0o022 != 0 {
		return Err(io::Error::other(format!("{} is writable by other users (mode {:o})", path.display(), metadata.mode() & 0o777)));
	}
	Ok(())
}

/// The directory holding the lock and socket: a real directory owned by `uid` that no other user can access.
fn check_private(path: &Path, metadata: &Metadata, uid: libc::uid_t) -> io::Result<()> {
	if metadata.file_type().is_symlink() || !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
		return Err(io::Error::other(format!("{} must be a directory owned by uid {uid} with mode 0700", path.display())));
	}
	Ok(())
}

/// Logs why the single-instance setup failed and exits. Returning the error from plugin setup instead would go
/// through Tauri's builder panic, which can hang during runtime shutdown.
fn refuse_to_start(error: &io::Error) -> ! {
	log::error!("Single instance check failed, not starting: {error}");
	eprintln!("Single instance check failed, not starting: {error}");
	std::process::exit(1);
}

/// Replaces any stale socket left by an earlier owner and listens on it. Only the lock holder calls this.
fn listen(paths: &InstancePaths) -> io::Result<UnixListener> {
	match std::fs::remove_file(&paths.socket) {
		Ok(()) => {}
		Err(error) if error.kind() == io::ErrorKind::NotFound => {}
		Err(error) => return Err(io::Error::new(error.kind(), format!("cannot remove stale socket {}: {error}", paths.socket.display()))),
	}
	let listener = UnixListener::bind_addr(&paths.address).map_err(|error| io::Error::new(error.kind(), format!("cannot listen on {}: {error}", paths.socket.display())))?;
	std::fs::set_permissions(&paths.socket, std::fs::Permissions::from_mode(0o600))?;
	Ok(listener)
}

/// Takes the instance lock. Returns `None` if another instance holds it.
fn try_lock(path: &Path) -> io::Result<Option<File>> {
	let file = OpenOptions::new().read(true).write(true).create(true).mode(0o600).custom_flags(libc::O_NOFOLLOW).open(path)?;
	// SAFETY: `file` owns a valid open file descriptor.
	if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
		return Ok(Some(file));
	}
	let error = io::Error::last_os_error();
	if error.kind() == io::ErrorKind::WouldBlock { Ok(None) } else { Err(error) }
}

/// Sends this process's arguments to the instance that holds the lock, then exits.
/// The owner may still be starting up, so this retries connecting for about two seconds.
fn forward_to_existing_instance(paths: &InstancePaths) -> ! {
	let mut stream = None;
	for _ in 0..40 {
		if let Ok(connected) = UnixStream::connect_addr(&paths.address) {
			stream = Some(connected);
			break;
		}
		std::thread::sleep(Duration::from_millis(50));
	}
	let Some(mut stream) = stream else {
		eprintln!("Another instance holds the single-instance lock but did not accept a connection on {}", paths.socket.display());
		std::process::exit(1);
	};
	let args: Vec<String> = std::env::args().collect();
	let cwd = std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
	let message = serde_json::to_string(&(args, cwd)).expect("a tuple of strings always serialises");
	match stream.write_all(message.as_bytes()).and_then(|()| stream.shutdown(std::net::Shutdown::Write)) {
		Ok(()) => std::process::exit(0),
		Err(error) => {
			eprintln!("Failed to send arguments to the running instance: {error}");
			std::process::exit(1);
		}
	}
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
