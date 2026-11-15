use crate::model::Project;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;
static TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectFile {
    schema_version: u32,
    project: Project,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub language: String,
    pub theme: Theme,
    pub reduced_motion: bool,
    pub startup_update_check: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: "en".into(),
            theme: Theme::Light,
            reduced_motion: false,
            startup_update_check: true,
        }
    }
}

pub fn preferences_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("LOCALAPPDATA").ok_or("settings_directory_unavailable")?;
    Ok(PathBuf::from(base).join("CutPlanner").join("settings.json"))
}

pub fn read_text(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_FILE_SIZE {
        return Err("file_size_limit".into());
    }
    let mut text = String::new();
    file.take(MAX_FILE_SIZE + 1)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    if text.len() as u64 > MAX_FILE_SIZE {
        return Err("file_size_limit".into());
    }
    Ok(text)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    serde_json::from_str(&read_text(path)?).map_err(|e| e.to_string())
}

pub fn save_project(path: &Path, project: &Project) -> Result<(), String> {
    project.validate().map_err(|e| e.to_string())?;
    let data = serde_json::to_vec_pretty(&ProjectFile {
        schema_version: 1,
        project: project.clone(),
    })
    .map_err(|e| e.to_string())?;
    atomic_write(path, &data)
}

pub fn load_project(path: &Path) -> Result<Project, String> {
    let file: ProjectFile = read_json(path)?;
    if file.schema_version != 1 {
        return Err("unsupported_project_version".into());
    }
    file.project.validate().map_err(|e| e.to_string())?;
    Ok(file.project)
}

pub fn load_preferences(path: &Path) -> Result<Preferences, String> {
    if !path.exists() {
        return Ok(Preferences::default());
    }
    read_json(path)
}

pub fn save_preferences(path: &Path, preferences: &Preferences) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    atomic_write(
        path,
        &serde_json::to_vec_pretty(preferences).map_err(|e| e.to_string())?,
    )
}

/// Write and flush a sibling temporary file, then atomically replace the destination.
/// A failed write or rename never truncates the previous project.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let name = path
        .file_name()
        .ok_or("invalid_file_name")?
        .to_string_lossy();
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = None;
    for _ in 0..32 {
        let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(".{name}.{}.{id}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    let (temporary, mut file) = temporary.ok_or("temporary_file_unavailable")?;
    let written = file
        .write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string());
    drop(file);
    let result = written.and_then(|_| replace_file(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: both strings are NUL-terminated and remain alive for the call.
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> Result<(), String> {
    fs::rename(source, target).map_err(|e| e.to_string())
}
