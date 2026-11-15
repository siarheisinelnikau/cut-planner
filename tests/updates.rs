use cut_planner::{storage::Preferences, updates::*};
use std::{
    collections::HashMap,
    io::Write,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

struct FakeHttp {
    responses: HashMap<String, Vec<u8>>,
    calls: AtomicUsize,
}
impl Transport for FakeHttp {
    fn get(
        &self,
        url: &str,
        output: &mut dyn Write,
        limit: u64,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(u64),
    ) -> Result<(), UpdateError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if cancel.load(Ordering::Relaxed) {
            return Err(UpdateError::new("cancelled", "fixture cancellation"));
        }
        let bytes = self
            .responses
            .get(url)
            .ok_or_else(|| UpdateError::new("update_unpublished", "fixture 404"))?;
        if bytes.len() as u64 > limit {
            return Err(UpdateError::new("update_verification", "fixture limit"));
        }
        output.write_all(bytes).unwrap();
        progress(bytes.len() as u64);
        Ok(())
    }
}
fn directory() -> tempfile::TempDir {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-data");
    std::fs::create_dir_all(&path).unwrap();
    tempfile::tempdir_in(path).unwrap()
}
fn fake_exe(byte: u8) -> Vec<u8> {
    let mut bytes = vec![byte; 1024];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[60..64].copy_from_slice(&64_u32.to_le_bytes());
    bytes[64..68].copy_from_slice(b"PE\0\0");
    bytes[68..70].copy_from_slice(&0x8664_u16.to_le_bytes());
    bytes
}
fn release(bytes: &[u8]) -> Release {
    use sha2::{Digest, Sha256};
    Release {
        tag_name: "v0.2.0".into(),
        draft: false,
        prerelease: false,
        assets: vec![Asset {
            name: "CutPlanner-v0.2.0.exe".into(),
            browser_download_url: format!(
                "{REPOSITORY}/releases/download/v0.2.0/CutPlanner-v0.2.0.exe"
            ),
            size: bytes.len() as u64,
            digest: Some(format!("sha256:{:x}", Sha256::digest(bytes))),
        }],
    }
}

#[test]
fn versions_assets_release_channels_and_opt_out() {
    let bytes = fake_exe(5);
    assert!(select_release(release(&bytes), "0.1.0").unwrap().is_some());
    assert!(select_release(release(&bytes), "0.10.0").unwrap().is_none());
    let mut r = release(&bytes);
    r.prerelease = true;
    assert!(select_release(r, "0.1.0").unwrap().is_none());
    let mut r = release(&bytes);
    r.assets[0].browser_download_url = "https://example.test/other.exe".into();
    assert_eq!(
        select_release(r, "0.1.0").unwrap_err().key,
        "update_verification"
    );
    let mut r = release(&bytes);
    r.assets[0].digest = None;
    assert_eq!(
        select_release(r, "0.1.0").unwrap_err().key,
        "update_asset_missing"
    );
    let mut r = release(&bytes);
    r.assets[0].name = "CutPlanner-v0.2.0-arm64.exe".into();
    assert_eq!(
        select_release(r, "0.1.0").unwrap_err().key,
        "update_asset_missing"
    );
    assert!(should_check_at_startup(&Preferences::default()));
    assert!(!should_check_at_startup(&Preferences {
        startup_update_check: false,
        ..Preferences::default()
    }));
}

#[test]
fn missing_repository_is_an_error_not_an_up_to_date_result() {
    let http = FakeHttp {
        responses: HashMap::new(),
        calls: AtomicUsize::new(0),
    };
    assert_eq!(check(&http, "0.1.0").unwrap_err().key, "update_unpublished");
}

