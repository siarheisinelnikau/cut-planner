//! Public HTTPS checks and user-initiated, verified portable Windows updates.
//! The application never requires Codex, a GitHub login or a permanent updater service.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

pub const REPOSITORY: &str = "https://github.com/siarheisinelnikau/cut-planner";
pub const LATEST_API: &str =
    "https://api.github.com/repos/siarheisinelnikau/cut-planner/releases/latest";
pub const MAX_DOWNLOAD: u64 = 128 * 1024 * 1024;
static NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateError {
    pub key: &'static str,
    pub detail: String,
}
impl UpdateError {
    pub fn new(key: &'static str, detail: impl ToString) -> Self {
        Self {
            key,
            detail: detail.to_string(),
        }
    }
}
impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.key, self.detail)
    }
}
impl std::error::Error for UpdateError {}
fn io(e: impl ToString) -> UpdateError {
    UpdateError::new("update_failed", e)
}
fn verification(e: impl ToString) -> UpdateError {
    UpdateError::new("update_verification", e)
}

#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    #[serde(default)]
    pub digest: Option<String>,
}
#[derive(Clone, Debug)]
pub struct AvailableUpdate {
    pub version: String,
    pub executable: Asset,
    pub checksum: Option<Asset>,
    pub digest: Option<String>,
}

/// All networking is injectable; tests use in-memory release and download fixtures.
pub trait Transport {
    fn get(
        &self,
        url: &str,
        output: &mut dyn Write,
        limit: u64,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(u64),
    ) -> Result<(), UpdateError>;
}

pub struct HttpsTransport;
impl Transport for HttpsTransport {
    fn get(
        &self,
        url: &str,
        output: &mut dyn Write,
        limit: u64,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(u64),
    ) -> Result<(), UpdateError> {
        let timeout = if limit > 1024 * 1024 { 120 } else { 15 };
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .https_only(true)
            .timeout_connect(Some(Duration::from_secs(5)))
            .timeout_global(Some(Duration::from_secs(timeout)))
            .build()
            .into();
        let mut response = agent
            .get(url)
            .header(
                "User-Agent",
                concat!("CutPlanner/", env!("CARGO_PKG_VERSION")),
            )
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| match e {
                ureq::Error::StatusCode(404) => UpdateError::new("update_unpublished", "HTTP 404"),
                ureq::Error::StatusCode(403 | 429) => UpdateError::new("update_rate_limit", e),
                _ => io(e),
            })?;
        let mut reader = response.body_mut().as_reader();
        let mut buffer = [0_u8; 64 * 1024];
        let mut read = 0_u64;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(UpdateError::new("cancelled", "download cancelled"));
            }
            let count = reader.read(&mut buffer).map_err(io)?;
            if count == 0 {
                break;
            }
            read += count as u64;
            if read > limit {
                return Err(verification("response exceeds expected size"));
            }
            output.write_all(&buffer[..count]).map_err(io)?;
            progress(read);
        }
        Ok(())
    }
}

pub fn check(
    transport: &impl Transport,
    installed: &str,
) -> Result<Option<AvailableUpdate>, UpdateError> {
    let mut bytes = Vec::new();
    transport.get(
        LATEST_API,
        &mut bytes,
        1024 * 1024,
        &AtomicBool::new(false),
        &mut |_| {},
    )?;
    let release: Release = serde_json::from_slice(&bytes).map_err(io)?;
    select_release(release, installed)
}

pub fn should_check_at_startup(preferences: &crate::storage::Preferences) -> bool {
    preferences.startup_update_check
}

pub fn select_release(
    release: Release,
    installed: &str,
) -> Result<Option<AvailableUpdate>, UpdateError> {
    let version = semver::Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name),
    )
    .map_err(io)?;
    let installed = semver::Version::parse(installed).map_err(io)?;
    if release.draft || release.prerelease || !version.pre.is_empty() {
        return Ok(None);
    }
    if !version.cmp_precedence(&installed).is_gt() {
        return Ok(None);
    }
    if !cfg!(all(windows, target_arch = "x86_64")) {
        return Err(UpdateError::new(
            "update_asset_missing",
            "only Windows x64 is supported",
        ));
    }
    let single = format!("CutPlanner-v{version}.exe");
    let multi = format!("CutPlanner-v{version}-x64.exe");
    let executable = release
        .assets
        .iter()
        .find(|a| a.name == multi)
        .or_else(|| release.assets.iter().find(|a| a.name == single))
        .ok_or_else(|| UpdateError::new("update_asset_missing", "missing Windows x64 asset"))?
        .clone();
    validate_asset(&release.tag_name, &executable)?;
    if executable.size < 512 || executable.size > MAX_DOWNLOAD {
        return Err(verification("invalid executable size"));
    }
    let digest = executable
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .map(str::to_owned);
    if let Some(digest) = &digest {
        validate_hash(digest)?;
    }
    let checksum = release
        .assets
        .iter()
        .find(|a| a.name == format!("{}.sha256", executable.name))
        .cloned();
    if let Some(asset) = &checksum {
        validate_asset(&release.tag_name, asset)?;
    }
    if digest.is_none() && checksum.is_none() {
        return Err(UpdateError::new(
            "update_asset_missing",
            "SHA-256 digest or matching .sha256 asset required",
        ));
    }
    Ok(Some(AvailableUpdate {
        version: version.to_string(),
        executable,
        checksum,
        digest,
    }))
}

