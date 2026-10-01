use crate::app::{App, Focus, Phase, TrimHandle};
use crate::effects::{self, Meter};
use crate::engine::{MediaMode, Stage, AUDIO_QUALITIES, VIDEO_QUALITIES};
use crate::theme;
use crate::util::{self, format_timestamp, Field};
use crate::vhs::{self, LabelText};
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::time::Duration;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    frame.render_widget(Block::new().style(theme::root()), area);
    if app.phase == Phase::Boot {
        draw_boot(frame, area, app);
        return;
    }
    if area.width < 80 || area.height < 22 {
        draw_tiny(frame, area);
        return;
    }

    let roomy = area.height >= 32;
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(10),
        Constraint::Length(if roomy { 5 } else { 3 }),
        Constraint::Length(if roomy { 5 } else { 4 }),
        Constraint::Length(1),
    ])
    .split(area);

    draw_header(frame, rows[0], app);
    draw_body(frame, rows[1], app);
    draw_log(frame, rows[2], app);
    draw_tape(frame, rows[3], app);
    draw_keys(frame, rows[4], app);

    if app.help {
        draw_help(frame, area);
    }
}

fn draw_tiny(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("NITRATE", theme::accent_bold())),
        Line::from(Span::styled(
            format!("screen {}×{} — needs 80×22", area.width, area.height),
            theme::dim(),
        )),
    ];
    let y = area.y + area.height.saturating_sub(2) / 2;
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        Rect { y, height: area.height.min(2), ..area },
    );
}

fn panel(title: &str, focused: bool) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(theme::border(focused))
        .title(Span::styled(format!(" {title} "), theme::title(focused)))
        .style(theme::root())
}

fn blink(app: &App, period: u64) -> bool {
    (app.tick / period).is_multiple_of(2)
}

// ── header ──────────────────────────────────────────────────────────────

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let row = Rect { height: 1, ..area };
    let mut left = vec![
        Span::raw(" "),
        Span::styled("NITRATE", theme::bold()),
        Span::raw("  "),
    ];
    for c in theme::STRIPES {
        left.push(Span::styled("━", Style::new().fg(c).bg(theme::BG)));
    }
    left.push(Span::raw("  "));
    left.push(Span::styled("video cassette recorder", theme::ghost()));
    frame.render_widget(Paragraph::new(Line::from(left)), row);

    let lamp = |on: bool| {
        Span::styled(
            "●",
            Style::new().fg(if on { theme::OK } else { theme::REC }).bg(theme::BG),
        )
    };
    let mut right = Vec::new();
    if let Some(ver) = &app.update_available {
        right.push(Span::styled(format!("update {ver} ready  "), theme::accent()));
    }
    right.extend([
        Span::styled("yt-dlp ", theme::ghost()),
        lamp(app.tools.ytdlp.is_some()),
        Span::styled("   ffmpeg ", theme::ghost()),
        lamp(app.tools.ffmpeg.is_some()),
        Span::styled(format!("   rev {} ", crate::update::VERSION), theme::ghost()),
    ]);
    frame.render_widget(Paragraph::new(Line::from(right)).right_aligned(), row);
}

// ── body ────────────────────────────────────────────────────────────────

fn draw_body(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.width >= 104 {
        let cols = Layout::horizontal([
            Constraint::Length(30),
            Constraint::Min(40),
            Constraint::Length(30),
        ])
        .split(area);
        draw_left(frame, cols[0], app);
        draw_center(frame, cols[1], app);
        draw_setup(frame, cols[2], app);
    } else {
        let cols = Layout::horizontal([Constraint::Min(40), Constraint::Length(30)]).split(area);
        app.hits.history = Rect::default();
        draw_center(frame, cols[0], app);
        draw_setup(frame, cols[1], app);
    }
}

fn draw_left(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = Layout::vertical([
        Constraint::Length(8),
        Constraint::Length(4),
        Constraint::Min(3),
    ])
    .split(area);

    let block = panel("LABEL", false);
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    let val_w = inner.width.saturating_sub(8) as usize;
    let url = app.url.text();
    let plat = app.platform();
    let info = app.info.as_ref();
    let title = match (app.phase, info) {
        (Phase::Probing, _) => "reading tape…",
        (_, Some(i)) => i.title.as_str(),
        _ => "—",
    };
    let link = info
        .map(|i| i.webpage_url.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(url.as_str());
    let dur = app
        .duration()
        .map(format_timestamp)
        .unwrap_or_else(|| "—".into());
    let row = |key: &'static str, value: Span<'static>| {
        Line::from(vec![Span::styled(format!(" {key:<7}"), theme::ghost()), value])
    };
    let lines = vec![
        row(
            "source",
            if url.trim().is_empty() && info.is_none() {
                Span::styled("—", theme::root())
            } else {
                Span::styled(
                    plat.label(),
                    Style::new().fg(theme::platform(plat)).bg(theme::BG),
                )
            },
        ),
        row(
            "id",
            Span::styled(info.map(|i| i.id.clone()).unwrap_or("—".into()), theme::root()),
        ),
        row("length", Span::styled(dur, theme::root())),
        row(
            "title",
            Span::styled(util::marquee(title, val_w, app.tick), theme::bold()),
        ),
        row(
            "site",
            Span::styled(
                info.map(|i| i.extractor.to_ascii_lowercase())
                    .unwrap_or("—".into()),
                theme::dim(),
            ),
        ),
        row(
            "link",
            Span::styled(util::marquee(link, val_w, app.tick), theme::ghost()),
        ),
    ];
    frame.render_widget(Paragraph::new(lines), inner);

    let block = panel("TOOLS", false);
    let inner = block.inner(rows[1]);
    frame.render_widget(block, rows[1]);
    let version = |tool: &Option<(std::path::PathBuf, String)>| match tool {
        Some((_, v)) => Span::styled(
            util::truncate(v, inner.width.saturating_sub(9) as usize),
            theme::dim(),
        ),
        None => Span::styled("missing", theme::rec()),
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" yt-dlp  ", theme::ghost()),
                version(&app.tools.ytdlp),
            ]),
            Line::from(vec![
                Span::styled(" ffmpeg  ", theme::ghost()),
                version(&app.tools.ffmpeg),
            ]),
        ]),
        inner,
    );

    draw_library(frame, rows[2], app);
}