#[test]
fn verified_download_replacement_and_failed_restart_roll_back() {
    let dir = directory();
    let bytes = fake_exe(5);
    let available = select_release(release(&bytes), "0.1.0").unwrap().unwrap();
    let http = FakeHttp {
        responses: HashMap::from([(
            available.executable.browser_download_url.clone(),
            bytes.clone(),
        )]),
        calls: AtomicUsize::new(0),
    };
    let mut last = 0;
    let update = download(
        &http,
        &available,
        &dir.path().join("cache"),
        &AtomicBool::new(false),
        |n, total| {
            assert_eq!(total, 1024);
            last = n;
        },
    )
    .unwrap();
    assert_eq!(last, 1024);
    verify(&update).unwrap();
    let target = dir.path().join("CutPlanner-v0.1.0.exe");
    let old = fake_exe(8);
    std::fs::write(&target, &old).unwrap();
    let project = dir.path().join("keep.cutplan");
    std::fs::write(&project, "user data").unwrap();
    let ticket = prepare_ticket(&update, &target, Some(project.clone())).unwrap();
    let mut launches = 0;
    apply_files(&ticket, |_, reopen| {
        launches += 1;
        assert_eq!(reopen, Some(project.as_path()));
        Ok(())
    })
    .unwrap();
    assert_eq!(launches, 1);
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
    assert_eq!(std::fs::read(&ticket.backup).unwrap(), old);
    assert_eq!(std::fs::read_to_string(&project).unwrap(), "user data");
    std::fs::write(&target, &old).unwrap();
    let ticket = prepare_ticket(&update, &target, None).unwrap();
    let result = apply_files(&ticket, |_, _| Err("synthetic failed launch".into()));
    assert!(result.is_err());
    assert_eq!(std::fs::read(&target).unwrap(), old);
    assert_eq!(std::fs::read_to_string(&project).unwrap(), "user data");
}

#[test]
fn corrupt_truncated_and_cancelled_downloads_never_install() {
    let dir = directory();
    let bytes = fake_exe(5);
    let available = select_release(release(&bytes), "0.1.0").unwrap().unwrap();
    for bad in [fake_exe(7), vec![1; 9]] {
        let http = FakeHttp {
            responses: HashMap::from([(available.executable.browser_download_url.clone(), bad)]),
            calls: AtomicUsize::new(0),
        };
        assert_eq!(
            download(
                &http,
                &available,
                dir.path(),
                &AtomicBool::new(false),
                |_, _| {}
            )
            .unwrap_err()
            .key,
            "update_verification"
        );
    }
    let http = FakeHttp {
        responses: HashMap::from([(available.executable.browser_download_url.clone(), bytes)]),
        calls: AtomicUsize::new(0),
    };
    assert_eq!(
        download(
            &http,
            &available,
            dir.path(),
            &AtomicBool::new(true),
            |_, _| {}
        )
        .unwrap_err()
        .key,
        "cancelled"
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn matching_checksum_file_fallback_is_required_and_verified() {
    use sha2::{Digest, Sha256};
    let dir = directory();
    let bytes = fake_exe(3);
    let mut release = release(&bytes);
    release.assets[0].digest = None;
    let name = format!("{}.sha256", release.assets[0].name);
    let url = format!("{REPOSITORY}/releases/download/v0.2.0/{name}");
    release.assets.push(Asset {
        name,
        browser_download_url: url.clone(),
        size: 100,
        digest: None,
    });
    let available = select_release(release, "0.1.0").unwrap().unwrap();
    let checksum = format!(
        "{:x} *{}\n",
        Sha256::digest(&bytes),
        available.executable.name
    );
    let mut http = FakeHttp {
        responses: HashMap::from([
            (available.executable.browser_download_url.clone(), bytes),
            (url.clone(), checksum.as_bytes().to_vec()),
        ]),
        calls: AtomicUsize::new(0),
    };
    let update = download(
        &http,
        &available,
        dir.path(),
        &AtomicBool::new(false),
        |_, _| {},
    )
    .unwrap();
    verify(&update).unwrap();
    http.responses.insert(
        url,
        checksum
            .replace(&available.executable.name, "unrelated.exe")
            .into_bytes(),
    );
    assert_eq!(
        download(
            &http,
            &available,
            dir.path(),
            &AtomicBool::new(false),
            |_, _| {}
        )
        .unwrap_err()
        .key,
        "update_verification"
    );
}
