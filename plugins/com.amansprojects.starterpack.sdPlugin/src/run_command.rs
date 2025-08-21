use std::io::Read;
use std::process::{Command, Stdio};

use super::ActionEvent;

use openaction::*;

#[cfg(unix)]
fn is_flatpak() -> bool {
	use std::env::var;
	var("FLATPAK_ID").is_ok()
		|| var("container")
			.map(|x| x.to_lowercase().trim() == "flatpak")
			.unwrap_or(false)
}

async fn run_command(
	event: impl ActionEvent,
	action: &str,
	ticks: Option<i16>,
) -> EventHandlerResult {
	let Some(settings) = event.settings().as_object() else {
		return Ok(());
	};
	let Some(mut value) = settings
		.get(action)
		.and_then(|v| v.as_str())
		.map(|x| x.to_owned())
	else {
		return Ok(());
	};
	if value.is_empty() {
		return Ok(());
	}
	if let Some(ticks) = ticks {
		value = value.replace("%d", &ticks.to_string());
	}

	#[cfg(unix)]
	let command = if is_flatpak() { "flatpak-spawn" } else { "sh" };
	#[cfg(unix)]
	let extra_args = if is_flatpak() {
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
	if let Some(home_dir) = std::env::home_dir() {
		command.current_dir(home_dir);
	}
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
			.set_title(
				event.context().clone(),
				Some(output.trim().to_owned()),
				None,
			)
			.await?;
	}

	Ok(())
}

pub fn down_up(
	direction: &'static str,
	event: impl ActionEvent + Send + 'static,
) -> EventHandlerResult {
	tokio::spawn(async move {
		if let Err(error) = run_command(event, direction, None).await {
			log::warn!("Failed to run command: {error}");
		}
	});

	Ok(())
}

pub fn rotate(event: DialRotateEvent) -> EventHandlerResult {
	tokio::spawn(async move {
		let ticks = event.payload.ticks;
		if let Err(error) = run_command(event, "rotate", Some(ticks)).await {
			log::warn!("Failed to run command: {error}");
		}
	});

	Ok(())
}