/// Finished recordings, shelved as cassette spines.
fn draw_library(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.focus == Focus::History;
    let block = panel("LIBRARY", focused);
    app.hits.history = area;
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let count = match app.history.len() {
        0 => "shelf is empty".to_string(),
        1 => "1 tape".to_string(),
        n => format!("{n} tapes"),
    };
    let mut lines = vec![Line::from(Span::styled(format!(" {count}"), theme::ghost()))];
    let h = inner.height.saturating_sub(1) as usize;
    let start = app.history_cursor.saturating_sub(h.saturating_sub(1));
    let text_w = inner.width.saturating_sub(5) as usize;
    for (i, item) in app.history.iter().enumerate().skip(start).take(h) {
        let selected = focused && i == app.history_cursor;
        let bg = if selected { theme::BG_SELECT } else { theme::BG };
        let spine = if item.ok { theme::platform(item.platform) } else { theme::REC };
        let fg = match (selected, item.ok) {
            (true, _) => theme::AMBER,
            (_, false) => theme::FG_GHOST,
            _ => theme::FG,
        };
        let title = util::truncate(&item.title, text_w);
        let pad = text_w.saturating_sub(unicode_width::UnicodeWidthStr::width(title.as_str()));
        lines.push(Line::from(vec![
            Span::styled(" ▐", Style::new().fg(spine).bg(theme::BG)),
            Span::styled(" ", Style::new().bg(bg)),
            Span::styled(title, Style::new().fg(fg).bg(bg)),
            Span::styled(" ".repeat(pad + 1), Style::new().bg(bg)),
            Span::styled(
                if item.ok { " " } else { "✕" },
                Style::new().fg(theme::REC).bg(bg),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_center(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(4),
        Constraint::Length(5),
    ])
    .split(area);

    app.hits.url = rows[0];
    let url_focus = app.focus == Focus::Url;
    let block = panel("URL", url_focus);
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    draw_field(frame, inner, &app.url, url_focus, "paste a video link");

    draw_deck(frame, rows[1], app);

    app.hits.timeline = rows[2];
    draw_timeline(frame, rows[2], app);
}

// ── the deck ────────────────────────────────────────────────────────────

/// What the post-processing step is doing, in plain words.
fn stage_detail(stage: Stage) -> &'static str {
    match stage {
        Stage::Merge => "joining video and audio",
        Stage::Convert => "converting",
        Stage::Trim => "cutting the clip",
        Stage::Finalize => "finalizing the file",
        Stage::Probe | Stage::Download => "",
    }
}

fn clock_secs(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

fn stage_word(stage: Stage) -> &'static str {
    match stage {
        Stage::Probe => "TRACKING",
        Stage::Download => "RECORDING",
        Stage::Merge => "SPLICING",
        Stage::Convert => "DUBBING",
        Stage::Trim => "EDITING",
        Stage::Finalize => "FINISHING",
    }
}

fn draw_deck(frame: &mut Frame, area: Rect, app: &App) {
    let live = app.phase == Phase::Extracting;
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(if live {
            Style::new().fg(theme::REC_DEEP).bg(theme::BG)
        } else {
            theme::border(false)
        })
        .title(Span::styled(" DECK ", theme::title(false)))
        .style(Style::new().fg(theme::FG).bg(theme::BG_DECK));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height < 2 {
        return;
    }
    let osd = Rect { height: 1, ..inner };
    let bay = Rect {
        y: inner.y + 1,
        height: inner.height - 1,
        ..inner
    };

    if app.deck.seated <= 0.0 && app.url.is_empty() {
        vhs::render_slot(frame.buffer_mut(), bay, app.tick);
    } else {
        let info = app.info.as_ref();
        let url = app.url.text();
        let title = match (app.phase, info) {
            (_, Some(i)) => i.title.clone(),
            (Phase::Probing, None) => "reading tape…".into(),
            _ => host(&url).unwrap_or_else(|| "untitled tape".into()),
        };
        let mut detail = vec![app.platform().label().to_string()];
        if let Some(d) = app.duration() {
            detail.push(format_timestamp(d));
        }
        detail.push(format!("{} {}", quality_label(app), container_label(app)));
        let detail = detail.join(" · ");
        let brand = format!("NITRATE {}", if app.mode == MediaMode::Audio { "HiFi" } else { "HQ" });
        vhs::render_cassette(
            frame.buffer_mut(),
            bay,
            &app.deck,
            &LabelText {
                title: &title,
                detail: &detail,
                brand: &brand,
            },
            app.tick,
        );
    }
    if app.phase == Phase::Probing {
        effects::render_tracking(frame.buffer_mut(), bay, app.tick);
    }
    draw_osd(frame, osd, app);
}

fn host(url: &str) -> Option<String> {
    let rest = url.trim().split("://").nth(1).unwrap_or(url.trim());
    let host = rest.split(['/', '?', '#']).next()?.trim_start_matches("www.");
    (!host.is_empty()).then(|| host.to_string())
}

/// On-screen display, drawn like a VCR's overlay text.
fn draw_osd(frame: &mut Frame, area: Rect, app: &App) {
    let bg = theme::BG_DECK;
    let text = |s: String, fg: Color| {
        Span::styled(s, Style::new().fg(fg).bg(bg).add_modifier(Modifier::BOLD))
    };
    let (glyph, glyph_fg, word) = match app.phase {
        Phase::Extracting => (
            if blink(app, 25) { "●" } else { " " },
            theme::REC,
            match (app.progress.stage, app.progress.post) {
                (Stage::Download | Stage::Probe, _) => "REC".to_string(),
                (stage, Some(p)) => format!("REC  {} {p:.0}%", stage_word(stage)),
                (stage, None) => format!("REC  {}", stage_word(stage)),
            },
        ),
        Phase::Probing => (
            "◀◀",
            if blink(app, 12) { theme::WHITE } else { theme::FG_GHOST },
            "TRACKING".into(),
        ),
        Phase::Ready => ("▌▌", theme::WHITE, "READY".into()),
        Phase::Done => ("■", theme::OK, "SAVED".into()),
        Phase::Failed => ("■", theme::REC, "ERROR".into()),
        _ if app.url.is_empty() => ("▲", theme::FG_DIM, "EJECT".into()),
        _ => ("■", theme::WHITE, "STOP".into()),
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::new().bg(bg)),
            text(glyph.into(), glyph_fg),
            Span::styled(" ", Style::new().bg(bg)),
            text(word, theme::WHITE),
        ]))
        .style(Style::new().bg(bg)),
        area,
    );

    let span = app
        .trim_range()
        .map(|(inn, out, _)| out - inn)
        .or_else(|| app.duration())
        .unwrap_or(0.0);
    let counter = (app.deck.pos * span).max(0.0) as u64;
    let clock = format!(
        "{}:{:02}:{:02}",
        counter / 3600,
        (counter % 3600) / 60,
        counter % 60
    );
    let speed = if app.mode == MediaMode::Audio { "HiFi" } else { "SP" };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            text(speed.into(), theme::FG_DIM),
            Span::styled("  ", Style::new().bg(bg)),
            text(clock, theme::VFD),
            Span::styled(" ", Style::new().bg(bg)),
        ]))
        .right_aligned(),
        area,
    );
}

