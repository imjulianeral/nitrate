use crate::util::{parse_yt_size, Platform};
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaMode {
    Video,
    Audio,
}

impl MediaMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Video => "VIDEO",
            Self::Audio => "AUDIO",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Video => Self::Audio,
            Self::Audio => Self::Video,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoContainer {
    Mp4,
    Mkv,
    Webm,
    Mov,
}

impl VideoContainer {
    pub const ALL: &[Self] = &[Self::Mp4, Self::Mkv, Self::Webm, Self::Mov];

    pub fn label(self) -> &'static str {
        match self {
            Self::Mp4 => "MP4",
            Self::Mkv => "MKV",
            Self::Webm => "WEBM",
            Self::Mov => "MOV",
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::Webm => "webm",
            Self::Mov => "mov",
        }
    }

    pub fn next(self) -> Self {
        cycle(Self::ALL, self, 1)
    }

    pub fn prev(self) -> Self {
        cycle(Self::ALL, self, -1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioContainer {
    Mp3,
    M4a,
    Opus,
    Flac,
    Wav,
    Ogg,
    Aac,
}

impl AudioContainer {
    pub const ALL: &[Self] = &[
        Self::Mp3,
        Self::M4a,
        Self::Opus,
        Self::Flac,
        Self::Wav,
        Self::Ogg,
        Self::Aac,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Mp3 => "MP3",
            Self::M4a => "M4A",
            Self::Opus => "OPUS",
            Self::Flac => "FLAC",
            Self::Wav => "WAV",
            Self::Ogg => "OGG",
            Self::Aac => "AAC",
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Opus => "opus",
            Self::Flac => "flac",
            Self::Wav => "wav",
            Self::Ogg => "ogg",
            Self::Aac => "aac",
        }
    }

    pub fn next(self) -> Self {
        cycle(Self::ALL, self, 1)
    }

    pub fn prev(self) -> Self {
        cycle(Self::ALL, self, -1)
    }
}


pub struct VideoQuality {
    pub label: &'static str,
    pub height: Option<u32>,
}

pub const VIDEO_QUALITIES: &[VideoQuality] = &[
    VideoQuality {
        label: "MAX",
        height: None,
    },
    VideoQuality {
        label: "2160P",
        height: Some(2160),
    },
    VideoQuality {
        label: "1440P",
        height: Some(1440),
    },
    VideoQuality {
        label: "1080P",
        height: Some(1080),
    },
    VideoQuality {
        label: "720P",
        height: Some(720),
    },
    VideoQuality {
        label: "480P",
        height: Some(480),
    },
    VideoQuality {
        label: "360P",
        height: Some(360),
    },
];

pub struct AudioQuality {
    pub label: &'static str,
    pub arg: &'static str,
}

pub const AUDIO_QUALITIES: &[AudioQuality] = &[
    AudioQuality {
        label: "BEST",
        arg: "0",
    },
    AudioQuality {
        label: "320K",
        arg: "320K",
    },
    AudioQuality {
        label: "256K",
        arg: "256K",
    },
    AudioQuality {
        label: "192K",
        arg: "192K",
    },
    AudioQuality {
        label: "128K",
        arg: "128K",
    },
    AudioQuality {
        label: "96K",
        arg: "96K",
    },
    AudioQuality {
        label: "64K",
        arg: "64K",
    },
];

fn cycle<T: Copy + PartialEq>(all: &[T], cur: T, dir: i32) -> T {
    let i = all.iter().position(|x| *x == cur).unwrap_or(0) as i32;
    let n = all.len() as i32;
    let j = (i + dir).rem_euclid(n) as usize;
    all[j]
}


#[derive(Clone, Debug)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub duration: Option<f64>,
    pub extractor: String,
    pub webpage_url: String,
    pub platform: Platform,
}

#[derive(Clone, Debug)]
pub struct Tools {
    pub ytdlp: Option<(PathBuf, String)>,
    pub ffmpeg: Option<(PathBuf, String)>,
}

impl Tools {
    pub fn detect() -> Self {
        Self {
            ytdlp: find_tool("yt-dlp", "YT_DLP", &["--version"]),
            ffmpeg: find_tool("ffmpeg", "FFMPEG", &["-version"]),
        }
    }
}

