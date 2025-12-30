use std::io::Read;
use std::process::{Command, Stdio};

use openaction::*;

use serde::{Deserialize, Serialize};

#[cfg(unix)]
fn is_flatpak() -> bool {
	use std::env::var;
	var("FLATPAK_ID").is_ok()
		|| var("container")
			.map(|x| x.to_lowercase().trim() == "flatpak")
			.unwrap_or(false)
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct RunCommandSettings {
	down: Option<String>,
	up: Option<String>,
	rotate: Option<String>,
	file: Option<String>,
	show: bool,
}

async fn run_command(
	instance_id: InstanceId,
	settings: &RunCommandSettings,
	value: &Option<String>,
	ticks: Option<i16>,
) -> Result<(), anyhow::Error> {
	let Some(mut value) = value.clone() else {
		return Ok(());
	};
	if value.trim().is_empty() {
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

	if let Some(path) = settings.file.as_deref()
		&& !path.is_empty()
	{
		tokio::fs::write(path, &output).await?;
	}

	if settings.show
		&& let Some(instance) = get_instance(instance_id).await
	{
		instance
			.set_title(Some(output.trim().to_owned()), None)
			.await?;
	}

	Ok(())
}

pub struct RunCommandAction;
#[async_trait]
impl Action for RunCommandAction {
	const UUID: &'static str = "com.amansprojects.starterpack.runcommand";
	type Settings = RunCommandSettings;

	async fn key_down(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		let instance_id = instance.instance_id.clone();
		let settings = settings.clone();
		tokio::spawn(async move {
			if let Err(error) = run_command(instance_id, &settings, &settings.down, None).await {
				log::warn!("Failed to run command: {error}");
			}
		});
		Ok(())
	}

	async fn key_up(&self, instance: &Instance, settings: &Self::Settings) -> OpenActionResult<()> {
		let instance_id = instance.instance_id.clone();
		let settings = settings.clone();
		tokio::spawn(async move {
			if let Err(error) = run_command(instance_id, &settings, &settings.up, None).await {
				log::warn!("Failed to run command: {error}");
			}
		});
		Ok(())
	}

	async fn dial_down(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		self.key_down(instance, settings).await
	}

	async fn dial_up(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		self.key_up(instance, settings).await
	}

	async fn dial_rotate(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
		ticks: i16,
		_pressed: bool,
	) -> OpenActionResult<()> {
		let instance_id = instance.instance_id.clone();
		let settings = settings.clone();
		tokio::spawn(async move {
			if let Err(error) =
				run_command(instance_id, &settings, &settings.rotate, Some(ticks)).await
			{
				log::warn!("Failed to run command: {error}");
			}
		});

		Ok(())
	}
}