fn quality_label(app: &App) -> &'static str {
    match app.mode {
        MediaMode::Video => VIDEO_QUALITIES[app.video_quality].label,
        MediaMode::Audio => AUDIO_QUALITIES[app.audio_quality].label,
    }
}

fn container_label(app: &App) -> &'static str {
    match app.mode {
        MediaMode::Video => app.video_container.label(),
        MediaMode::Audio => app.audio_container.label(),
    }
}

// ── trim ────────────────────────────────────────────────────────────────

fn draw_timeline(frame: &mut Frame, area: Rect, app: &mut App) {
    let dragging = app.trim_drag.is_some();
    let block = panel("TRIM", matches!(app.focus, Focus::TrimIn | Focus::TrimOut) || dragging);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let cols = Layout::horizontal([
        Constraint::Length(12),
        Constraint::Min(10),
        Constraint::Length(12),
    ])
    .spacing(1)
    .split(inner);

    app.hits.trim_in = draw_time_box(
        frame,
        cols[0],
        "IN",
        &app.trim_in,
        app.focus == Focus::TrimIn,
        "0:00",
    );
    app.hits.trim_out = draw_time_box(
        frame,
        cols[2],
        "OUT",
        &app.trim_out,
        app.focus == Focus::TrimOut,
        "end",
    );
    app.hits.trim_bar = cols[1];

    let bar = Rect {
        x: cols[1].x,
        y: cols[1].y.saturating_add(1),
        width: cols[1].width,
        height: 1.min(cols[1].height.saturating_sub(1)),
    };
    if bar.width == 0 {
        return;
    }
    let Some((inn, out, dur)) = app.trim_range() else {
        frame.render_widget(
            Paragraph::new("load a tape to cut it").style(theme::ghost()),
            bar,
        );
        return;
    };
    let w = bar.width;
    let mut x0 = util::trim_x(inn, dur, w);
    let mut x1 = util::trim_x(out, dur, w);
    if let Some(drag) = app.trim_drag {
        match drag.handle {
            TrimHandle::In => x0 = Some(drag.x.min(w.saturating_sub(1))),
            TrimHandle::Out => x1 = Some(drag.x.min(w.saturating_sub(1))),
        }
    }
    let (lo, hi) = match (x0, x1) {
        (Some(a), Some(b)) => (a.min(b), a.max(b)),
        (Some(a), None) | (None, Some(a)) => (a, a),
        _ => (u16::MAX, u16::MAX),
    };
    let active_in = app.focus != Focus::TrimOut;
    for i in 0..w {
        let on_in = x0 == Some(i);
        let on_out = x1 == Some(i);
        let inside = i >= lo && i <= hi;
        let (ch, fg) = if on_in {
            ('●', if active_in { theme::WHITE } else { theme::AMBER })
        } else if on_out {
            ('○', if active_in { theme::AMBER } else { theme::WHITE })
        } else if inside {
            ('━', theme::AMBER)
        } else {
            ('─', theme::BORDER)
        };
        let cell = &mut frame.buffer_mut()[(bar.x + i, bar.y)];
        cell.set_char(ch);
        cell.fg = fg;
        cell.bg = theme::BG;
    }
    if cols[1].height > 2 {
        let live = if app.focus == Focus::TrimOut { out } else { inn };
        let caption = slider_caption(
            w as usize,
            &format_timestamp(0.0),
            &format_timestamp(live),
            &format_timestamp(dur),
        );
        frame.render_widget(
            Paragraph::new(caption).style(theme::ghost()),
            Rect {
                y: cols[1].y.saturating_add(2),
                height: 1,
                ..cols[1]
            },
        );
    }
}