fn find_tool(name: &str, env_key: &str, ver_args: &[&str]) -> Option<(PathBuf, String)> {
    let path = std::env::var(env_key)
        .ok()
        .and_then(|p| existing_exe(PathBuf::from(p)))
        .or_else(|| which(name))?;
    let out = command(&path).args(ver_args).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let text = if text.trim().is_empty() {
        String::from_utf8_lossy(&out.stderr).into_owned()
    } else {
        text.into_owned()
    };
    let ver = text
        .lines()
        .next()
        .unwrap_or("OK")
        .trim()
        .chars()
        .take(48)
        .collect();
    Some((path, ver))
}

fn command(bin: &Path) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new(bin);
        cmd.creation_flags(0x0800_0000);
        cmd
    }
    #[cfg(not(windows))]
    {
        Command::new(bin)
    }
}

fn existing_exe(path: PathBuf) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path);
    }
    #[cfg(windows)]
    if path.extension().is_none() {
        for ext in ["exe", "cmd", "bat", "com"] {
            let candidate = path.with_extension(ext);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn tool_filenames(name: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        if name.contains('.') {
            return vec![name.to_string()];
        }
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".into());
        let mut names = vec![name.to_string()];
        for ext in pathext.split(';') {
            let ext = ext.trim();
            if ext.is_empty() {
                continue;
            }
            let ext = ext.strip_prefix('.').unwrap_or(ext);
            names.push(format!("{name}.{ext}"));
        }
        names
    }
    #[cfg(not(windows))]
    {
        vec![name.to_string()]
    }
}

fn which(name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    if let Ok(cwd) = std::env::current_dir() {
        dirs.insert(0, cwd);
    }
    #[cfg(unix)]
    {
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".local/bin"));
        }
        dirs.push(PathBuf::from("/usr/local/bin"));
        dirs.push(PathBuf::from("/usr/bin"));
    }
    #[cfg(windows)]
    {
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(r"scoop\shims"));
            dirs.push(home.join(r"AppData\Local\Microsoft\WinGet\Links"));
            dirs.push(home.join(r"AppData\Local\Microsoft\WindowsApps"));
            dirs.push(home.join(r"AppData\Local\Programs\yt-dlp"));
            dirs.push(home.join(r"AppData\Local\Programs\ffmpeg\bin"));
        }
        dirs.push(PathBuf::from(r"C:\ffmpeg\bin"));
        dirs.push(PathBuf::from(r"C:\Program Files\ffmpeg\bin"));
        dirs.push(PathBuf::from(r"C:\Program Files\yt-dlp"));
        dirs.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin"));
    }
    let names = tool_filenames(name);
    for dir in dirs {
        for n in &names {
            let p = dir.join(n);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}


fn js_runtime_args() -> Vec<String> {
    if which("deno").is_some() {
        return vec!["--js-runtimes".into(), "deno".into()];
    }
    if let Some(node) = which("node").or_else(|| which("nodejs")) {
        return vec![
            "--js-runtimes".into(),
            format!("node:{}", node.display()),
        ];
    }
    Vec::new()
}

#[derive(Clone, Debug)]
pub struct JobSpec {
    pub url: String,
    pub mode: MediaMode,
    pub video_container: VideoContainer,
    pub audio_container: AudioContainer,
    pub video_quality: usize,
    pub audio_quality: usize,
    pub exact_format: Option<String>,
    pub exact_has_audio: bool,
    pub exact_has_video: bool,
    pub trim_in: Option<f64>,
    pub trim_out: Option<f64>,
    pub duration: Option<f64>,
    pub output_dir: PathBuf,
    pub playlist: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Stage {
    Probe,
    Download,
    Merge,
    Convert,
    Trim,
    Finalize,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Probe => "LOCK",
            Self::Download => "LINK",
            Self::Merge => "MERGE",
            Self::Convert => "XCODE",
            Self::Trim => "TRIM",
            Self::Finalize => "WRITE",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Progress {
    pub percent: f64,
    pub speed: String,
    pub eta: String,
    pub total: String,
    pub speed_bps: u64,
    pub stage: Stage,
    pub filename: String,
}

#[derive(Clone, Debug)]
pub struct JobResult {
    pub path: Option<PathBuf>,
    pub title: String,
}

pub enum EngineEvent {
    Log(String),
    Progress(Progress),
    ProbeDone(Result<VideoInfo, String>),
    JobDone(Result<JobResult, String>),
    UpdateAvailable(String),
}

pub fn video_format_selector(height: Option<u32>, container: VideoContainer) -> String {
    let cap = height
        .map(|h| format!("[height<={h}]"))
        .unwrap_or_default();
    match container {
        VideoContainer::Mp4 | VideoContainer::Mov => format!(
            "bv*[ext=mp4]{cap}+ba[ext=m4a]/bv*{cap}+ba[ext=m4a]/bv*{cap}+ba/b{cap}/bv*+ba/b"
        ),
        VideoContainer::Webm => {
            format!("bv*[ext=webm]{cap}+ba/bv*{cap}+ba/b{cap}/bv*+ba/b")
        }
        VideoContainer::Mkv => format!("bv*{cap}+ba/b{cap}/bv*+ba/b"),
    }
}

fn video_mux_format(spec: &JobSpec) -> String {
    if let Some(id) = spec.exact_format.as_deref() {
        if spec.exact_has_video && spec.exact_has_audio {
            return id.to_string();
        }
        if spec.exact_has_video {
            return format!("{id}+bestaudio/bv*+ba/b");
        }
        if spec.exact_has_audio {
            return format!("bv*+{id}/bv*+ba/b");
        }
        return format!("{id}+bestaudio/bv*+ba/b");
    }
    let height = VIDEO_QUALITIES
        .get(spec.video_quality)
        .and_then(|q| q.height);
    video_format_selector(height, spec.video_container)
}

pub fn build_ytdlp_args(spec: &JobSpec) -> Vec<String> {
    let mut args = vec![
        "--color".into(),
        "never".into(),
        "--progress".into(),
        "--newline".into(),
        "--progress-template".into(),
        "download:NITRATE_PCT:%(progress.percent)s %(progress._speed_str)s %(progress._eta_str)s".into(),
    ];
    args.extend(js_runtime_args());
    if spec.playlist {
        args.push("--yes-playlist".into());
    } else {
        args.push("--no-playlist".into());
    }

    match spec.mode {
        MediaMode::Audio => {
            args.push("-f".into());
            if let Some(id) = spec.exact_format.as_deref() {
                args.push(id.to_string());
            } else {
                args.push("bestaudio/best".into());
            }
            args.push("-x".into());
            args.push("--audio-format".into());
            args.push(spec.audio_container.ext().into());
            let q = AUDIO_QUALITIES
                .get(spec.audio_quality)
                .unwrap_or(&AUDIO_QUALITIES[0]);
            args.push("--audio-quality".into());
            args.push(q.arg.into());
        }
        MediaMode::Video => {
            args.push("-f".into());
            args.push(video_mux_format(spec));
            args.push("--merge-output-format".into());
            args.push(spec.video_container.ext().into());
            if matches!(
                spec.video_container,
                VideoContainer::Mp4 | VideoContainer::Mov
            ) {
                args.push("--postprocessor-args".into());
                args.push("Merger:-c:v copy -c:a aac -movflags +faststart".into());
            }
        }
    }

    if spec.trim_in.is_some() || spec.trim_out.is_some() {
        let start = spec
            .trim_in
            .filter(|v| v.is_finite())
            .map(|v| format!("{v}"))
            .unwrap_or_else(|| "0".into());
        let end = match spec.trim_out {
            Some(v) if v.is_finite() => format!("{v}"),
            _ => "inf".into(),
        };
        args.push("--download-sections".into());
        args.push(format!("*{start}-{end}"));
        args.push("--force-keyframes-at-cuts".into());
    }

    let template = spec
        .output_dir
        .join("%(title).180B [%(id)s].%(ext)s")
        .to_string_lossy()
        .into_owned();
    args.push("-o".into());
    args.push(template);
    args.push("--no-mtime".into());
    args.push("--restrict-filenames".into());
    args.push("--print".into());
    args.push("after_move:NITRATE_OUT:%(filepath)s".into());
    args.push(spec.url.clone());
    args
}

pub fn spawn_probe(
    ytdlp: PathBuf,
    url: String,
    playlist: bool,
    kill: Arc<AtomicBool>,
    tx: Sender<EngineEvent>,
) {
    thread::Builder::new()
        .name("nitrate-probe".into())
        .spawn(move || {
            let mut args = vec![
                "--print".into(),
                "%(.{id,title,duration,extractor_key,webpage_url})j".into(),
                "--color".into(),
                "never".into(),
                "--no-download".into(),
                "--skip-download".into(),
            ];
            args.extend(js_runtime_args());
            if playlist {
                args.push("--yes-playlist".into());
            } else {
                args.push("--no-playlist".into());
            }

            args.push(url.clone());

            let mut stdout = String::new();
            let mut last_err = String::new();
            let status = match run_streaming(&ytdlp, &args, &kill, |pipe, line| match pipe {
                Pipe::Out => {
                    stdout.push_str(line);
                    stdout.push('\n');
                }
                Pipe::Err => {
                    last_err = line.to_string();
                    let _ = tx.send(EngineEvent::Log(line.to_string()));
                }
            }) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(EngineEvent::ProbeDone(Err(e)));
                    return;
                }
            };

            if kill.load(Ordering::Relaxed) {
                let _ = tx.send(EngineEvent::ProbeDone(Err("ABORTED".into())));
                return;
            }
            if !status.success() {
                let msg = if last_err.is_empty() {
                    format!("PROBE FAILED (status {status})")
                } else {
                    last_err
                };
                let _ = tx.send(EngineEvent::ProbeDone(Err(msg)));
                return;
            }
            match parse_info(&stdout, &url) {
                Ok(info) => {
                    let _ = tx.send(EngineEvent::ProbeDone(Ok(info)));
                }
                Err(e) => {
                    let _ = tx.send(EngineEvent::ProbeDone(Err(e)));
                }
            }
        })
        .ok();
}

pub fn spawn_download(
    ytdlp: PathBuf,
    spec: JobSpec,
    title_hint: String,
    kill: Arc<AtomicBool>,
    tx: Sender<EngineEvent>,
) {
    thread::Builder::new()
        .name("nitrate-dl".into())
        .spawn(move || {
            if let Err(e) = std::fs::create_dir_all(&spec.output_dir) {
                let _ = tx.send(EngineEvent::JobDone(Err(format!("OUTPUT DIR: {e}"))));
                return;
            }
            let args = build_ytdlp_args(&spec);
            if let Some(sel) = args
                .windows(2)
                .find(|w| w[0] == "-f")
                .map(|w| w[1].clone())
            {
                let _ = tx.send(EngineEvent::Log(format!("SEL {sel}")));
            }
            let mut dest = None;
            let mut last_err = String::new();
            let mut progress = Progress {
                percent: 0.0,
                speed: String::new(),
                eta: String::new(),
                total: String::new(),
                speed_bps: 0,
                stage: Stage::Download,
                filename: String::new(),
            };
            let mut parts = if spec.mode == MediaMode::Video { 2 } else { 1 };
            let mut part = 0usize;
            let mut last_raw = 0.0f64;

            let status = match run_streaming(&ytdlp, &args, &kill, |_, line| {
                if let Some(rest) = line.split("format(s):").nth(1) {
                    let n = rest.split('+').filter(|s| !s.trim().is_empty()).count();
                    if n > 0 {
                        parts = n;
                    }
                }
                match interpret_line(line) {
                    LineKind::Progress(p) => {
                        if p.percent + 12.0 < last_raw {
                            part = (part + 1).min(parts.saturating_sub(1));
                        }
                        last_raw = p.percent;
                        let overall = (part as f64 * 100.0 + p.percent) / parts.max(1) as f64;
                        progress.percent = overall.clamp(0.0, 99.9);
                        if !p.speed.is_empty() {
                            progress.speed = p.speed;
                        }
                        if !p.eta.is_empty() {
                            progress.eta = p.eta;
                        }
                        if !p.total.is_empty() {
                            progress.total = p.total;
                        }
                        progress.speed_bps = p.speed_bps;
                        progress.stage = Stage::Download;
                        let _ = tx.send(EngineEvent::Progress(progress.clone()));
                    }
                    LineKind::MediaTime(sec) => {
                        if let Some(dur) = spec.duration.filter(|d| *d > 0.2) {
                            progress.percent = (sec / dur * 100.0).clamp(0.0, 99.9);
                            progress.stage = Stage::Download;
                            let _ = tx.send(EngineEvent::Progress(progress.clone()));
                        }
                    }
                    LineKind::Destination(path) => {
                        progress.filename = path.clone();
                        dest = Some(PathBuf::from(&path));
                        let _ = tx.send(EngineEvent::Log(format!("DEST {path}")));
                    }
                    LineKind::OutputPath(path) => {
                        dest = Some(PathBuf::from(&path));
                        progress.filename = path;
                    }
                    LineKind::Stage(stage) => {
                        progress.stage = stage;
                        if stage == Stage::Merge {
                            progress.percent = progress.percent.max(90.0);
                        }
                        let _ = tx.send(EngineEvent::Progress(progress.clone()));
                        let _ = tx.send(EngineEvent::Log(stage.label().into()));
                    }
                    LineKind::Error(msg) => {
                        last_err = msg.clone();
                        let _ = tx.send(EngineEvent::Log(msg));
                    }
                    LineKind::Log(msg) => {
                        if let Some(rest) = msg.split("format(s):").nth(1) {
                            let n = rest
                                .split('+')
                                .filter(|s| !s.trim().is_empty())
                                .count();
                            if n > 0 {
                                parts = n;
                            }
                        }
                        let _ = tx.send(EngineEvent::Log(msg));
                    }
                }
            }) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(EngineEvent::JobDone(Err(e)));
                    return;
                }
            };

            if kill.load(Ordering::Relaxed) {
                let _ = tx.send(EngineEvent::JobDone(Err("ABORTED".into())));
                return;
            }
            if !status.success() {
                let msg = if last_err.is_empty() {
                    format!("EXTRACT FAILED (status {status})")
                } else {
                    last_err
                };
                let _ = tx.send(EngineEvent::JobDone(Err(msg)));
                return;
            }
            let _ = tx.send(EngineEvent::JobDone(Ok(JobResult {
                path: dest,
                title: title_hint,
            })));
        })
        .ok();
}

