use crate::effects::{self, Particle, Rng};
use crate::engine::{
    self, AudioContainer, EngineEvent, JobResult, JobSpec, MediaMode, Progress, Stage, Tools,
    VideoContainer, VideoInfo, AUDIO_QUALITIES, VIDEO_QUALITIES,
};
use crate::ui;
use crate::util::{self, detect_platform, format_timestamp, parse_timestamp, Field, Platform};
use crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::{Position, Rect};
use ratatui::DefaultTerminal;
use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Url,
    Output,
    Mode,
    Container,
    Quality,
    Playlist,
    TrimIn,
    TrimOut,
    Download,
    History,
}

impl Focus {
    const ORDER: &[Focus] = &[
        Self::Url,
        Self::Mode,
        Self::Container,
        Self::Quality,
        Self::TrimIn,
        Self::TrimOut,
        Self::Download,
        Self::Playlist,
        Self::Output,
        Self::History,
    ];

    fn is_text(self) -> bool {
        matches!(self, Self::Url | Self::Output | Self::TrimIn | Self::TrimOut)
    }

    fn next(self) -> Self {
        let i = Self::ORDER.iter().position(|f| *f == self).unwrap_or(0);
        Self::ORDER[(i + 1) % Self::ORDER.len()]
    }

    fn prev(self) -> Self {
        let i = Self::ORDER.iter().position(|f| *f == self).unwrap_or(0);
        Self::ORDER[(i + Self::ORDER.len() - 1) % Self::ORDER.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrimHandle {
    In,
    Out,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrimDrag {
    pub handle: TrimHandle,
    pub x: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Boot,
    Idle,
    Probing,
    Ready,
    Extracting,
    Done,
    Failed,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Boot => "BOOT",
            Self::Idle => "IDLE",
            Self::Probing => "LOCKING",
            Self::Ready => "ARMED",
            Self::Extracting => "LIVE",
            Self::Done => "LOCK",
            Self::Failed => "FAULT",
        }
    }

    pub fn busy(self) -> bool {
        matches!(self, Self::Probing | Self::Extracting)
    }
}

#[derive(Clone, Debug)]
pub struct HistoryItem {
    pub title: String,
    pub path: Option<PathBuf>,
    pub platform: Platform,
    pub ok: bool,
}

#[derive(Default, Clone, Copy)]
pub struct Hits {
    pub url: Rect,
    pub output: Rect,
    pub mode: Rect,
    pub container: Rect,
    pub quality: Rect,
    pub playlist: Rect,
    pub timeline: Rect,
    pub trim_bar: Rect,
    pub history: Rect,
    pub trim_in: Rect,
    pub trim_out: Rect,
    pub download: Rect,
}

pub struct App {
    pub started: Instant,
    pub tick: u64,
    pub phase: Phase,
    pub focus: Focus,
    pub help: bool,
    pub url: Field,
    pub output: Field,
    pub trim_in: Field,
    pub trim_out: Field,
    pub mode: MediaMode,
    pub video_container: VideoContainer,
    pub audio_container: AudioContainer,
    pub video_quality: usize,
    pub audio_quality: usize,
    pub playlist: bool,
    pub history_cursor: usize,
    pub info: Option<VideoInfo>,
    pub tools: Tools,
    pub logs: VecDeque<String>,
    pub history: Vec<HistoryItem>,
    pub progress: Progress,
    pub speed_hist: Vec<u64>,
    pub last_path: Option<PathBuf>,
    pub last_error: Option<String>,
    pub particles: Vec<Particle>,
    pub hits: Hits,
    pub trim_drag: Option<TrimDrag>,
    pub rng: Rng,
    pub update_available: Option<String>,
    pub apply_update: bool,
    tx: Sender<EngineEvent>,
    rx: Receiver<EngineEvent>,
    kill: Option<Arc<AtomicBool>>,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let tools = Tools::detect();
        let output = default_output();
        let mut app = Self {
            started: Instant::now(),
            tick: 0,
            phase: Phase::Boot,
            focus: Focus::Url,
            help: false,
            url: Field::default(),
            output: Field::from_str(&output.to_string_lossy()),
            trim_in: Field::default(),
            trim_out: Field::default(),
            mode: MediaMode::Video,
            video_container: VideoContainer::Mp4,
            audio_container: AudioContainer::Mp3,
            video_quality: 0,
            audio_quality: 0,
            playlist: false,
            history_cursor: 0,
            info: None,
            tools,
            logs: VecDeque::new(),
            history: Vec::new(),
            progress: Progress {
                percent: 0.0,
                speed: String::new(),
                eta: String::new(),
                total: String::new(),
                speed_bps: 0,
                stage: Stage::Download,
                filename: String::new(),
            },
            speed_hist: Vec::new(),
            last_path: None,
            last_error: None,
            particles: Vec::new(),
            hits: Hits::default(),
            trim_drag: None,
            rng: Rng::new(0x4E495452415445),
            update_available: None,
            apply_update: false,
            tx,
            rx,
            kill: None,
        };
        app.log("NITRATE VIDEO EXTRACTION CONSOLE");
        app.log("UNIT / VT-01   CLASS: UNRESTRICTED");
        match &app.tools.ytdlp {
            Some((_, ver)) => app.log(format!("YT-DLP  {ver}")),
            None => app.log("YT-DLP  MISSING — install yt-dlp"),
        }
        match &app.tools.ffmpeg {
            Some((_, ver)) => app.log(format!("FFMPEG  {ver}")),
            None => app.log("FFMPEG  MISSING — merge/trim needs ffmpeg"),
        }
        app.log("PASTE URL  ENTER LOCK  F6 EXTRACT  ? HELP");
        app
    }

    pub fn start_update_check(&self) {
        crate::update::spawn_check(self.tx.clone());
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        if msg.is_empty() {
            return;
        }
        let clock = util::format_mission(self.started.elapsed());
        self.logs.push_back(format!("{clock}  {msg}"));
        while self.logs.len() > 160 {
            self.logs.pop_front();
        }
    }

    pub fn platform(&self) -> Platform {
        self.info
            .as_ref()
            .map(|i| i.platform)
            .unwrap_or_else(|| detect_platform(&self.url.text()))
    }


    pub fn duration(&self) -> Option<f64> {
        self.info.as_ref().and_then(|i| i.duration)
    }

    pub fn skip_boot(&mut self) {
        if self.phase == Phase::Boot {
            self.phase = Phase::Idle;
        }
    }

    pub fn cancel_job(&mut self) {
        if let Some(kill) = &self.kill {
            kill.store(true, Ordering::Relaxed);
        }
    }

    fn busy(&self) -> bool {
        self.phase.busy()
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if self.phase == Phase::Boot && self.started.elapsed() >= Duration::from_millis(2400) {
            self.phase = Phase::Idle;
        }
        let bounds = Rect {
            x: 0,
            y: 0,
            width: 240,
            height: 80,
        };
        effects::step_particles(&mut self.particles, bounds);
    }

    pub fn drain_engine(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                EngineEvent::Log(msg) => self.log(msg),
                EngineEvent::Progress(p) => {
                    self.progress = p;
                    if self.progress.speed_bps > 0 {
                        self.speed_hist.push(self.progress.speed_bps);
                        if self.speed_hist.len() > 96 {
                            self.speed_hist.remove(0);
                        }
                    }
                }
                EngineEvent::ProbeDone(result) => {
                    self.kill = None;
                    match result {
                        Ok(info) => {
                            self.log(format!(
                                "LOCKED  {}  {}",
                                info.platform.label(),
                                info.title
                            ));
                            self.info = Some(info);
                            self.video_quality = 0;
                            self.audio_quality = 0;
                            self.phase = Phase::Ready;
                        }
                        Err(e) => {
                            self.last_error = Some(e.clone());
                            self.log(e);
                            self.phase = Phase::Failed;
                        }
                    }
                }
                EngineEvent::JobDone(result) => {
                    self.kill = None;
                    match result {
                        Ok(JobResult { path, title }) => {
                            self.last_path = path.clone();
                            self.progress.percent = 100.0;
                            self.phase = Phase::Done;
                            self.log(match &path {
                                Some(p) => format!("WRITE  {}", p.display()),
                                None => "WRITE  COMPLETE".into(),
                            });
                            self.history.insert(
                                0,
                                HistoryItem {
                                    title,
                                    path,
                                    platform: self.platform(),
                                    ok: true,
                                },
                            );
                            if self.history.len() > 40 {
                                self.history.pop();
                            }
                            effects::spawn_sparks(
                                &mut self.particles,
                                &mut self.rng,
                                60.0,
                                18.0,
                                28,
                            );
                        }
                        Err(e) => {
                            self.last_error = Some(e.clone());
                            self.log(e);
                            self.phase = Phase::Failed;
                            self.history.insert(
                                0,
                                HistoryItem {
                                    title: self
                                        .info
                                        .as_ref()
                                        .map(|i| i.title.clone())
                                        .unwrap_or_else(|| self.url.text()),
                                    path: None,
                                    platform: self.platform(),
                                    ok: false,
                                },
                            );
                        }
                    }
                }
                EngineEvent::UpdateAvailable(ver) => {
                    self.update_available = Some(ver.clone());
                    self.log(format!("UPDATE  {ver}  U TO APPLY OR  nitrate update"));
                }
            }
        }
    }

    fn field_mut(&mut self) -> Option<&mut Field> {
        match self.focus {
            Focus::Url => Some(&mut self.url),
            Focus::Output => Some(&mut self.output),
            Focus::TrimIn => Some(&mut self.trim_in),
            Focus::TrimOut => Some(&mut self.trim_out),
            _ => None,
        }
    }

    pub fn handle_paste(&mut self, s: &str) {
        if self.help || self.phase == Phase::Boot {
            self.help = false;
            self.skip_boot();
        }
        let s = s.trim();
        let into_url = self.focus == Focus::Url || !self.focus.is_text();
        if self.focus.is_text() {
            if let Some(f) = self.field_mut() {
                f.insert_str(s);
            }
        } else {
            self.url.insert_str(s);
            self.focus = Focus::Url;
        }
        self.on_url_changed();
        if into_url && looks_like_url(&self.url.text()) {
            self.probe();
        }
    }

    fn on_url_changed(&mut self) {
        if self.phase == Phase::Failed || self.phase == Phase::Done {
            self.phase = Phase::Idle;
        }
        let p = detect_platform(&self.url.text());
        if p != Platform::Unknown {
            self.log(format!("TARGET  {}", p.label()));
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.phase == Phase::Boot {
            self.skip_boot();
            return false;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return self.handle_ctrl(key);
        }
        if self.help {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter
            ) {
                self.help = false;
            }
            return false;
        }
        match key.code {
            KeyCode::Char('q') if !self.focus.is_text() => {
                self.cancel_job();
                return true;
            }
            KeyCode::Esc => {
                if self.busy() {
                    self.cancel_job();
                    self.log("ABORT");
                } else {
                    self.help = false;
                }
            }
            KeyCode::Char('?') if !self.focus.is_text() => self.help = true,
            KeyCode::F(1) => self.help = !self.help,
            KeyCode::Tab => self.focus = self.focus.next(),
            KeyCode::BackTab => self.focus = self.focus.prev(),
            KeyCode::F(5) => self.probe(),
            KeyCode::F(6) => self.fire(),
            KeyCode::Enter => self.on_enter(),
            KeyCode::Char('p') if !self.focus.is_text() => self.probe(),
            KeyCode::Char('f') if !self.focus.is_text() => self.fire(),
            KeyCode::Char('m') if !self.focus.is_text() => self.toggle_mode(),
            KeyCode::Char('u') if !self.focus.is_text() => {
                if self.update_available.is_some() {
                    self.apply_update = true;
                    self.cancel_job();
                    return true;
                }
                self.log("UPDATE  CHECKING");
                crate::update::spawn_check(self.tx.clone());
            }
            KeyCode::Char('l') if !self.focus.is_text() => {
                self.playlist = !self.playlist;
                self.log(if self.playlist {
                    "PLAYLIST  ON"
                } else {
                    "PLAYLIST  OFF"
                });
            }
            KeyCode::Char('[') if !self.focus.is_text() => self.nudge_trim(-1.0),
            KeyCode::Char(']') if !self.focus.is_text() => self.nudge_trim(1.0),
            KeyCode::Char('{') => self.nudge_trim(-5.0),
            KeyCode::Char('}') => self.nudge_trim(5.0),
            KeyCode::Left => self.left(),
            KeyCode::Right => self.right(),
            KeyCode::Up => self.up(),
            KeyCode::Down => self.down(),
            KeyCode::Home => {
                if let Some(f) = self.field_mut() {
                    f.home();
                }
            }
            KeyCode::End => {
                if let Some(f) = self.field_mut() {
                    f.end();
                }
            }
            KeyCode::Backspace => {
                if let Some(f) = self.field_mut() {
                    f.backspace();
                }
            }
            KeyCode::Delete => {
                if let Some(f) = self.field_mut() {
                    f.delete();
                }
            }
            KeyCode::Char(c) if self.focus.is_text() => {
                if !c.is_control() {
                    if let Some(f) = self.field_mut() {
                        f.insert(c);
                    }
                    if self.focus == Focus::Url {
                        self.on_url_changed();
                    }
                }
            }
            _ => {}
        }
        false
    }

    fn handle_ctrl(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => {
                self.cancel_job();
                true
            }
            KeyCode::Char('u') => {
                if let Some(f) = self.field_mut() {
                    f.clear();
                }
                false
            }
            KeyCode::Char('a') => {
                if let Some(f) = self.field_mut() {
                    f.home();
                }
                false
            }
            KeyCode::Char('e') => {
                if let Some(f) = self.field_mut() {
                    f.end();
                }
                false
            }
            KeyCode::Char('w') => {
                if let Some(f) = self.field_mut() {
                    f.kill_word();
                }
                false
            }
            KeyCode::Char('p') => {
                self.probe();
                false
            }
            KeyCode::Char('s') | KeyCode::Char('f') => {
                self.fire();
                false
            }
            _ => false,
        }
    }

    fn on_enter(&mut self) {
        match self.focus {
            Focus::Url => self.probe(),
            Focus::History => self.recall_history(),
            Focus::Mode => self.toggle_mode(),
            Focus::Container => self.cycle_container(1),
            Focus::Quality => self.cycle_quality(1),
            Focus::Playlist => self.playlist = !self.playlist,
            Focus::Download => self.fire(),
            _ => self.fire(),
        }
    }

    fn recall_history(&mut self) {
        let item = self.history.get(self.history_cursor).cloned();
        if let Some(item) = item {
            if let Some(path) = &item.path {
                self.log(format!("RECALL  {}", path.display()));
            }
            self.url.set(&item.title);
        }
    }

    fn left(&mut self) {
        match self.focus {
            Focus::Url | Focus::Output | Focus::TrimIn | Focus::TrimOut => {
                if let Some(f) = self.field_mut() {
                    f.left();
                }
            }
            Focus::Mode => self.toggle_mode(),
            Focus::Container => self.cycle_container(-1),
            Focus::Quality => self.cycle_quality(-1),
            Focus::Playlist => self.playlist = !self.playlist,
            _ => {}
        }
    }

    fn right(&mut self) {
        match self.focus {
            Focus::Url | Focus::Output | Focus::TrimIn | Focus::TrimOut => {
                if let Some(f) = self.field_mut() {
                    f.right();
                }
            }
            Focus::Mode => self.toggle_mode(),
            Focus::Container => self.cycle_container(1),
            Focus::Quality => self.cycle_quality(1),
            Focus::Playlist => self.playlist = !self.playlist,
            _ => {}
        }
    }

    fn up(&mut self) {
        match self.focus {
            Focus::History => {
                if self.history_cursor > 0 {
                    self.history_cursor -= 1;
                }
            }
            _ => self.focus = self.focus.prev(),
        }
    }

    fn down(&mut self) {
        match self.focus {
            Focus::History => {
                if !self.history.is_empty() && self.history_cursor + 1 < self.history.len() {
                    self.history_cursor += 1;
                }
            }
            _ => self.focus = self.focus.next(),
        }
    }

    fn toggle_mode(&mut self) {
        self.mode = self.mode.toggle();
        self.log(format!("TYPE  {}", self.mode.label()));
    }

    fn cycle_container(&mut self, dir: i32) {
        match self.mode {
            MediaMode::Video => {
                self.video_container = if dir < 0 {
                    self.video_container.prev()
                } else {
                    self.video_container.next()
                };
                self.log(format!("FORMAT  {}", self.video_container.label()));
            }
            MediaMode::Audio => {
                self.audio_container = if dir < 0 {
                    self.audio_container.prev()
                } else {
                    self.audio_container.next()
                };
                self.log(format!("FORMAT  {}", self.audio_container.label()));
            }
        }
    }

    fn cycle_quality(&mut self, dir: i32) {
        match self.mode {
            MediaMode::Video => {
                let n = VIDEO_QUALITIES.len() as i32;
                self.video_quality =
                    (self.video_quality as i32 + dir).rem_euclid(n) as usize;
                self.log(format!(
                    "QUALITY  {}",
                    VIDEO_QUALITIES[self.video_quality].label
                ));
            }
            MediaMode::Audio => {
                let n = AUDIO_QUALITIES.len() as i32;
                self.audio_quality =
                    (self.audio_quality as i32 + dir).rem_euclid(n) as usize;
                self.log(format!(
                    "QUALITY  {}",
                    AUDIO_QUALITIES[self.audio_quality].label
                ));
            }
        }
    }

    pub fn trim_range(&self) -> Option<(f64, f64, f64)> {
        let dur = match self.duration().filter(|d| *d > 0.0) {
            Some(d) => d,
            None if !self.url.text().trim().is_empty() => {
                let inn = parse_timestamp(&self.trim_in.text())
                    .filter(|v| v.is_finite())
                    .unwrap_or(0.0);
                let out = parse_timestamp(&self.trim_out.text())
                    .filter(|v| v.is_finite())
                    .unwrap_or(0.0);
                inn.max(out).max(60.0)
            }
            None => return None,
        };
        let inn = parse_timestamp(&self.trim_in.text())
            .filter(|v| v.is_finite())
            .unwrap_or(0.0)
            .clamp(0.0, dur)
            .round();
        let mut out = parse_timestamp(&self.trim_out.text())
            .filter(|v| v.is_finite())
            .unwrap_or(dur)
            .clamp(0.0, dur)
            .round();
        if out < inn {
            out = inn;
        }
        Some((inn, out, dur.round().max(inn.max(out))))
    }

    fn nudge_trim(&mut self, delta: f64) {
        let Some((mut inn, mut out, dur)) = self.trim_range() else {
            return;
        };
        if self.focus == Focus::TrimOut {
            out = (out + delta).round().clamp((inn + 1.0).min(dur), dur);
            self.trim_out.set(&format_timestamp(out));
        } else {
            inn = (inn + delta).round().clamp(0.0, (out - 1.0).max(0.0));
            self.trim_in.set(&format_timestamp(inn));
        }
    }


    pub fn handle_mouse(&mut self, m: MouseEvent) {
        if self.phase == Phase::Boot {
            self.skip_boot();
            return;
        }
        if self.help {
            if matches!(m.kind, MouseEventKind::Down(_)) {
                self.help = false;
            }
            return;
        }
        match m.kind {
            MouseEventKind::Up(_) => {
                self.trim_drag = None;
                return;
            }
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {
                if self.trim_drag.is_some() {
                    self.drag_trim(m.column);
                }
                return;
            }
            _ => {}
        }
        let pos = Position::new(m.column, m.row);
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.trim_drag = None;
                if self.hits.url.contains(pos) {
                    self.focus = Focus::Url;
                } else if self.hits.output.contains(pos) {
                    self.focus = Focus::Output;
                } else if self.hits.mode.contains(pos) {
                    self.focus = Focus::Mode;
                    self.toggle_mode();
                } else if self.hits.container.contains(pos) {
                    self.focus = Focus::Container;
                    self.cycle_container(1);
                } else if self.hits.quality.contains(pos) {
                    self.focus = Focus::Quality;
                    self.cycle_quality(1);
                } else if self.hits.playlist.contains(pos) {
                    self.focus = Focus::Playlist;
                    self.playlist = !self.playlist;
                } else if self.hits.history.contains(pos) {
                    self.focus = Focus::History;
                    self.click_history(m.row);
                } else if self.hits.trim_in.contains(pos) {
                    self.focus = Focus::TrimIn;
                } else if self.hits.trim_out.contains(pos) {
                    self.focus = Focus::TrimOut;
                } else if self.hits.trim_bar.contains(pos) {
                    self.grab_trim(m.column);
                } else if self.hits.download.contains(pos) {
                    self.focus = Focus::Download;
                    self.fire();
                }
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if self.hits.container.contains(pos) {
                    self.cycle_container(-1);
                } else if self.hits.quality.contains(pos) {
                    self.cycle_quality(-1);
                } else if self.hits.trim_bar.contains(pos) {
                    self.click_timeline_out(m.column);
                }
            }
            MouseEventKind::ScrollUp => match self.focus {
                Focus::History => self.up(),
                Focus::Quality => self.cycle_quality(-1),
                Focus::Container => self.cycle_container(-1),
                Focus::TrimIn | Focus::TrimOut => self.nudge_trim(-1.0),
                _ => {}
            },
            MouseEventKind::ScrollDown => match self.focus {
                Focus::History => self.down(),
                Focus::Quality => self.cycle_quality(1),
                Focus::Container => self.cycle_container(1),
                Focus::TrimIn | Focus::TrimOut => self.nudge_trim(1.0),
                _ => {}
            }
            _ => {}
        }
    }



    fn click_history(&mut self, row: u16) {
        let inner_y = self.hits.history.y.saturating_add(1);
        if row <= inner_y {
            return;
        }
        let idx = (row - inner_y - 1) as usize;
        if idx < self.history.len() {
            self.history_cursor = idx;
        }
    }

    fn grab_trim(&mut self, col: u16) {
        let Some((inn, out, dur)) = self.trim_range() else {
            return;
        };
        let bar = self.hits.trim_bar;
        if bar.width == 0 {
            return;
        }
        let x = col.saturating_sub(bar.x).min(bar.width.saturating_sub(1));
        let in_x = util::trim_x(inn, dur, bar.width);
        let out_x = util::trim_x(out, dur, bar.width);
        let on_in = in_x == Some(x);
        let on_out = out_x == Some(x);
        let handle = if on_in && !on_out {
            TrimHandle::In
        } else if on_out && !on_in {
            TrimHandle::Out
        } else if on_in && on_out {
            if self.focus == Focus::TrimOut {
                TrimHandle::Out
            } else {
                TrimHandle::In
            }
        } else {
            let t = util::trim_t(x, dur, bar.width);
            if (t - inn).abs() <= (t - out).abs() {
                TrimHandle::In
            } else {
                TrimHandle::Out
            }
        };
        self.focus = match handle {
            TrimHandle::In => Focus::TrimIn,
            TrimHandle::Out => Focus::TrimOut,
        };
        let hold = match handle {
            TrimHandle::In => on_in,
            TrimHandle::Out => on_out,
        };
        self.trim_drag = Some(TrimDrag { handle, x });
        if !hold {
            self.apply_trim_drag_at(col);
        }
    }

    fn drag_trim(&mut self, col: u16) {
        if self.trim_drag.is_none() {
            return;
        }
        self.apply_trim_drag_at(col);
    }

    fn apply_trim_drag_at(&mut self, col: u16) {
        let Some(drag) = self.trim_drag else {
            return;
        };
        let Some((inn, out, dur)) = self.trim_range() else {
            return;
        };
        let bar = self.hits.trim_bar;
        if bar.width == 0 {
            return;
        }
        let x = col.saturating_sub(bar.x).min(bar.width.saturating_sub(1));
        let raw = util::trim_t(x, dur, bar.width);
        const GAP: f64 = 1.0;
        let t = match drag.handle {
            TrimHandle::In => raw.clamp(0.0, (out - GAP).max(0.0)),
            TrimHandle::Out => raw.clamp((inn + GAP).min(dur), dur),
        };
        let mut draw_x = x;
        match drag.handle {
            TrimHandle::In => {
                if let Some(ox) = util::trim_x(out, dur, bar.width) {
                    draw_x = draw_x.min(ox.saturating_sub(1));
                }
            }
            TrimHandle::Out => {
                if let Some(ix) = util::trim_x(inn, dur, bar.width) {
                    draw_x = draw_x.max(ix.saturating_add(1).min(bar.width.saturating_sub(1)));
                }
            }
        }
        if let Some(d) = &mut self.trim_drag {
            d.x = draw_x;
        }
        match drag.handle {
            TrimHandle::In => self.trim_in.set(&format_timestamp(t)),
            TrimHandle::Out => self.trim_out.set(&format_timestamp(t)),
        }
    }

    fn click_timeline_out(&mut self, col: u16) {
        self.trim_drag = None;
        let Some((inn, _, dur)) = self.trim_range() else {
            return;
        };
        let bar = self.hits.trim_bar;
        if bar.width == 0 {
            return;
        }
        let x = col.saturating_sub(bar.x).min(bar.width.saturating_sub(1));
        let t = util::trim_t(x, dur, bar.width).clamp((inn + 1.0).min(dur), dur);
        self.trim_out.set(&format_timestamp(t));
        self.focus = Focus::TrimOut;
    }

    pub fn probe(&mut self) {
        if self.busy() {
            return;
        }
        let url = self.url.text();
        let url = url.trim().to_string();
        if url.is_empty() {
            self.log("NO TARGET URL");
            return;
        }
        let Some((bin, _)) = self.tools.ytdlp.clone() else {
            self.log("YT-DLP MISSING");
            self.phase = Phase::Failed;
            return;
        };
        self.cancel_job();
        let kill = Arc::new(AtomicBool::new(false));
        self.kill = Some(kill.clone());
        self.phase = Phase::Probing;
        self.last_error = None;
        self.progress.percent = 0.0;
        self.log(format!("LOCKING  {url}"));
        let ffmpeg = self.tools.ffmpeg.as_ref().map(|(p, _)| p.clone());
        engine::spawn_probe(bin, ffmpeg, url, self.playlist, kill, self.tx.clone());
    }

    pub fn fire(&mut self) {
        if self.busy() {
            return;
        }
        let url = self.url.text();
        let url = url.trim().to_string();
        if url.is_empty() {
            self.log("NO TARGET URL");
            return;
        }
        let Some((bin, _)) = self.tools.ytdlp.clone() else {
            self.log("YT-DLP MISSING");
            self.phase = Phase::Failed;
            return;
        };
        if self.tools.ffmpeg.is_none() {
            self.log("WARN  FFMPEG MISSING — MERGE/TRIM MAY FAIL");
        }
        let spec = self.job_spec(url);
        self.cancel_job();
        let kill = Arc::new(AtomicBool::new(false));
        self.kill = Some(kill.clone());
        self.phase = Phase::Extracting;
        self.last_error = None;
        self.progress.percent = 0.0;
        self.progress.speed.clear();
        self.progress.eta.clear();
        self.speed_hist.clear();
        self.log("EXTRACT  START");
        let title = self
            .info
            .as_ref()
            .map(|i| i.title.clone())
            .unwrap_or_else(|| spec.url.clone());
        engine::spawn_download(bin, spec, title, kill, self.tx.clone());
    }

    fn job_spec(&self, url: String) -> JobSpec {
        JobSpec {
            url,
            mode: self.mode,
            video_container: self.video_container,
            audio_container: self.audio_container,
            video_quality: self.video_quality,
            audio_quality: self.audio_quality,
            exact_format: None,
            exact_has_audio: false,
            exact_has_video: true,
            trim_in: parse_timestamp(&self.trim_in.text()),
            trim_out: parse_timestamp(&self.trim_out.text()),
            duration: self.trim_range().map(|(_, _, dur)| dur).or_else(|| self.duration()),
            output_dir: PathBuf::from(self.output.text()),
            playlist: self.playlist,
            ffmpeg: self.tools.ffmpeg.as_ref().map(|(p, _)| p.clone()),
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.cancel_job();
    }
}

