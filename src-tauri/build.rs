use std::fs;

fn main() {
	#[cfg(not(debug_assertions))]
	println!("cargo:rerun-if-changed=../plugins");
	if let Err(error) = || -> Result<(), std::io::Error> {
		for entry in fs::read_dir("../plugins")?.flatten() {
			let out_dir = std::path::Path::new("target").join("plugins").join(entry.file_name());
			fs::create_dir_all(&out_dir)?;
			let status = std::process::Command::new("deno")
				.args([
					"run",
					"--lock=target/deno.lock",
					"--allow-all",
					"build.ts",
					fs::canonicalize(out_dir)?.to_string_lossy().as_ref(),
					&std::env::var("TARGET").unwrap(),
				])
				.current_dir(entry.path())
				.status()?;
			if !status.success() {
				panic!("Failed to build plugin {}: status code {}", entry.file_name().to_string_lossy(), status.code().unwrap());
			}
		}

		Ok(())
	}() {
		#[cfg(debug_assertions)]
		eprintln!("Failed to build builtin plugins: {error}");
		#[cfg(not(debug_assertions))]
		panic!("Failed to build builtin plugins: {error}");
	}

	built::write_built_file().expect("failed to acquire build-time information");
	tauri_build::build();
}
