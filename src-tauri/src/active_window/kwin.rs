//! KDE Wayland source: a single KWin script, loaded once, pushes focus changes to us over D-Bus.
//!
//! This replaces polling through `kdotool`, which opens fresh D-Bus connections and loads/unloads a KWin script on every call.

use std::time::Duration;

use anyhow::Context;
use futures::StreamExt;
use tokio::sync::mpsc;
use zbus::{Connection, Proxy, fdo::DBusProxy};

const SCRIPT_NAME: &str = "opendeck-active-window";
const OBJECT_PATH: &str = "/me/amankhanna/opendeck/ActiveWindow";
const INTERFACE: &str = "me.amankhanna.opendeck.ActiveWindow";

struct Receiver {
	tx: mpsc::Sender<String>,
}

#[zbus::interface(name = "me.amankhanna.opendeck.ActiveWindow")]
impl Receiver {
	async fn changed(&self, app_name: String) {
		let _ = self.tx.send(app_name).await;
	}
}

pub fn available() -> bool {
	std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.to_uppercase().contains("KDE"))
}

pub async fn run(tx: mpsc::Sender<String>) -> anyhow::Result<()> {
	let (raw_tx, mut raw_rx) = mpsc::channel(16);
	let connection = zbus::connection::Builder::session()?.serve_at(OBJECT_PATH, Receiver { tx: raw_tx })?.build().await?;
	let address = connection.unique_name().context("connection has no unique name")?.to_string();

	// Reload the script if KWin restarts, as it forgets all loaded scripts.
	let mut kwin_owner_changes = DBusProxy::new(&connection).await?.receive_name_owner_changed_with_args(&[(0, "org.kde.KWin")]).await?;

	load_script(&connection, &address).await?;

	// The script reports the current window immediately, so silence means it isn't working.
	let mut last = tokio::time::timeout(Duration::from_secs(5), raw_rx.recv())
		.await
		.context("timed out waiting for KWin script")?
		.context("channel closed")?;
	if tx.send(last.clone()).await.is_err() {
		return Ok(());
	}

	loop {
		tokio::select! {
			Some(app_name) = raw_rx.recv() => {
				if app_name != last {
					last = app_name.clone();
					if tx.send(app_name).await.is_err() {
						return Ok(());
					}
				}
			}
			Some(signal) = kwin_owner_changes.next() => {
				if signal.args()?.new_owner().is_some() {
					let mut result = Ok(());
					for _ in 0..5 {
						tokio::time::sleep(Duration::from_secs(1)).await;
						result = load_script(&connection, &address).await;
						if result.is_ok() {
							break;
						}
					}
					result.context("failed to reload KWin script after KWin restart")?;
				}
			}
		}
	}
}

async fn load_script(connection: &Connection, address: &str) -> anyhow::Result<()> {
	let kde5 = std::env::var("KDE_SESSION_VERSION").is_ok_and(|v| v == "5");
	let (active, signal) = if kde5 { ("activeClient", "clientActivated") } else { ("activeWindow", "windowActivated") };
	let script = format!(
		r#"const send = () => {{
	const w = workspace.{active};
	callDBus("{address}", "{OBJECT_PATH}", "{INTERFACE}", "Changed", w ? String(w.resourceClass) : "");
}};
workspace.{signal}.connect(send);
send();
"#
	);

	// Prefer the runtime directory, as KWin must be able to read the file and it is per-user.
	let directory = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
	let path = directory.join(format!("{SCRIPT_NAME}.js"));
	std::fs::write(&path, script)?;

	let scripting = Proxy::new(connection, "org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting").await?;
	// Remove any copy left behind by a previous run.
	let _: bool = scripting.call("unloadScript", &(SCRIPT_NAME,)).await?;
	let id: i32 = scripting.call("loadScript", &(path.to_str().context("non-UTF-8 script path")?, SCRIPT_NAME)).await?;
	anyhow::ensure!(id >= 0, "KWin refused to load script");

	let script_path = if kde5 { format!("/{id}") } else { format!("/Scripting/Script{id}") };
	let script_proxy = Proxy::new(connection, "org.kde.KWin", script_path, "org.kde.kwin.Script").await?;
	script_proxy.call::<_, _, ()>("run", &()).await?;
	Ok(())
}