fn looks_like_url(s: &str) -> bool {
    let s = s.trim();
    s.contains("://") || detect_platform(s) != Platform::Unknown
}

fn default_output() -> PathBuf {
    dirs::video_dir()
        .or_else(dirs::download_dir)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nitrate")
}

pub fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;
        app.drain_engine();
        if crossterm::event::poll(Duration::from_millis(20))? {
            loop {
                match crossterm::event::read()? {
                    crossterm::event::Event::Key(key)
                        if key.kind == crossterm::event::KeyEventKind::Press
                            || key.kind == crossterm::event::KeyEventKind::Repeat =>
                    {
                        if app.handle_key(key) {
                            return Ok(());
                        }
                    }
                    crossterm::event::Event::Mouse(m) => app.handle_mouse(m),
                    crossterm::event::Event::Paste(s) => app.handle_paste(&s),
                    crossterm::event::Event::Resize(_, _) => {}
                    _ => {}
                }
                if !crossterm::event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
        app.tick();
        app.drain_engine();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn armed() -> App {
        let mut app = App::new();
        app.skip_boot();
        app.info = Some(VideoInfo {
            id: "id".into(),
            title: "t".into(),
            duration: Some(100.0),
            extractor: "Youtube".into(),
            webpage_url: "https://youtu.be/id".into(),
            platform: Platform::YouTube,
        });
        app.hits.trim_bar = Rect {
            x: 10,
            y: 5,
            width: 21,
            height: 3,
        };
        app.trim_in.set("0:10");
        app.trim_out.set("0:20");
        app
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn clicking_in_handle_does_not_jump() {
        let mut app = armed();
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 12, 6));
        assert_eq!(app.trim_in.text(), "0:10");
        assert_eq!(app.trim_out.text(), "0:20");
        let drag = app.trim_drag.expect("grab IN");
        assert_eq!(drag.handle, TrimHandle::In);
        assert_eq!(drag.x, 2);
    }

    #[test]
    fn clicking_out_handle_does_not_jump() {
        let mut app = armed();
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 14, 6));
        assert_eq!(app.trim_in.text(), "0:10");
        assert_eq!(app.trim_out.text(), "0:20");
        let drag = app.trim_drag.expect("grab OUT");
        assert_eq!(drag.handle, TrimHandle::Out);
        assert_eq!(drag.x, 4);
    }

    #[test]
    fn drag_follows_cursor_across_full_duration() {
        let mut app = armed();
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 12, 6));
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 11, 6));
        assert_eq!(app.trim_in.text(), "0:05");
        let drag = app.trim_drag.expect("dragging");
        assert_eq!(drag.handle, TrimHandle::In);
        assert_eq!(drag.x, 1);
        app.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 11, 6));
        assert!(app.trim_drag.is_none());
        assert_eq!(app.trim_in.text(), "0:05");
    }

    #[test]
    fn track_click_moves_nearest_handle_to_cursor() {
        let mut app = armed();
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 11, 6));
        assert_eq!(app.trim_in.text(), "0:05");
        assert_eq!(app.trim_out.text(), "0:20");
        let drag = app.trim_drag.expect("grab nearest");
        assert_eq!(drag.handle, TrimHandle::In);
        assert_eq!(drag.x, 1);
    }

    #[test]
    fn in_field_click_does_not_move_handles() {
        let mut app = armed();
        app.hits.trim_in = Rect {
            x: 0,
            y: 5,
            width: 8,
            height: 2,
        };
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 1, 6));
        assert_eq!(app.trim_in.text(), "0:10");
        assert_eq!(app.focus, Focus::TrimIn);
        assert!(app.trim_drag.is_none());
    }

    #[test]
    fn in_cannot_cross_out() {
        let mut app = armed();
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 12, 6));
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 30, 6));
        assert_eq!(app.trim_in.text(), "0:19");
        assert_eq!(app.trim_out.text(), "0:20");
        let drag = app.trim_drag.expect("clamped");
        assert_eq!(drag.x, 3);
    }

    #[test]
    fn long_video_maps_start_and_end_to_bar_edges() {
        let mut app = armed();
        if let Some(info) = app.info.as_mut() {
            info.duration = Some(3600.0);
        }
        app.trim_in.set("1:40");
        app.trim_out.set("10:00");
        app.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), 11, 6));
        assert_eq!(app.trim_in.text(), "1:40");
        let drag = app.trim_drag.expect("grab IN");
        assert_eq!(drag.handle, TrimHandle::In);
        assert_eq!(drag.x, 1);
        app.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), 12, 6));
        assert_eq!(app.trim_in.text(), "6:00");
        let drag = app.trim_drag.expect("dragging");
        assert_eq!(drag.x, 2);
    }
}
