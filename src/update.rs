use crate::engine::EngineEvent;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;
use std::thread;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn github_repo() -> &'static str {
    option_env!("NITRATE_GITHUB_REPO").unwrap_or("imjulianeral/nitrate")
}

pub fn asset_name() -> Option<String> {
    let os = host_os()?;
    let arch = host_arch()?;
    let ext = if os == "windows" { ".exe" } else { "" };
    Some(format!("nitrate-{os}-{arch}{ext}"))
}

pub fn host_os() -> Option<&'static str> {
    if cfg!(target_os = "linux") {
        Some("linux")
    } else if cfg!(target_os = "macos") {
        Some("macos")
    } else if cfg!(target_os = "windows") {
        Some("windows")
    } else {
        None
    }
}

pub fn host_arch() -> Option<&'static str> {
    if cfg!(target_arch = "x86_64") {
        Some("x64")
    } else if cfg!(target_arch = "aarch64") {
        Some("arm64")
    } else {
        None
    }
}

pub fn version_newer(latest: &str, current: &str) -> bool {
    match (parse_ver(latest), parse_ver(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

pub fn parse_tag(json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(json).ok()?;
    let tag = v.get("tag_name")?.as_str()?;
    let tag = tag.trim().trim_start_matches('v');
    if tag.is_empty() {
        None
    } else {
        Some(tag.to_string())
    }
}

pub fn spawn_check(tx: Sender<EngineEvent>) {
    thread::Builder::new()
        .name("nitrate-update".into())
        .spawn(move || {
            if let Ok(latest) = fetch_latest_version() {
                if version_newer(&latest, VERSION) {
                    let _ = tx.send(EngineEvent::UpdateAvailable(latest));
                }
            }
        })
        .ok();
}

pub fn run_cli_update() -> Result<String, String> {
    let latest = fetch_latest_version()?;
    if !version_newer(&latest, VERSION) {
        return Ok(format!("NITRATE  {VERSION}  already current"));
    }
    let dest = apply_latest()?;
    Ok(format!(
        "NITRATE  {VERSION} -> {latest}\nWRITE  {}",
        dest.display()
    ))
}

fn fetch_latest_version() -> Result<String, String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        github_repo()
    );
    let body = http_get(&url)?;
    parse_tag(&body).ok_or_else(|| "NO RELEASE TAG".into())
}

fn apply_latest() -> Result<PathBuf, String> {
    let asset = asset_name().ok_or("UNSUPPORTED PLATFORM")?;
    let url = format!(
        "https://github.com/{}/releases/latest/download/{asset}",
        github_repo()
    );
    let tmp = temp_download_path(&asset)?;
    http_download(&url, &tmp)?;
    let exe = std::env::current_exe().map_err(|e| format!("EXE: {e}"))?;
    replace_exe(&tmp, &exe)?;
    let _ = fs::remove_file(&tmp);
    Ok(exe)
}

fn temp_download_path(asset: &str) -> Result<PathBuf, String> {
    let mut p = std::env::temp_dir();
    p.push(format!("nitrate-update-{}-{asset}", std::process::id()));
    Ok(p)
}

fn replace_exe(src: &Path, dest: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(src, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("CHMOD: {e}"))?;
        let _ = fs::remove_file(dest);
        fs::rename(src, dest).map_err(|e| format!("REPLACE: {e}"))?;
    }
    #[cfg(windows)]
    {
        let bak = dest.with_extension("old");
        let _ = fs::remove_file(&bak);
        if dest.exists() {
            fs::rename(dest, &bak).map_err(|e| format!("RENAME: {e}"))?;
        }
        fs::rename(src, dest).map_err(|e| format!("REPLACE: {e}"))?;
    }
    Ok(())
}

fn http_get(url: &str) -> Result<String, String> {
    let out = curl()
        .args([
            "-fsSL",
            "--retry",
            "2",
            "--connect-timeout",
            "8",
            "-A",
            "nitrate",
            url,
        ])
        .output()
        .map_err(|e| format!("CURL: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("HTTP  {}", err.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn http_download(url: &str, dest: &Path) -> Result<(), String> {
    let status = curl()
        .args([
            "-fsSL",
            "--retry",
            "2",
            "--connect-timeout",
            "8",
            "-A",
            "nitrate",
            "-o",
        ])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|e| format!("CURL: {e}"))?;
    if !status.success() {
        return Err("DOWNLOAD FAILED".into());
    }
    if !dest.is_file() {
        return Err("DOWNLOAD EMPTY".into());
    }
    Ok(())
}

fn curl() -> Command {
    Command::new("curl")
}

fn parse_ver(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches('v');
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(version_newer("1.2.0", "1.0.0"));
        assert!(version_newer("v1.0.1", "1.0.0"));
        assert!(!version_newer("1.0.0", "1.0.0"));
        assert!(!version_newer("1.0.0", "2.0.0"));
        assert!(version_newer("2.0.0", "1.9.9"));
    }

    #[test]
    fn parse_github_tag() {
        let json = r#"{"tag_name":"v1.4.2","name":"1.4.2"}"#;
        assert_eq!(parse_tag(json).as_deref(), Some("1.4.2"));
        assert_eq!(parse_tag("{}"), None);
    }

    #[test]
    fn asset_name_matches_host() {
        let name = asset_name().expect("supported host");
        assert!(name.starts_with("nitrate-"));
        if cfg!(windows) {
            assert!(name.ends_with(".exe"));
        } else {
            assert!(!name.ends_with(".exe"));
        }
    }

    #[test]
    fn github_repo_is_set() {
        assert_eq!(github_repo(), "imjulianeral/nitrate");
    }
}