fn draw_time_box(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    field: &Field,
    focused: bool,
    hint: &str,
) -> Rect {
    if area.width == 0 || area.height == 0 {
        return area;
    }
    let box_area = Rect {
        height: area.height.min(3),
        ..area
    };
    let borders = if box_area.height >= 3 {
        Borders::ALL
    } else {
        Borders::LEFT | Borders::RIGHT
    };
    let block = Block::bordered()
        .borders(borders)
        .border_type(BorderType::Rounded)
        .border_style(theme::border(focused))
        .title(Span::styled(format!(" {title} "), theme::title(focused)))
        .style(theme::root());
    let inner = block.inner(box_area);
    frame.render_widget(block, box_area);
    draw_field(frame, inner, field, focused, hint);
    box_area
}

fn slider_caption(width: usize, left: &str, mid: &str, right: &str) -> String {
    if width == 0 {
        return String::new();
    }
    let mut line = vec![' '; width];
    for (i, c) in left.chars().enumerate() {
        if i < width {
            line[i] = c;
        }
    }
    let rs = width.saturating_sub(right.chars().count());
    for (i, c) in right.chars().enumerate() {
        if let Some(cell) = line.get_mut(rs + i) {
            *cell = c;
        }
    }
    let ms = width.saturating_sub(mid.chars().count()) / 2;
    for (i, c) in mid.chars().enumerate() {
        if let Some(cell) = line.get_mut(ms + i) {
            *cell = c;
        }
    }
    line.into_iter().collect()
}

// ── setup column ────────────────────────────────────────────────────────

fn draw_setup(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = Layout::vertical([Constraint::Min(7), Constraint::Length(3)]).split(area);
    let block = panel("SETUP", false);
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);

    // Stack "label / value" when there is room, otherwise one line each.
    let stacked = inner.height >= 10;
    let gap = u16::from(inner.height >= 15);
    let step = if stacked { 2 + gap } else { 1 };
    let slot = |i: u16| -> Rect {
        let y = inner.y + i * step;
        let h = if stacked { 2 } else { 1 };
        Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: h.min((inner.y + inner.height).saturating_sub(y)),
        }
    };
    let value = |r: Rect| -> Rect {
        if stacked {
            Rect {
                x: r.x + 1,
                y: r.y + 1,
                width: r.width.saturating_sub(2),
                height: r.height.saturating_sub(1).min(1),
            }
        } else {
            Rect {
                x: r.x + 10,
                width: r.width.saturating_sub(11),
                ..r
            }
        }
    };
    let caption = |frame: &mut Frame, r: Rect, text: &str, focused: bool| {
        if r.height == 0 {
            return;
        }
        let (mark, style) = if focused {
            ("▸", theme::accent_bold())
        } else {
            (" ", theme::ghost())
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(mark, theme::accent()),
                Span::styled(text.to_string(), style),
            ])),
            Rect { height: 1, ..r },
        );
    };

    let r = slot(0);
    app.hits.mode = r;
    let f = app.focus == Focus::Mode;
    caption(frame, r, "type", f);
    draw_chips(
        frame,
        value(r),
        f,
        &[
            (MediaMode::Video.label(), app.mode == MediaMode::Video),
            (MediaMode::Audio.label(), app.mode == MediaMode::Audio),
        ],
    );

    let r = slot(1);
    app.hits.container = r;
    let f = app.focus == Focus::Container;
    caption(frame, r, "format", f);
    let items: Vec<(&str, bool)> = match app.mode {
        MediaMode::Video => crate::engine::VideoContainer::ALL
            .iter()
            .map(|c| (c.label(), *c == app.video_container))
            .collect(),
        MediaMode::Audio => crate::engine::AudioContainer::ALL
            .iter()
            .map(|c| (c.label(), *c == app.audio_container))
            .collect(),
    };
    draw_chips(frame, value(r), f, &items);

    let r = slot(2);
    app.hits.quality = r;
    let f = app.focus == Focus::Quality;
    caption(frame, r, "quality", f);
    let items: Vec<(&str, bool)> = match app.mode {
        MediaMode::Video => VIDEO_QUALITIES
            .iter()
            .enumerate()
            .map(|(i, q)| (q.label, i == app.video_quality))
            .collect(),
        MediaMode::Audio => AUDIO_QUALITIES
            .iter()
            .enumerate()
            .map(|(i, q)| (q.label, i == app.audio_quality))
            .collect(),
    };
    draw_chips(frame, value(r), f, &items);

    let r = slot(3);
    app.hits.playlist = r;
    let f = app.focus == Focus::Playlist;
    caption(frame, r, "playlist", f);
    draw_chips(frame, value(r), f, &[("OFF", !app.playlist), ("ON", app.playlist)]);

    let r = slot(4);
    app.hits.output = r;
    let f = app.focus == Focus::Output;
    caption(frame, r, "save to", f);
    draw_field(frame, value(r), &app.output, f, "directory");

    app.hits.download = rows[1];
    draw_rec_button(frame, rows[1], app);
}