fn validate_asset(tag: &str, asset: &Asset) -> Result<(), UpdateError> {
    if asset.name.contains(['/', '\\', '\0'])
        || asset.browser_download_url
            != format!("{REPOSITORY}/releases/download/{tag}/{}", asset.name)
    {
        return Err(verification(
            "asset URL is not in the canonical repository release",
        ));
    }
    Ok(())
}
fn validate_hash(value: &str) -> Result<(), UpdateError> {
    if value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(verification("invalid SHA-256 value"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifiedUpdate {
    pub path: PathBuf,
    pub version: String,
    pub sha256: String,
    pub size: u64,
}

pub fn download(
    transport: &impl Transport,
    update: &AvailableUpdate,
    cache: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<VerifiedUpdate, UpdateError> {
    fs::create_dir_all(cache).map_err(io)?;
    let expected = if let Some(digest) = &update.digest {
        digest.clone()
    } else {
        let checksum = update
            .checksum
            .as_ref()
            .ok_or_else(|| verification("missing checksum"))?;
        let mut bytes = Vec::new();
        transport.get(
            &checksum.browser_download_url,
            &mut bytes,
            4096,
            cancel,
            &mut |_| {},
        )?;
        let text = std::str::from_utf8(&bytes).map_err(verification)?;
        let mut fields = text.split_whitespace();
        let digest = fields
            .next()
            .ok_or_else(|| verification("empty checksum"))?;
        validate_hash(digest)?;
        if fields.next().map(|n| n.trim_start_matches('*')) != Some(update.executable.name.as_str())
            || fields.next().is_some()
        {
            return Err(verification("checksum filename mismatch"));
        }
        digest.to_owned()
    };
    let id = NONCE.fetch_add(1, Ordering::Relaxed);
    let path = cache.join(format!(
        "CutPlanner-v{}-{}-{id}.staged.exe",
        update.version,
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(io)?;
    let result = transport
        .get(
            &update.executable.browser_download_url,
            &mut file,
            update.executable.size,
            cancel,
            &mut |n| progress(n, update.executable.size),
        )
        .and_then(|_| file.sync_all().map_err(io));
    drop(file);
    let verified = VerifiedUpdate {
        path,
        version: update.version.clone(),
        sha256: expected.to_ascii_lowercase(),
        size: update.executable.size,
    };
    let result = result.and_then(|_| verify(&verified));
    if let Err(e) = result {
        let _ = fs::remove_file(&verified.path);
        return Err(e);
    }
    Ok(verified)
}

pub fn file_digest(path: &Path) -> Result<String, UpdateError> {
    let mut file = File::open(path).map_err(io)?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(io)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn verify(update: &VerifiedUpdate) -> Result<(), UpdateError> {
    validate_hash(&update.sha256)?;
    if update.size < 512
        || update.size > MAX_DOWNLOAD
        || fs::metadata(&update.path).map_err(io)?.len() != update.size
    {
        return Err(verification("download length mismatch"));
    }
    if file_digest(&update.path)? != update.sha256.to_ascii_lowercase() {
        return Err(verification("SHA-256 mismatch"));
    }
    let mut file = File::open(&update.path).map_err(io)?;
    let mut dos = [0_u8; 64];
    file.read_exact(&mut dos).map_err(verification)?;
    if &dos[..2] != b"MZ" {
        return Err(verification("not a Windows executable"));
    }
    let pe_offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
    if pe_offset < 64 || pe_offset + 6 > update.size {
        return Err(verification("invalid PE header offset"));
    }
    file.seek(SeekFrom::Start(pe_offset)).map_err(io)?;
    let mut pe = [0_u8; 6];
    file.read_exact(&mut pe).map_err(verification)?;
    if &pe[..4] != b"PE\0\0" || u16::from_le_bytes([pe[4], pe[5]]) != 0x8664 {
        return Err(verification("not a Windows x64 executable"));
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallTicket {
    pub update: VerifiedUpdate,
    pub target: PathBuf,
    pub backup: PathBuf,
    pub pending: PathBuf,
    pub parent_pid: u32,
    pub reopen: Option<PathBuf>,
}

/// No process execution here: prepare a reviewable, recoverable transaction on disk.
pub fn prepare_ticket(
    update: &VerifiedUpdate,
    target: &Path,
    reopen: Option<PathBuf>,
) -> Result<InstallTicket, UpdateError> {
    verify(update)?;
    let target = target.canonicalize().map_err(io)?;
    let id = NONCE.fetch_add(1, Ordering::Relaxed);
    Ok(InstallTicket {
        update: update.clone(),
        backup: target.with_extension(format!("previous-{}-{id}.exe", std::process::id())),
        pending: target.with_extension(format!("pending-{}-{id}.exe", std::process::id())),
        target,
        parent_pid: std::process::id(),
        reopen,
    })
}

/// Replace only the executable. Project and preference paths are never changed.
/// Tests inject a launcher; the real helper calls this only after its parent exits.
pub fn apply_files(
    ticket: &InstallTicket,
    mut relaunch: impl FnMut(&Path, Option<&Path>) -> Result<(), String>,
) -> Result<(), UpdateError> {
    verify(&ticket.update)?;
    let parent = ticket
        .target
        .parent()
        .ok_or_else(|| io("missing executable directory"))?;
    if ticket.backup.parent() != Some(parent)
        || ticket.pending.parent() != Some(parent)
        || ticket.backup == ticket.target
        || ticket.pending == ticket.target
        || ticket.backup == ticket.pending
    {
        return Err(verification("invalid replacement paths"));
    }
    if ticket.backup.exists() || ticket.pending.exists() {
        return Err(io("replacement path already exists"));
    }
    let mut source = File::open(&ticket.update.path).map_err(io)?;
    let mut pending = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&ticket.pending)
        .map_err(io)?;
    let copied = std::io::copy(&mut source, &mut pending)
        .and_then(|_| pending.sync_all())
        .map_err(io);
    drop(pending);
    if let Err(error) = copied {
        let _ = fs::remove_file(&ticket.pending);
        return Err(error);
    }
    let pending_update = VerifiedUpdate {
        path: ticket.pending.clone(),
        ..ticket.update.clone()
    };
    if let Err(error) = verify(&pending_update) {
        let _ = fs::remove_file(&ticket.pending);
        return Err(error);
    }
    if let Err(error) = fs::rename(&ticket.target, &ticket.backup) {
        let _ = fs::remove_file(&ticket.pending);
        return Err(io(error));
    }
    if let Err(error) = fs::rename(&ticket.pending, &ticket.target) {
        let rollback = fs::rename(&ticket.backup, &ticket.target);
        return Err(io(format!(
            "replacement failed: {error}; rollback: {rollback:?}"
        )));
    }
    if let Err(error) = relaunch(&ticket.target, ticket.reopen.as_deref()) {
        let failed = fs::remove_file(&ticket.target);
        let rollback = if failed.is_ok() {
            fs::rename(&ticket.backup, &ticket.target)
        } else {
            failed
        };
        return Err(io(format!(
            "restart failed: {error}; rollback: {rollback:?}"
        )));
    }
    // Keep the previous binary for manual recovery; it is never user data.
    Ok(())
}

#[cfg(windows)]
pub fn launch_helper(ticket: &InstallTicket) -> Result<(), UpdateError> {
    use std::os::windows::process::CommandExt;
    let cache = ticket
        .update
        .path
        .parent()
        .ok_or_else(|| io("missing staging directory"))?;
    let id = NONCE.fetch_add(1, Ordering::Relaxed);
    let helper = cache.join(format!(
        "CutPlanner-updater-{}-{id}.exe",
        std::process::id()
    ));
    let manifest = cache.join(format!("install-{}-{id}.json", std::process::id()));
    if helper.exists() || manifest.exists() {
        return Err(io("an update transaction is already pending"));
    }
    fs::copy(std::env::current_exe().map_err(io)?, &helper).map_err(io)?;
    crate::storage::atomic_write(&manifest, &serde_json::to_vec_pretty(ticket).map_err(io)?)
        .map_err(io)?;
    std::process::Command::new(helper)
        .arg("--apply-update")
        .arg(&manifest)
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
        .spawn()
        .map_err(io)?;
    Ok(())
}

#[cfg(windows)]
pub fn run_helper(manifest: &Path) -> Result<(), UpdateError> {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };
    let ticket: InstallTicket =
        serde_json::from_str(&crate::storage::read_text(manifest).map_err(io)?).map_err(io)?;
    // SAFETY: request only wait access to this recorded parent; never terminate it.
    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, ticket.parent_pid) };
    if !handle.is_null() {
        let waited = unsafe { WaitForSingleObject(handle, 60_000) };
        unsafe {
            CloseHandle(handle);
        }
        if waited != WAIT_OBJECT_0 {
            return Err(io("the application did not exit within 60 seconds"));
        }
    }
    let result = apply_files(&ticket, |path, reopen| {
        let mut command = std::process::Command::new(path);
        if let Some(project) = reopen {
            command.arg("--project").arg(project);
        }
        command.spawn().map(|_| ()).map_err(|e| e.to_string())
    });
    if let Err(error) = &result {
        let error_path = manifest.with_extension("error.txt");
        let _ = crate::storage::atomic_write(&error_path, error.to_string().as_bytes());
        let mut command = std::process::Command::new(&ticket.target);
        command.arg("--update-error").arg(&error_path);
        if let Some(project) = &ticket.reopen {
            command.arg("--project").arg(project);
        }
        let _ = command.spawn();
    } else {
        let _ = fs::remove_file(manifest);
        let _ = fs::remove_file(&ticket.update.path);
    }
    result
}