#[derive(Clone, Copy)]
enum Pipe {
    Out,
    Err,
}

fn run_streaming(
    bin: &Path,
    args: &[String],
    kill: &AtomicBool,
    mut on_line: impl FnMut(Pipe, &str),
) -> Result<ExitStatus, String> {
    let mut child = command(bin)
        .args(args)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("SPAWN {bin:?}: {e}"))?;
    pump(&mut child, kill, &mut on_line)
}

fn pump(
    child: &mut Child,
    kill: &AtomicBool,
    on_line: &mut impl FnMut(Pipe, &str),
) -> Result<ExitStatus, String> {
    let stdout = child.stdout.take().ok_or("NO STDOUT")?;
    let stderr = child.stderr.take().ok_or("NO STDERR")?;
    let (tx, rx) = mpsc::channel::<(Pipe, String)>();
    spawn_pipe_reader(stdout, tx.clone(), Pipe::Out);
    spawn_pipe_reader(stderr, tx, Pipe::Err);

    loop {
        if kill.load(Ordering::Relaxed) {
            kill_child(child);
        }
        while let Ok((pipe, line)) = rx.try_recv() {
            on_line(pipe, &line);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                thread::sleep(Duration::from_millis(80));
                while let Ok((pipe, line)) = rx.try_recv() {
                    on_line(pipe, &line);
                }
                return Ok(status);
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(format!("WAIT: {e}")),
        }
    }
}

