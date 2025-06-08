//! Original source from https://github.com/MCOfficer/zip-extract/blob/master/src/lib.rs, modified to better suit extracting OpenAction plugins.

#![forbid(unsafe_code)]

use std::io::{BufReader, Cursor, Read, Seek};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::{fs, io};

use log::{debug, trace};

#[derive(Debug)]
#[allow(dead_code)]
pub enum ZipExtractError {
	Io(io::Error),
	Zip(zip::result::ZipError),
}
impl From<io::Error> for ZipExtractError {
	fn from(value: io::Error) -> Self {
		Self::Io(value)
	}
}
impl From<zip::result::ZipError> for ZipExtractError {
	fn from(value: zip::result::ZipError) -> Self {
		Self::Zip(value)
	}
}
impl std::fmt::Display for ZipExtractError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self)
	}
}
impl std::error::Error for ZipExtractError {}

pub fn dir_name<S: Read + Seek>(source: S) -> Result<String, ZipExtractError> {
	let mut archive = zip::ZipArchive::new(source)?;

	if archive.len() == 1 {
		let file = archive.by_index(0)?;
		if file.is_file() {
			return dir_name(Cursor::new(BufReader::new(file).bytes().flatten().collect::<Vec<u8>>()));
		}
	}

	for i in 0..archive.len() {
		let file = archive.by_index(i)?;
		if let Some(c) = PathBuf::from(file.name().replace('\\', "/"))
			.components()
			.find(|c| c.as_os_str().to_string_lossy().to_lowercase().ends_with(".sdplugin"))
		{
			return Ok(c.as_os_str().to_string_lossy().to_string());
		}
	}

	Err(ZipExtractError::Zip(zip::result::ZipError::FileNotFound))
}

pub fn extract<S: Read + Seek>(source: S, target_dir: &Path) -> Result<(), ZipExtractError> {
	if !target_dir.exists() {
		fs::create_dir(target_dir)?;
	}

	let mut archive = zip::ZipArchive::new(source)?;

	if archive.len() == 1 {
		let file = archive.by_index(0)?;
		if file.is_file() {
			return extract(Cursor::new(BufReader::new(file).bytes().flatten().collect::<Vec<u8>>()), target_dir);
		}
	}

	debug!("Extracting to {}", target_dir.to_string_lossy());
	for i in 0..archive.len() {
		let mut file = archive.by_index(i)?;
		let relative_path = file.mangled_name();

		if relative_path.to_string_lossy().is_empty() {
			continue;
		}

		let mut outpath = target_dir.to_path_buf();
		outpath.push(relative_path);

		let name = file.name().replace('\\', "/");

		trace!("Extracting {} to {}", name, outpath.to_string_lossy());
		if name.ends_with('/') {
			fs::create_dir_all(&outpath)?;
		} else {
			if let Some(p) = outpath.parent() {
				if !p.exists() {
					fs::create_dir_all(p)?;
				}
			}
			let mut outfile = fs::File::create(&outpath)?;
			io::copy(&mut file, &mut outfile)?;
		}

		#[cfg(unix)]
		set_unix_mode(&file, &outpath)?;
	}

	debug!("Extracted {} files", archive.len());
	Ok(())
}

#[cfg(unix)]
fn set_unix_mode<R: Read>(file: &zip::read::ZipFile<R>, outpath: &Path) -> io::Result<()> {
	if let Some(mut m) = file.unix_mode() {
		m %= 0o1000;
		m = if m >= 0o700 || file.is_dir() { 0o755 } else { 0o644 };
		fs::set_permissions(outpath, PermissionsExt::from_mode(m))?
	}
	Ok(())
}
