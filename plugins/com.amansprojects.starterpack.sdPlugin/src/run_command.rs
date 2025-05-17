use std::io::Read;
use std::process::{Command, Stdio};

use openaction::*;

async fn run_command(event: KeyEvent, action: &str) -> EventHandlerResult {
	let settings = event.payload.settings.as_object().unwrap();
	let Some(value) = settings.get(action).map(|v| v.as_str().unwrap()) else {
		return Ok(());
	};
	if value.is_empty() {
		return Ok(());
	}

	#[cfg(unix)]
	let command = if std::env::var("container").is_ok() {
		"flatpak-spawn"
	} else {
		"sh"
	};
	#[cfg(unix)]
	let extra_args = if std::env::var("container").is_ok() {
		vec!["--host", "sh", "-c"]
	} else {
		vec!["-c"]
	};

	#[cfg(windows)]
	let command = "cmd";
	#[cfg(windows)]
	let extra_args = ["/C"];

	let (mut reader, writer) = os_pipe::pipe()?;
	let mut command = Command::new(command);
	command
		.args(extra_args)
		.arg(value)
		.stdout(Stdio::from(writer.try_clone()?))
		.stderr(Stdio::from(writer));
	command.spawn()?.wait()?;
	drop(command);
	let mut output = String::new();
	reader.read_to_string(&mut output)?;

	if let Some(path) = settings.get("file").map(|v| v.as_str().unwrap()) {
		if !path.is_empty() {
			tokio::fs::write(path, &output).await?;
		}
	}

	if settings
		.get("show")
		.unwrap_or(&serde_json::Value::Bool(false))
		.as_bool()
		.unwrap()
	{
		let mut lock = OUTBOUND_EVENT_MANAGER.lock().await;
		let outbound = lock.as_mut().unwrap();
		outbound
			.set_title(event.context, Some(output.trim().to_owned()), None)
			.await?;
	}

	Ok(())
}

pub fn key_down(event: KeyEvent) -> EventHandlerResult {
	tokio::spawn(async move {
		if let Err(error) = run_command(event, "down").await {
			log::warn!("Failed to run command: {error}");
		}
	});

	Ok(())
}

pub fn key_up(event: KeyEvent) -> EventHandlerResult {
	tokio::spawn(async move {
		if let Err(error) = run_command(event, "up").await {
			log::warn!("Failed to run command: {error}");
		}
	});

	Ok(())
}
