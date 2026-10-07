//! X11 source: listens for `_NET_ACTIVE_WINDOW` changes on the root window instead of polling.

use anyhow::Context;
use tokio::sync::mpsc;
use xcb::{Connection, Xid, x};

pub fn available() -> bool {
	std::env::var_os("DISPLAY").is_some()
}

pub async fn run(tx: mpsc::Sender<String>) -> anyhow::Result<()> {
	tokio::task::spawn_blocking(move || listen(tx)).await?
}

fn listen(tx: mpsc::Sender<String>) -> anyhow::Result<()> {
	let (connection, screen) = Connection::connect(None)?;
	let root = connection.get_setup().roots().nth(screen as usize).context("no root window")?.root();
	let active_window_atom = intern(&connection, b"_NET_ACTIVE_WINDOW")?;
	anyhow::ensure!(active_window_atom != x::ATOM_NONE, "window manager does not support _NET_ACTIVE_WINDOW");

	connection.send_and_check_request(&x::ChangeWindowAttributes {
		window: root,
		value_list: &[x::Cw::EventMask(x::EventMask::PROPERTY_CHANGE)],
	})?;

	let mut last = app_name(&connection, root, active_window_atom);
	if tx.blocking_send(last.clone()).is_err() {
		return Ok(());
	}

	loop {
		if let xcb::Event::X(x::Event::PropertyNotify(event)) = connection.wait_for_event()?
			&& event.atom() == active_window_atom
		{
			let name = app_name(&connection, root, active_window_atom);
			if name != last {
				last = name.clone();
				if tx.blocking_send(name).is_err() {
					return Ok(());
				}
			}
		}
	}
}

fn intern(connection: &Connection, name: &[u8]) -> xcb::Result<x::Atom> {
	let cookie = connection.send_request(&x::InternAtom { only_if_exists: true, name });
	Ok(connection.wait_for_reply(cookie)?.atom())
}

/// Matches how `active-win-pos-rs` names X11 windows: the last non-empty entry of `WM_CLASS`.
fn app_name(connection: &Connection, root: x::Window, active_window_atom: x::Atom) -> String {
	let lookup = || -> xcb::Result<String> {
		let cookie = connection.send_request(&x::GetProperty {
			delete: false,
			window: root,
			property: active_window_atom,
			r#type: x::ATOM_WINDOW,
			long_offset: 0,
			long_length: 1,
		});
		let reply = connection.wait_for_reply(cookie)?;
		let Some(window) = reply.value::<x::Window>().first().copied().filter(|w| !w.is_none()) else {
			return Ok(String::new());
		};

		let cookie = connection.send_request(&x::GetProperty {
			delete: false,
			window,
			property: x::ATOM_WM_CLASS,
			r#type: x::ATOM_STRING,
			long_offset: 0,
			long_length: 1024,
		});
		let reply = connection.wait_for_reply(cookie)?;
		let class = std::str::from_utf8(reply.value()).unwrap_or("");
		Ok(class.split('\0').rfind(|s| !s.is_empty()).unwrap_or("").to_owned())
	};
	lookup().unwrap_or_default()
}
