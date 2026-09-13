use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const YTDLP_TAG: &str = "2026.08.19";
const FFMPEG_TAG: &str = "n8.1.2-1";
const QJS_TAG: &str = "v0.16.2";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=NITRATE_SKIP_BUNDLE");
    println!("cargo:rerun-if-env-changed=NITRATE_BUNDLE_TOOLS");

    let profile = env::var("PROFILE").unwrap_or_default();
    let skip = env::var("NITRATE_SKIP_BUNDLE").as_deref() == Ok("1");
    let force = env::var("NITRATE_BUNDLE_TOOLS").as_deref() == Ok("1");
    if skip || (!force && profile != "release") {
        return;
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    let Some(files) = tool_files(&target_os, &target_arch, &target_env) else {
        println!("cargo:warning=no bundled yt-dlp/ffmpeg for {target_os}-{target_arch}-{target_env}");
        return;
    };

    let dest = PathBuf::from(env::var("OUT_DIR").unwrap())
        .ancestors()
        .nth(3)
        .expect("cargo OUT_DIR layout")
        .join("tools");
    fs::create_dir_all(&dest).unwrap();

    let cache = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("vendor")
        .join("tools")
        .join(format!("{target_os}-{target_arch}-{target_env}"));
    fs::create_dir_all(&cache).unwrap();

    for (name, url) in files {
        let cached = cache.join(&name);
        if !cached.is_file() || cached.metadata().map(|m| m.len()).unwrap_or(0) < 1024 {
            download(&url, &cached);
        }
        let dest_path = dest.join(&name);
        fs::copy(&cached, &dest_path).unwrap_or_else(|e| {
            panic!("copy {} -> {}: {e}", cached.display(), dest_path.display())
        });
        chmod_exec(&dest_path);
    }
}

fn tool_files(os: &str, arch: &str, env: &str) -> Option<Vec<(String, String)>> {
    let ytdlp_base = format!("https://github.com/yt-dlp/yt-dlp/releases/download/{YTDLP_TAG}");
    let ff_base = format!(
        "https://github.com/shaka-project/static-ffmpeg-binaries/releases/download/{FFMPEG_TAG}"
    );
    let qjs_base =
        format!("https://github.com/quickjs-ng/quickjs/releases/download/{QJS_TAG}");

    let (ytdlp_src, ytdlp_dst, ffmpeg_src, ffprobe_src, ffmpeg_dst, ffprobe_dst) =
        match (os, arch) {
            ("linux", "x86_64") => (
                if env == "musl" {
                    "yt-dlp_musllinux"
                } else {
                    "yt-dlp_linux"
                },
                "yt-dlp",
                "ffmpeg-linux-x64",
                "ffprobe-linux-x64",
                "ffmpeg",
                "ffprobe",
            ),
            ("linux", "aarch64") => (
                if env == "musl" {
                    "yt-dlp_musllinux_aarch64"
                } else {
                    "yt-dlp_linux_aarch64"
                },
                "yt-dlp",
                "ffmpeg-linux-arm64",
                "ffprobe-linux-arm64",
                "ffmpeg",
                "ffprobe",
            ),
            ("macos", "aarch64") => (
                "yt-dlp_macos",
                "yt-dlp",
                "ffmpeg-osx-arm64",
                "ffprobe-osx-arm64",
                "ffmpeg",
                "ffprobe",
            ),
            ("macos", "x86_64") => (
                "yt-dlp_macos",
                "yt-dlp",
                "ffmpeg-osx-x64",
                "ffprobe-osx-x64",
                "ffmpeg",
                "ffprobe",
            ),
            ("windows", "x86_64") => (
                "yt-dlp.exe",
                "yt-dlp.exe",
                "ffmpeg-win-x64.exe",
                "ffprobe-win-x64.exe",
                "ffmpeg.exe",
                "ffprobe.exe",
            ),
            _ => return None,
        };

    let (qjs_src, qjs_dst) = match (os, arch) {
        ("linux", "x86_64") => ("qjs-linux-x86_64", "qjs"),
        ("linux", "aarch64") => ("qjs-linux-aarch64", "qjs"),
        ("macos", "aarch64") => ("qjs-darwin-arm64", "qjs"),
        ("macos", "x86_64") => ("qjs-darwin-x86_64", "qjs"),
        ("windows", "x86_64") => ("qjs-windows-x86_64.exe", "qjs.exe"),
        _ => return None,
    };

    Some(vec![
        (
            ytdlp_dst.into(),
            format!("{ytdlp_base}/{ytdlp_src}"),
        ),
        (
            ffmpeg_dst.into(),
            format!("{ff_base}/{ffmpeg_src}"),
        ),
        (
            ffprobe_dst.into(),
            format!("{ff_base}/{ffprobe_src}"),
        ),
        (
            qjs_dst.into(),
            format!("{qjs_base}/{qjs_src}"),
        ),
    ])
}

fn download(url: &str, dest: &Path) {
    let tmp = dest.with_extension("part");
    let status = Command::new("curl")
        .args([
            "-fL",
            "--retry",
            "3",
            "--connect-timeout",
            "20",
            "-A",
            "nitrate-build",
            "-o",
        ])
        .arg(&tmp)
        .arg(url)
        .status()
        .unwrap_or_else(|e| panic!("curl {url}: {e}"));
    if !status.success() {
        let _ = fs::remove_file(&tmp);
        panic!("curl failed ({status}) for {url}");
    }
    let len = tmp.metadata().map(|m| m.len()).unwrap_or(0);
    if len < 1024 {
        let _ = fs::remove_file(&tmp);
        panic!("download too small ({len} bytes): {url}");
    }
    fs::rename(&tmp, dest).unwrap_or_else(|e| panic!("rename {}: {e}", tmp.display()));
}

fn chmod_exec(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}