fn draw_chips(frame: &mut Frame, area: Rect, focused: bool, items: &[(&str, bool)]) {
    if items.is_empty() || area.width == 0 || area.height == 0 {
        return;
    }
    let selected = items.iter().position(|(_, on)| *on).unwrap_or(0);
    let widths: Vec<usize> = items.iter().map(|(l, _)| l.len() + 2).collect();
    let width = area.width as usize;
    let (lo, hi) = util::choice_window(&widths, selected, width.saturating_sub(2));
    let arrow = |on: bool, ch: &'static str| {
        Span::styled(if on { ch } else { " " }, theme::ghost())
    };
    let mut spans = vec![arrow(lo > 0, "‹")];
    for (i, (label, on)) in items.iter().enumerate().take(hi + 1).skip(lo) {
        if i > lo {
            spans.push(Span::raw(" "));
        }
        let style = match (*on, focused) {
            (true, true) => Style::new()
                .fg(theme::BG)
                .bg(theme::AMBER)
                .add_modifier(Modifier::BOLD),
            (true, false) => Style::new()
                .fg(theme::BG)
                .bg(theme::FG_DIM)
                .add_modifier(Modifier::BOLD),
            _ => theme::dim(),
        };
        spans.push(Span::styled(format!(" {label} "), style));
    }
    spans.push(arrow(hi + 1 < items.len(), "›"));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// A chunky record button with half-block shoulders.
fn draw_rec_button(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < 3 || area.width < 4 {
        return;
    }
    let focused = app.focus == Focus::Download;
    let live = app.phase == Phase::Extracting;
    let face = if focused || live { theme::REC } else { theme::REC_DEEP };
    let buf = frame.buffer_mut();
    for x in area.x + 1..area.x + area.width - 1 {
        for (dy, ch) in [(0, '▄'), (2, '▀')] {
            let cell = &mut buf[(x, area.y + dy)];
            cell.set_char(ch);
            cell.fg = face;
            cell.bg = theme::BG;
        }
    }
    let mid = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width - 2,
        height: 1,
    };
    let dot = if live && !blink(app, 25) { " " } else { "●" };
    let dot_fg = if focused || live { theme::WHITE } else { theme::REC };
    let label = match app.progress.stage {
        stage if live && stage.is_post() => stage_word(stage),
        _ if live => "RECORDING",
        _ => "DOWNLOAD",
    };
    let text_style = Style::new()
        .fg(if focused || live { theme::WHITE } else { theme::FG })
        .bg(face)
        .add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(dot, Style::new().fg(dot_fg).bg(face)),
            Span::styled(format!("  {label}"), text_style),
        ]))
        .alignment(Alignment::Center)
        .style(Style::new().bg(face)),
        mid,
    );
}

// ── inputs ──────────────────────────────────────────────────────────────

fn draw_field(frame: &mut Frame, area: Rect, field: &Field, focused: bool, hint: &str) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let area = Rect { height: 1, ..area };
    if field.is_empty() && !focused {
        frame.render_widget(
            Paragraph::new(format!(" {hint}"))
                .style(Style::new().fg(theme::FG_GHOST).bg(theme::BG_ELEVATED)),
            area,
        );
        return;
    }
    let text = Rect {
        x: area.x + 1,
        width: area.width.saturating_sub(1),
        ..area
    };
    frame.render_widget(Block::new().style(theme::input(focused)), area);
    let width = text.width.max(1) as usize;
    frame.render_widget(Paragraph::new(field_line(field, width, focused)), text);
    if focused && text.width > 0 {
        let (_, cur_x) = field.visible(width);
        frame.set_cursor_position(Position::new(text.x + cur_x as u16, text.y));
    }
}

fn field_line(field: &Field, width: usize, focused: bool) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }
    let (start, cur) = field.visible(width);
    let chars = field.chars();
    let spans: Vec<Span> = (0..width)
        .map(|i| {
            let ch = chars.get(start + i).copied().unwrap_or(' ');
            let style = if focused && i == cur {
                Style::new().fg(theme::BG).bg(theme::AMBER)
            } else {
                theme::input(focused)
            };
            Span::styled(ch.to_string(), style)
        })
        .collect();
    Line::from(spans)
}

// ── log and tape meter ──────────────────────────────────────────────────

