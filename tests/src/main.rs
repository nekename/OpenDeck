use std::env;
use std::fs;
use std::process::Command;

use thirtyfour::prelude::*;

async fn sleep_secs(secs: u64) {
	tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
}

async fn run_tests() -> WebDriverResult<()> {
	let mut capabilities = Capabilities::new();
	capabilities.insert(
		"tauri:options".to_owned(),
		serde_json::json!({
			"application": fs::canonicalize("../src-tauri/target/debug/opendeck").unwrap(),
		}),
	);

	let driver = WebDriver::new("http://localhost:4444", capabilities).await?;
	sleep_secs(2).await; // Wait for the webpage to load

	driver.find(By::XPath("//*[text()='Plugins']")).await?.click().await?;
	sleep_secs(2).await;

	Ok(())
}

#[tokio::main]
async fn main() {
	if fs::exists("target/opendeck").unwrap_or(false) {
		fs::remove_dir_all("target/opendeck").expect("to be able to remove leftover OpenDeck data");
	}
	fs::create_dir_all("target/opendeck/config").expect("to be able to create OpenDeck config directory");
	// SAFETY: std::env::set_var can cause race conditions in multithreaded contexts. We have not spawned any other threads at this point.
	unsafe {
		std::env::set_var("OPENDECK_CONFIG_DIR", fs::canonicalize("target/opendeck/config").unwrap());
	}

	// Build the application
	if !Command::new("deno")
		.args(["task", "tauri", "build", "--debug", "--no-bundle"])
		.current_dir("..")
		.status()
		.expect("to be able to build the application")
		.success()
	{
		panic!("Failed to build the application");
	}

	// Start tauri-driver
	let mut tauri_driver = Command::new(env::home_dir().unwrap().join(".cargo/bin/tauri-driver"))
		.current_dir("..")
		.spawn()
		.expect("to be able to start tauri-driver");

	sleep_secs(1).await; // Wait for tauri-driver to start
	if let Err(error) = run_tests().await {
		eprintln!("Failed to run tests: {error}");
	}
	sleep_secs(1).await; // Wait for tauri-driver to clean up the application process

	tauri_driver.kill().expect("to be able to kill tauri-driver");
	tauri_driver.wait().expect("to be able to wait for tauri-driver to exit");
	#[cfg(target_os = "linux")]
	Command::new("killall").args(["-9", "WebKitWebDriver"]).status().expect("to be able to kill WebKitWebDriver");
}