fn spawn_pipe_reader<R: Read + Send + 'static>(mut reader: R, tx: Sender<(Pipe, String)>, pipe: Pipe) {
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        let mut acc = Vec::new();
        loop {
            let n = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            for &b in &buf[..n] {
                if b == b'\n' || b == b'\r' {
                    if !acc.is_empty() {
                        let line = String::from_utf8_lossy(&acc).into_owned();
                        acc.clear();
                        if tx.send((pipe, line)).is_err() {
                            return;
                        }
                    }
                } else if acc.len() < 32 * 1024 * 1024 {
                    acc.push(b);
                }
            }
        }
        if !acc.is_empty() {
            let _ = tx.send((pipe, String::from_utf8_lossy(&acc).into_owned()));
        }
    });
}

fn kill_child(child: &mut Child) {
    #[cfg(windows)]
    {
        let pid = child.id();
        let _ = command(Path::new("taskkill"))
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}

#[derive(Debug)]
pub enum LineKind {
    Progress(Progress),
    MediaTime(f64),
    Destination(String),
    OutputPath(String),
    Stage(Stage),
    Error(String),
    Log(String),
}

pub fn interpret_line(line: &str) -> LineKind {
    let cleaned = strip_ansi(line);
    let t = cleaned.trim();
    if t.is_empty() {
        return LineKind::Log(String::new());
    }
    if let Some(path) = t.strip_prefix("NITRATE_OUT:") {
        return LineKind::OutputPath(path.trim().to_string());
    }
    if let Some(rest) = t.strip_prefix("NITRATE_PCT:") {
        if let Some(p) = parse_template_progress(rest) {
            return LineKind::Progress(p);
        }
    }
    if let Some(rest) = t.strip_prefix("[download]") {
        let rest = rest.trim();
        if let Some(dest) = rest.strip_prefix("Destination:") {
            return LineKind::Destination(dest.trim().to_string());
        }
        if rest.contains('%') {
            if let Some(p) = parse_percent_progress(rest) {
                return LineKind::Progress(p);
            }
        }
        return LineKind::Log(t.to_string());
    }
    if t.starts_with("[Merger]") {
        if let Some(path) = quoted_path(t) {
            return LineKind::Destination(path);
        }
        return LineKind::Stage(Stage::Merge);
    }
    if t.starts_with("[ExtractAudio]") {
        if let Some(dest) = t.split_once("Destination:").map(|(_, d)| d.trim()) {
            return LineKind::Destination(dest.to_string());
        }
        return LineKind::Stage(Stage::Convert);
    }
    if t.starts_with("[VideoConvertor]") || t.starts_with("[VideoRemuxer]") {
        return LineKind::Stage(Stage::Convert);
    }
    if t.starts_with("[Fixup") {
        return LineKind::Stage(Stage::Finalize);
    }
    if t.starts_with("ERROR:") || t.starts_with("error:") {
        return LineKind::Error(t.to_string());
    }
    if t.starts_with("[youtube]") || t.starts_with("[info]") || t.contains("Solving JS") {
        return LineKind::Stage(Stage::Probe);
    }
    if let Some(sec) = parse_ffmpeg_clock(t) {
        return LineKind::MediaTime(sec);
    }
    if t.contains('%') && (t.contains("ETA") || t.contains(" eta ") || t.contains(" of ")) {
        if let Some(p) = parse_percent_progress(t) {
            return LineKind::Progress(p);
        }
    }
    LineKind::Log(t.to_string())
}

fn parse_template_progress(rest: &str) -> Option<Progress> {
    let mut toks = rest.split_whitespace();
    let percent: f64 = toks.next()?.trim_end_matches('%').parse().ok()?;
    if !(0.0..=100.0).contains(&percent) {
        return None;
    }
    let speed = toks
        .next()
        .filter(|s| *s != "NA" && *s != "Unknown")
        .unwrap_or("")
        .to_string();
    let eta = toks
        .next()
        .filter(|s| *s != "NA" && *s != "Unknown")
        .unwrap_or("")
        .to_string();
    let speed_bps = parse_yt_size(&speed).unwrap_or(0);
    Some(Progress {
        percent,
        speed,
        eta,
        total: String::new(),
        speed_bps,
        stage: Stage::Download,
        filename: String::new(),
    })
}

fn parse_ffmpeg_clock(s: &str) -> Option<f64> {
    let rest = s.split("time=").nth(1)?;
    let tok = rest.split_whitespace().next()?;
    if tok == "N/A" || tok.starts_with('-') {
        return None;
    }
    let mut parts = tok.split(':');
    let a: f64 = parts.next()?.parse().ok()?;
    match (parts.next(), parts.next()) {
        (Some(b), Some(c)) => {
            let b: f64 = b.parse().ok()?;
            let c: f64 = c.parse().ok()?;
            Some(a * 3600.0 + b * 60.0 + c)
        }
        (Some(b), None) => {
            let b: f64 = b.parse().ok()?;
            Some(a * 60.0 + b)
        }
        _ => Some(a),
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        if c != '\u{08}' {
            out.push(c);
        }
    }
    out
}

fn quoted_path(s: &str) -> Option<String> {
    let start = s.find('"')?;
    let rest = &s[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_percent_progress(rest: &str) -> Option<Progress> {
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let percent = tokens.iter().find_map(|tok| {
        let tok = tok.trim_start_matches('~');
        let num = tok.strip_suffix('%')?;
        let p: f64 = num.parse().ok()?;
        (0.0..=100.0).contains(&p).then_some(p)
    })?;
    let mut total = String::new();
    let mut speed = String::new();
    let mut eta = String::new();
    let mut iter = tokens.iter().copied();
    while let Some(tok) = iter.next() {
        match tok {
            "of" => {
                if let Some(v) = iter.next() {
                    let v = if v == "~" {
                        iter.next().unwrap_or(v)
                    } else {
                        v
                    };
                    total = v.trim_start_matches('~').to_string();
                }
            }
            "at" => {
                if let Some(v) = iter.next() {
                    if v != "Unknown" {
                        speed = v.to_string();
                    }
                }
            }
            "ETA" | "eta" => {
                if let Some(v) = iter.next() {
                    if v != "Unknown" {
                        eta = v.to_string();
                    }
                }
            }
            _ => {}
        }
    }
    let speed_bps = parse_yt_size(&speed).unwrap_or(0);
    Some(Progress {
        percent: percent.clamp(0.0, 100.0),
        speed,
        eta,
        total,
        speed_bps,
        stage: Stage::Download,
        filename: String::new(),
    })
}

pub fn parse_info(json: &str, fallback_url: &str) -> Result<VideoInfo, String> {
    let json = json.trim();
    let start = json.find('{').ok_or("NO JSON FROM YT-DLP")?;
    let v: Value = serde_json::from_str(&json[start..]).map_err(|e| format!("JSON: {e}"))?;
    if v.get("entries").and_then(|e| e.as_array()).is_some() {
        if let Some(first) = v
            .get("entries")
            .and_then(|e| e.as_array())
            .and_then(|a| a.iter().find(|x| x.is_object()))
        {
            return parse_info(&first.to_string(), fallback_url);
        }
    }
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .unwrap_or("UNTITLED")
        .to_string();
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or("-")
        .to_string();
    let duration = v.get("duration").and_then(|x| {
        x.as_f64()
            .or_else(|| x.as_u64().map(|n| n as f64))
            .or_else(|| x.as_i64().map(|n| n as f64))
    });
    let extractor = v
        .get("extractor_key")
        .or_else(|| v.get("extractor"))
        .and_then(|x| x.as_str())
        .unwrap_or("generic")
        .to_string();
    let webpage_url = v
        .get("webpage_url")
        .and_then(|x| x.as_str())
        .unwrap_or(fallback_url)
        .to_string();
    let platform = platform_from_extractor(&extractor, &webpage_url);
    Ok(VideoInfo {
        id,
        title,
        duration,
        extractor,
        webpage_url,
        platform,
    })
}

fn platform_from_extractor(extractor: &str, url: &str) -> Platform {
    let e = extractor.to_ascii_lowercase();
    if e.contains("youtube") {
        Platform::YouTube
    } else if e.contains("twitter") || e == "x" {
        Platform::X
    } else if e.contains("facebook") {
        Platform::Facebook
    } else {
        crate::util::detect_platform(url)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_unknown_tool_is_none() {
        assert!(which("nitrate-no-such-tool-9f3a2c").is_none());
    }

    #[test]
    fn tool_filenames_include_bare_name() {
        let names = tool_filenames("ffmpeg");
        assert!(names.iter().any(|n| n.eq_ignore_ascii_case("ffmpeg")));
        #[cfg(windows)]
        assert!(names.iter().any(|n| n.to_ascii_lowercase().ends_with(".exe")));
        #[cfg(not(windows))]
        assert_eq!(names, vec!["ffmpeg".to_string()]);
    }

    #[test]
    fn selector_caps_height() {
        let max = video_format_selector(None, VideoContainer::Mp4);
        assert!(max.contains("+ba"), "{max}");
        assert!(max.contains("m4a"), "{max}");
        let capped = video_format_selector(Some(1080), VideoContainer::Mp4);
        assert!(capped.contains("height<=1080"), "{capped}");
        assert!(capped.contains("+ba"), "{capped}");
    }

    #[test]
    fn args_video_trim() {
        let spec = JobSpec {
            url: "https://youtu.be/x".into(),
            mode: MediaMode::Video,
            video_container: VideoContainer::Mp4,
            audio_container: AudioContainer::Mp3,
            video_quality: 3,
            audio_quality: 0,
            exact_format: None,
            exact_has_audio: false,
            exact_has_video: true,
            trim_in: Some(12.0),
            trim_out: Some(44.0),
            duration: Some(32.0),
            output_dir: PathBuf::from("/tmp/nitrate"),
            playlist: false,
        };
        let args = build_ytdlp_args(&spec);
        assert!(args.contains(&"--no-playlist".into()));
        assert!(!args.contains(&"--cookies-from-browser".into()));
        assert!(args.contains(&"--download-sections".into()));
        assert!(args.iter().any(|a| a == "*12-44"));
        assert!(args.contains(&"--merge-output-format".into()));
        assert!(args.contains(&"mp4".into()));
        assert!(!args.contains(&"--remux-video".into()));
        let sel = args
            .windows(2)
            .find(|w| w[0] == "-f")
            .map(|w| w[1].as_str())
            .unwrap();
        assert!(sel.contains("height<=1080"), "{sel}");
        assert!(sel.contains("+ba"), "{sel}");
        assert!(sel.contains("m4a"), "{sel}");
        assert!(args.iter().any(|a| a.contains("-c:a aac")));
    }

    #[test]
    fn exact_video_only_always_muxes_audio() {
        let spec = JobSpec {
            url: "https://youtu.be/x".into(),
            mode: MediaMode::Video,
            video_container: VideoContainer::Mp4,
            audio_container: AudioContainer::Mp3,
            video_quality: 3,
            audio_quality: 0,
            exact_format: Some("137".into()),
            exact_has_audio: false,
            exact_has_video: true,
            trim_in: None,
            trim_out: None,
            duration: None,
            output_dir: PathBuf::from("/tmp/nitrate"),
            playlist: false,
        };
        let sel = video_mux_format(&spec);
        assert_eq!(sel, "137+bestaudio/bv*+ba/b");
        assert!(!sel.ends_with("/137"));
    }

    #[test]
    fn args_audio_exact() {
        let spec = JobSpec {
            url: "https://x.com/a/status/1".into(),
            mode: MediaMode::Audio,
            video_container: VideoContainer::Mp4,
            audio_container: AudioContainer::Opus,
            video_quality: 0,
            audio_quality: 2,
            exact_format: Some("140".into()),
            exact_has_audio: true,
            exact_has_video: false,
            trim_in: None,
            trim_out: None,
            duration: None,
            output_dir: PathBuf::from("/tmp/out"),
            playlist: true,
        };
        let args = build_ytdlp_args(&spec);
        assert!(args.contains(&"--yes-playlist".into()));
        assert!(args.contains(&"-x".into()));
        assert!(args.contains(&"opus".into()));
        assert!(args.contains(&"256K".into()));
        assert!(args.contains(&"140".into()));
    }

    #[test]
    fn progress_line() {
        match interpret_line("[download]  45.2% of  54.21MiB at  2.31MiB/s ETA 00:12") {
            LineKind::Progress(p) => {
                assert!((p.percent - 45.2).abs() < f64::EPSILON);
                assert_eq!(p.total, "54.21MiB");
                assert_eq!(p.speed, "2.31MiB/s");
                assert_eq!(p.eta, "00:12");
                assert!(p.speed_bps > 1_000_000);
            }
            other => panic!("{other:?}"),
        }
        match interpret_line("[download]  12.3% of ~  50.00MiB at  Unknown B/s ETA Unknown (frag 3/40)") {
            LineKind::Progress(p) => {
                assert!((p.percent - 12.3).abs() < f64::EPSILON);
                assert_eq!(p.total, "50.00MiB");
                assert!(p.speed.is_empty());
                assert!(p.eta.is_empty());
            }
            other => panic!("{other:?}"),
        }
        match interpret_line("[download] 100% of 10.00MiB in 00:05 at 2.31MiB/s") {
            LineKind::Progress(p) => assert!((p.percent - 100.0).abs() < f64::EPSILON),
            other => panic!("{other:?}"),
        }
        match interpret_line("\u{1b}[0;32m[download]\u{1b}[0m  8.0% of  1.00MiB at  100.00KiB/s ETA 00:09") {
            LineKind::Progress(p) => assert!((p.percent - 8.0).abs() < f64::EPSILON),
            other => panic!("{other:?}"),
        }
        match interpret_line("[download] Destination: /tmp/a.mp4") {
            LineKind::Destination(p) => assert_eq!(p, "/tmp/a.mp4"),
            other => panic!("{other:?}"),
        }
        match interpret_line("NITRATE_OUT:/data/clip.mkv") {
            LineKind::OutputPath(p) => assert_eq!(p, "/data/clip.mkv"),
            other => panic!("{other:?}"),
        }
        match interpret_line("NITRATE_PCT:45.2 2.31MiB/s 00:12") {
            LineKind::Progress(p) => {
                assert!((p.percent - 45.2).abs() < f64::EPSILON);
                assert_eq!(p.speed, "2.31MiB/s");
            }
            other => panic!("{other:?}"),
        }
        match interpret_line("frame=  360 fps=30 q=-1.0 size=200KiB time=00:00:12.00 bitrate=136.8kbits/s speed=1.2x") {
            LineKind::MediaTime(sec) => assert!((sec - 12.0).abs() < 0.01),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_info_minimal() {
        let json = r#"{"id":"abc","title":"Hello","duration":90,"extractor_key":"Youtube","webpage_url":"https://youtu.be/abc"}"#;
        let info = parse_info(json, "https://youtu.be/abc").unwrap();
        assert_eq!(info.title, "Hello");
        assert_eq!(info.duration, Some(90.0));
        assert_eq!(info.platform, Platform::YouTube);
    }
}