fn draw_log(frame: &mut Frame, area: Rect, app: &App) {
    let block = panel("LOG", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let h = inner.height as usize;
    let start = app.logs.len().saturating_sub(h);
    let lines: Vec<Line> = app
        .logs
        .iter()
        .skip(start)
        .map(|l| {
            let (clock, msg) = l.split_once("  ").unwrap_or(("", l.as_str()));
            let fault = msg.contains("ERROR") || msg.contains("FAULT") || msg.contains("MISSING");
            let style = if fault {
                Style::new().fg(theme::REC).bg(theme::BG)
            } else if msg.starts_with("WRITE") || msg.starts_with("LOADED") {
                theme::root()
            } else {
                theme::dim()
            };
            let room = (inner.width as usize).saturating_sub(clock.len() + 3);
            Line::from(vec![
                Span::styled(format!(" {clock}  "), theme::ghost()),
                Span::styled(util::truncate(msg, room), style),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_tape(frame: &mut Frame, area: Rect, app: &App) {
    let live = app.phase == Phase::Extracting;
    let post = live && app.progress.stage.is_post();
    let mut block = panel("TAPE", false);
    if live {
        let (step, style) = if post {
            ("step 2", Style::new().fg(theme::VFD).bg(theme::BG))
        } else {
            ("step 1", theme::rec())
        };
        block = block.title(
            Line::from(vec![
                Span::styled(format!(" {step} · "), theme::ghost()),
                Span::styled(format!("{} ", stage_word(app.progress.stage)), style),
            ])
            .right_aligned(),
        );
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width < 4 {
        return;
    }
    let pad = |r: Rect| Rect {
        x: r.x + 1,
        width: r.width.saturating_sub(2),
        ..r
    };
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let (pct, meter) = match app.phase {
        Phase::Extracting if post => match app.progress.post {
            Some(p) => (p, Meter::Work),
            None => (0.0, Meter::Busy),
        },
        Phase::Extracting => (app.progress.percent, Meter::Rec),
        Phase::Done => (100.0, Meter::Done),
        Phase::Failed => (100.0, Meter::Fault),
        Phase::Probing => (0.0, Meter::Seek),
        _ => (0.0, Meter::Idle),
    };
    let bar = pad(rows[0]);
    effects::render_meter(frame.buffer_mut(), bar, pct, app.tick, meter);
    let stamp = match app.phase {
        Phase::Extracting if post => match app.progress.post {
            Some(p) => format!("{} {}", stage_word(app.progress.stage), format_live_percent(p)),
            None => stage_word(app.progress.stage).to_string(),
        },
        Phase::Extracting => format_live_percent(app.progress.percent),
        Phase::Done => "COMPLETE".into(),
        Phase::Failed => "ERROR".into(),
        _ => String::new(),
    };
    let fill = if meter == Meter::Busy { 0.0 } else { pct / 100.0 };
    effects::stamp_meter(frame.buffer_mut(), bar, &stamp, fill);

    if rows[1].height > 0 {
        let meta = meta_line(app);
        frame.render_widget(Paragraph::new(meta), pad(rows[1]));
    }
    if rows[2].height > 0 && inner.height >= 3 {
        effects::render_waveform(frame.buffer_mut(), pad(rows[2]), &app.speed_hist, live);
    }
}

fn meta_line(app: &App) -> Line<'static> {
    let or_dash = |s: &str| if s.is_empty() { "—".to_string() } else { s.to_string() };
    match app.phase {
        Phase::Extracting if app.progress.stage.is_post() => {
            let p = &app.progress;
            let elapsed = app.stage_started.elapsed().as_secs_f64();
            let teal = Style::new().fg(theme::VFD).bg(theme::BG);
            let mut spans = vec![Span::styled(stage_detail(p.stage), teal)];
            if let Some(pct) = p.post {
                spans.push(Span::styled(format!("   {}", format_live_percent(pct)), theme::bold()));
            }
            spans.push(Span::styled("   elapsed ", theme::ghost()));
            spans.push(Span::styled(clock_secs(elapsed), theme::root()));
            match p.post {
                Some(pct) if pct >= 2.0 && elapsed >= 2.0 => {
                    let eta = elapsed * (100.0 - pct) / pct;
                    spans.push(Span::styled("   eta ", theme::ghost()));
                    spans.push(Span::styled(clock_secs(eta), theme::root()));
                }
                Some(_) => {}
                None => spans.push(Span::styled(
                    "   long videos take a while here",
                    theme::ghost(),
                )),
            }
            Line::from(spans)
        }
        Phase::Extracting => {
            let p = &app.progress;
            let mut spans = vec![
                Span::styled(format_live_percent(p.percent), theme::bold()),
                Span::styled("   speed ", theme::ghost()),
                Span::styled(or_dash(&p.speed), theme::root()),
                Span::styled("   eta ", theme::ghost()),
                Span::styled(or_dash(&p.eta), theme::root()),
            ];
            if !p.total.is_empty() {
                spans.push(Span::styled("   size ", theme::ghost()));
                spans.push(Span::styled(p.total.clone(), theme::root()));
            }
            Line::from(spans)
        }
        Phase::Done => Line::from(vec![
            Span::styled("saved  ", theme::ok()),
            Span::styled(
                app.last_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                theme::dim(),
            ),
        ]),
        Phase::Failed => Line::from(Span::styled(
            app.last_error.clone().unwrap_or_else(|| "ERROR".into()),
            Style::new().fg(theme::REC).bg(theme::BG),
        )),
        Phase::Probing => Line::from(Span::styled("reading the tape header…", theme::dim())),
        Phase::Ready => Line::from(Span::styled(
            "tape cued — press F6 or DOWNLOAD to record",
            theme::dim(),
        )),
        _ => Line::from(Span::styled("stopped", theme::ghost())),
    }
}

fn format_live_percent(percent: f64) -> String {
    let p = percent.clamp(0.0, 99.9);
    if !(10.0..99.0).contains(&p) {
        format!("{p:.1}%")
    } else {
        format!("{p:.0}%")
    }
}

// ── keys, help, boot ────────────────────────────────────────────────────

fn draw_keys(frame: &mut Frame, area: Rect, app: &App) {
    let keys: &[(&str, &str)] = if app.phase == Phase::Extracting {
        &[("esc", "stop"), ("q", "quit")]
    } else if app.update_available.is_some() {
        &[
            ("u", "update"),
            ("tab", "focus"),
            ("enter", "load"),
            ("f6", "rec"),
            ("m", "type"),
            ("[ ]", "trim"),
            ("?", "help"),
            ("q", "quit"),
        ]
    } else {
        &[
            ("tab", "focus"),
            ("enter", "load"),
            ("f6", "rec"),
            ("m", "type"),
            ("[ ]", "trim"),
            ("?", "help"),
            ("q", "quit"),
        ]
    };
    let mut spans = Vec::new();
    for (i, (k, v)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("   ", theme::ghost()));
        }
        spans.push(Span::styled(*k, theme::dim()));
        spans.push(Span::styled(format!(" {v}"), theme::ghost()));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let popup = centered(area, 70, 20);
    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(theme::AMBER_DIM).bg(theme::BG))
        .title(Span::styled(" MANUAL ", theme::accent_bold()))
        .style(theme::root());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let entry = |k: &str, v: &str| {
        Line::from(vec![
            Span::styled(format!("  {k:<20}"), theme::accent()),
            Span::styled(v.to_string(), theme::root()),
        ])
    };
    let text = vec![
        Line::from(Span::styled(
            "  NITRATE — YouTube · X · Facebook",
            theme::bold(),
        )),
        Line::from(""),
        entry("paste / enter", "load the tape (reads metadata via yt-dlp)"),
        entry("f6 / download", "record with the chosen quality and trim"),
        entry("tab / shift-tab", "move focus"),
        entry("m", "switch video / audio"),
        entry("← → on a setting", "cycle format, quality, playlist"),
        entry("[ ]  { }", "nudge trim by 1s / 5s"),
        entry("drag ● / ○", "set the cut   right-click sets OUT"),
        entry("ctrl-u / ctrl-w", "clear field / delete word"),
        entry("u", "apply the latest release"),
        entry("esc", "stop recording"),
        entry("q / ctrl-c", "quit"),
        Line::from(""),
        Line::from(Span::styled(
            "  Trim uses yt-dlp --download-sections. ffmpeg is needed to merge.",
            theme::ghost(),
        )),
    ];
    frame.render_widget(Paragraph::new(text), inner);
}

/// Power-on: a burst of static, then the VCR's blue screen.
fn draw_boot(frame: &mut Frame, area: Rect, app: &App) {
    let elapsed = app.started.elapsed();
    let ms = elapsed.as_millis();
    let buf = frame.buffer_mut();
    if ms < 380 {
        let strength = 1.0 - ms as f32 / 900.0;
        effects::render_snow(buf, area, app.tick, strength);
        return;
    }
    let blue = theme::BLUE;
    let fg = theme::WHITE;
    let soft = theme::mix(blue, theme::WHITE, 0.55);
    frame.render_widget(Block::new().style(Style::new().bg(blue)), area);
    let bold = Style::new().fg(fg).bg(blue).add_modifier(Modifier::BOLD);
    let plain = Style::new().fg(soft).bg(blue);

    let buf = frame.buffer_mut();
    buf.set_string(area.x + 3, area.y + 1, "▶ PLAY", bold);
    let ch = "CH 03";
    buf.set_string(area.x + area.width.saturating_sub(ch.len() as u16 + 3), area.y + 1, ch, bold);

    let top = area.y + area.height.saturating_sub(18) / 2 + 1;
    let mark = vhs::render_wordmark(
        buf,
        Rect {
            y: top,
            height: 6,
            ..area
        },
        "NITRATE",
        fg,
        blue,
    );
    let mut y = top + mark.height.max(1) + 1;
    let center = |buf: &mut ratatui::buffer::Buffer, y: u16, s: &str, style: Style| {
        let w = unicode_width::UnicodeWidthStr::width(s) as u16;
        if y < area.y + area.height {
            buf.set_string(area.x + area.width.saturating_sub(w) / 2, y, s, style);
        }
    };
    center(buf, y, "VIDEO CASSETTE RECORDER", plain);
    y += 2;

    let checks = [
        (500, "video heads", true),
        (750, "tape transport", true),
        (1000, "yt-dlp", app.tools.ytdlp.is_some()),
        (1250, "ffmpeg", app.tools.ffmpeg.is_some()),
    ];
    for (at, name, ok) in checks {
        if ms >= at {
            let line = format!("{name:<16}{:>6}", if ok { "OK" } else { "MISSING" });
            let style = if ok { plain } else { bold.fg(theme::STRIPES[2]) };
            center(buf, y, &line, style);
        }
        y += 1;
    }
    if ms >= 1500 && blink(app, 30) {
        center(buf, y + 1, "press any key", plain);
    }

    let counter = format!("SP  {}", clock(elapsed));
    buf.set_string(area.x + 3, area.y + area.height.saturating_sub(2), counter, bold);

    if ms < 700 {
        effects::render_tracking(buf, area, app.tick);
    }
}

fn clock(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::time::Instant;

    fn dump(buf: &ratatui::buffer::Buffer) -> String {
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    fn render(app: &mut App) -> String {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, app)).unwrap();
        dump(terminal.backend().buffer())
    }

    #[test]
    fn boot_shows_blue_screen() {
        let mut app = App::new();
        app.started = Instant::now() - Duration::from_millis(1600);
        let text = render(&mut app);
        assert!(text.contains("PLAY"), "{text}");
        assert!(text.contains("VIDEO CASSETTE RECORDER"), "{text}");
    }

    #[test]
    fn console_renders_after_boot() {
        let mut app = App::new();
        app.skip_boot();
        let text = render(&mut app);
        assert!(text.contains("NITRATE"), "{text}");
        assert!(text.contains("URL"), "{text}");
        assert!(text.contains("LABEL"), "{text}");
        assert!(text.contains("DECK"), "{text}");
        assert!(text.contains("NO TAPE"), "{text}");
        assert!(text.contains("type"), "{text}");
        assert!(text.contains("format"), "{text}");
        assert!(text.contains("quality"), "{text}");
        assert!(text.contains("DOWNLOAD"), "{text}");
        assert!(!text.contains("COOKIES"), "{text}");
    }

    #[test]
    fn label_shows_title_and_cassette_loads() {
        let mut app = App::new();
        app.skip_boot();
        app.url.set("https://www.youtube.com/watch?v=abc");
        app.info = Some(crate::engine::VideoInfo {
            id: "abc".into(),
            title: "A VERY LONG VIDEO TITLE THAT MUST SLIDE IN THE LABEL".into(),
            duration: Some(90.0),
            extractor: "Youtube".into(),
            webpage_url: "https://www.youtube.com/watch?v=abcdefghijklmnop".into(),
            platform: crate::util::Platform::YouTube,
        });
        app.deck.seated = 1.0;
        let text = render(&mut app);
        assert!(text.contains("title"), "{text}");
        assert!(text.contains("abc"), "{text}");
        assert!(text.contains(" MAX "), "{text}");
        assert!(text.contains("NITRATE HQ"), "{text}");
        assert!(!text.contains("NO TAPE"), "{text}");
    }

    #[test]
    fn progress_bar_shows_percent_complete_and_error() {
        let mut app = App::new();
        app.skip_boot();

        app.phase = Phase::Extracting;
        app.progress.percent = 45.0;
        let text = render(&mut app);
        assert!(text.contains("45%"), "{text}");
        assert!(text.contains("REC"), "{text}");

        app.progress.percent = 0.4;
        assert!(render(&mut app).contains("0.4%"));

        app.phase = Phase::Done;
        app.progress.percent = 100.0;
        assert!(render(&mut app).contains("COMPLETE"));

        app.phase = Phase::Failed;
        assert!(render(&mut app).contains("ERROR"));
    }

    #[test]
    fn trim_slider_shows_after_url_without_probe() {
        let mut app = App::new();
        app.skip_boot();
        app.url.set("https://youtu.be/abc");
        let text = render(&mut app);
        assert!(text.contains("TRIM"), "{text}");
        assert!(!text.contains("load a tape to cut it"), "{text}");
        assert!(app.trim_range().is_some());
    }

    #[test]
    fn trim_handles_sit_at_start_and_end_with_time_boxes() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.skip_boot();
        app.info = Some(crate::engine::VideoInfo {
            id: "abc".into(),
            title: "t".into(),
            duration: Some(3600.0),
            extractor: "Youtube".into(),
            webpage_url: "https://youtu.be/abc".into(),
            platform: crate::util::Platform::YouTube,
        });
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let buf = terminal.backend().buffer();
        let text = dump(buf);
        let bar = app.hits.trim_bar;
        let y = bar.y.saturating_add(1);
        assert_eq!(buf[(bar.x, y)].symbol(), "●", "{text}");
        assert_eq!(
            buf[(bar.x + bar.width.saturating_sub(1), y)].symbol(),
            "○",
            "{text}"
        );
        let inn = app.hits.trim_in;
        let out = app.hits.trim_out;
        assert_eq!(buf[(inn.x, inn.y)].symbol(), "╭", "{text}");
        assert_eq!(buf[(out.x, out.y)].symbol(), "╭", "{text}");
        assert!(text.contains(" IN "), "{text}");
        assert!(text.contains(" OUT "), "{text}");
    }

    #[test]
    fn narrow_and_small_terminals_render() {
        let mut app = App::new();
        app.skip_boot();
        app.url.set("https://youtu.be/abc");
        app.deck.seated = 1.0;
        app.phase = Phase::Probing;
        for (w, h) in [(80, 22), (90, 26), (103, 30), (200, 60), (60, 10)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        }
    }
}
