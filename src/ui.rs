use crate::app::{App, Focus, Phase, TrimHandle};
use crate::effects::{self, MeterSkin};
use crate::engine::{MediaMode, AUDIO_QUALITIES, VIDEO_QUALITIES};
use crate::theme;
use crate::util::{self, format_timestamp, Field};
use ratatui::layout::{Alignment, Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    frame.render_widget(Block::new().style(theme::root()), area);
    if area.width < 80 || area.height < 22 {
        draw_tiny(frame, area, app);
        finish(frame, app);
        return;
    }

    let cols = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(8),
        Constraint::Length(5),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .split(area);

    draw_header(frame, cols[0], app);
    draw_body(frame, cols[1], app);
    draw_telemetry(frame, cols[2], app);
    draw_progress(frame, cols[3], app);
    draw_keys(frame, cols[4], app);

    if app.phase == Phase::Boot {
        draw_boot(frame, area, app);
    }
    if app.help {
        draw_help(frame, area);
    }
    finish(frame, app);
}

fn finish(frame: &mut Frame, app: &mut App) {
    effects::render_particles(frame.buffer_mut(), &app.particles);
}

fn draw_tiny(frame: &mut Frame, area: Rect, app: &App) {
    let msg = format!(
        "NITRATE  VIEWPORT TOO SMALL  {}x{}  NEED 80x22",
        area.width, area.height
    );
    frame.render_widget(
        Paragraph::new(msg)
            .style(theme::accent_bold())
            .alignment(Alignment::Center),
        area,
    );
    let _ = app;
}

fn panel(title: &str, focused: bool) -> Block<'_> {
    Block::bordered()
        .borders(Borders::ALL)
        .border_style(theme::border(focused))
        .title(format!(" {title} "))
        .title_style(theme::title(focused))
        .style(theme::root())
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let live = app.phase == Phase::Extracting;
    let block = Block::bordered()
        .border_style(if live {
            theme::accent()
        } else {
            theme::border(false)
        })
        .title(" NITRATE ")
        .title_style(theme::accent_bold())
        .title(Line::from(vec![
            Span::styled(" UNIT/VT-01 ", theme::dim()),
            Span::styled(format!("REV {} ", crate::update::VERSION), theme::ghost()),
        ]).right_aligned())
        .style(theme::root());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(inner);
    let clock = util::format_mission(app.started.elapsed());
    let plat = app.platform().label();
    let phase = app.phase.label();
    let phase_style = match app.phase {
        Phase::Done => theme::lock(),
        Phase::Failed | Phase::Extracting => theme::accent_bold(),
        Phase::Probing | Phase::Ready => theme::accent(),
        _ => theme::bold(),
    };

    let line0 = Line::from(vec![
        Span::styled("VIDEO EXTRACTION CONSOLE", theme::bold()),
        Span::raw("   "),
        Span::styled("///", theme::accent()),
        Span::raw("   "),
        Span::styled(clock, theme::dim()),
        Span::raw("   "),
        Span::styled(format!("FRM {:>6}", app.tick), theme::ghost()),
    ]);
    frame.render_widget(Paragraph::new(line0), rows[0]);

    if live && rows[1].width > 0 {
        effects::render_hazard(frame.buffer_mut(), rows[1], app.tick);
        let stamp = format!(" {}  {}  {} ", plat, phase, app.progress.stage.label());
        frame.buffer_mut().set_stringn(
            rows[1].x,
            rows[1].y,
            &stamp,
            stamp.len().min(rows[1].width as usize),
            theme::accent_bold(),
        );
    } else {
        let ytdlp = if app.tools.ytdlp.is_some() { "YT-DLP OK" } else { "YT-DLP --" };
        let ffmpeg = if app.tools.ffmpeg.is_some() { "FFMPEG OK" } else { "FFMPEG --" };
        let mut line1 = vec![
            Span::styled(format!("{plat:<10}"), theme::accent()),
            Span::styled(phase, phase_style),
            Span::raw("   "),
            Span::styled(ytdlp, theme::dim()),
            Span::raw("  "),
            Span::styled(ffmpeg, theme::dim()),
            Span::raw("  "),
            Span::styled("CLASS:UNRESTRICTED", theme::ghost()),
        ];
        if let Some(ver) = &app.update_available {
            line1.push(Span::raw("  "));
            line1.push(Span::styled(format!("UPDATE {ver}"), theme::accent_bold()));
        }
        let line1 = Line::from(line1);
        frame.render_widget(Paragraph::new(line1), rows[1]);
    }
}

fn draw_body(frame: &mut Frame, area: Rect, app: &mut App) {
    let cols = Layout::horizontal([
        Constraint::Length(30),
        Constraint::Min(36),
        Constraint::Length(28),
    ])
    .split(area);
    draw_left(frame, cols[0], app);
    draw_center(frame, cols[1], app);
    draw_right(frame, cols[2], app);
}

fn draw_left(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = Layout::vertical([
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Min(4),
    ])
    .split(area);

    let ident = panel("IDENT", false);
    let inner = ident.inner(rows[0]);
    frame.render_widget(ident, rows[0]);
    let url = app.url.text();
    let plat = app.platform().label();
    let id = app.info.as_ref().map(|i| i.id.as_str()).unwrap_or("—");
    let dur = app
        .duration()
        .map(format_timestamp)
        .unwrap_or_else(|| "--:--".into());
    let val_w = inner.width.saturating_sub(5) as usize;
    let title = if app.phase == Phase::Probing {
        "LOCKING TARGET"
    } else {
        app.info
            .as_ref()
            .map(|i| i.title.as_str())
            .unwrap_or("NO LOCK")
    };
    let link = app
        .info
        .as_ref()
        .map(|i| i.webpage_url.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(url.as_str());
    let ext = app
        .info
        .as_ref()
        .map(|i| i.extractor.as_str())
        .unwrap_or("—");
    let ident_lines = vec![
        Line::from(vec![
            Span::styled("SRC  ", theme::dim()),
            Span::styled(plat, theme::accent()),
        ]),
        Line::from(vec![
            Span::styled("ID   ", theme::dim()),
            Span::styled(id.to_string(), theme::root()),
        ]),
        Line::from(vec![
            Span::styled("LEN  ", theme::dim()),
            Span::styled(dur, theme::root()),
        ]),
        Line::from(vec![
            Span::styled("TTL  ", theme::dim()),
            Span::styled(util::marquee(title, val_w, app.tick), theme::bold()),
        ]),
        Line::from(vec![
            Span::styled("EXT  ", theme::dim()),
            Span::styled(util::marquee(&ext.to_ascii_uppercase(), val_w, app.tick), theme::accent()),
        ]),
        Line::from(vec![
            Span::styled("URL  ", theme::dim()),
            Span::styled(util::marquee(link, val_w, app.tick), theme::ghost()),
        ]),
    ];
    frame.render_widget(Paragraph::new(ident_lines), inner);

    let tools = panel("TOOLS", false);
    let inner = tools.inner(rows[1]);
    frame.render_widget(tools, rows[1]);
    let y = match &app.tools.ytdlp {
        Some((_, v)) => util::truncate(v, inner.width as usize),
        None => "MISSING".into(),
    };
    let f = match &app.tools.ffmpeg {
        Some((_, v)) => util::truncate(v, inner.width as usize),
        None => "MISSING".into(),
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled("YT-DLP", theme::dim())),
            Line::from(Span::styled(y, theme::root())),
            Line::from(Span::styled("FFMPEG", theme::dim())),
            Line::from(Span::styled(f, theme::root())),
        ]),
        inner,
    );

    let hist = panel("ARCHIVE", app.focus == Focus::History);
    app.hits.history = rows[2];
    let inner = hist.inner(rows[2]);
    frame.render_widget(hist, rows[2]);
    if app.history.is_empty() {
        frame.render_widget(
            Paragraph::new("NO WRITES").style(theme::ghost()),
            inner,
        );
    } else {
        let mut lines = Vec::new();
        let h = inner.height as usize;
        let start = app.history_cursor.saturating_sub(h.saturating_sub(1));
        for (i, item) in app.history.iter().enumerate().skip(start).take(h) {
            let mark = if item.ok { "+" } else { "x" };
            let sty = if i == app.history_cursor && app.focus == Focus::History {
                theme::accent_bold()
            } else if !item.ok {
                theme::accent()
            } else {
                theme::root()
            };
            lines.push(Line::from(Span::styled(
                format!(
                    "{mark} {} {}",
                    item.platform.label(),
                    util::truncate(&item.title, inner.width.saturating_sub(8) as usize)
                ),
                sty,
            )));
        }
        frame.render_widget(Paragraph::new(lines), inner);
    }
}

fn draw_center(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(4),
        Constraint::Min(4),
        Constraint::Length(3),
    ])
    .split(area);

    app.hits.url = rows[0];
    draw_input(frame, rows[0], "URL", &app.url, app.focus == Focus::Url, "paste target");

    let pair = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(rows[1]);
    app.hits.mode = pair[0];
    draw_choice(
        frame,
        pair[0],
        "TYPE",
        app.focus == Focus::Mode,
        &[
            (MediaMode::Video.label(), app.mode == MediaMode::Video),
            (MediaMode::Audio.label(), app.mode == MediaMode::Audio),
        ],
    );
    app.hits.container = pair[1];
    match app.mode {
        MediaMode::Video => {
            let items: Vec<(&str, bool)> = crate::engine::VideoContainer::ALL
                .iter()
                .map(|c| (c.label(), *c == app.video_container))
                .collect();
            draw_choice(frame, pair[1], "FORMAT", app.focus == Focus::Container, &items);
        }
        MediaMode::Audio => {
            let items: Vec<(&str, bool)> = crate::engine::AudioContainer::ALL
                .iter()
                .map(|c| (c.label(), *c == app.audio_container))
                .collect();
            draw_choice(frame, pair[1], "FORMAT", app.focus == Focus::Container, &items);
        }
    }

    app.hits.quality = rows[2];
    match app.mode {
        MediaMode::Video => {
            let items: Vec<(&str, bool)> = VIDEO_QUALITIES
                .iter()
                .enumerate()
                .map(|(i, q)| (q.label, i == app.video_quality))
                .collect();
            draw_choice(frame, rows[2], "QUALITY", app.focus == Focus::Quality, &items);
        }
        MediaMode::Audio => {
            let items: Vec<(&str, bool)> = AUDIO_QUALITIES
                .iter()
                .enumerate()
                .map(|(i, q)| (q.label, i == app.audio_quality))
                .collect();
            draw_choice(frame, rows[2], "QUALITY", app.focus == Focus::Quality, &items);
        }
    }

    app.hits.timeline = rows[3];
    draw_timeline(frame, rows[3], app);

    app.hits.download = rows[4];
    draw_download(frame, rows[4], app);
}

fn draw_download(frame: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Download;
    let fill = Style::new()
        .fg(Color::Black)
        .bg(theme::RED)
        .add_modifier(Modifier::BOLD);
    let block = Block::bordered()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(if focused { Color::Black } else { theme::RED }).bg(theme::RED))
        .style(fill);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let label = match app.phase {
        Phase::Extracting => "WORKING",
        _ => "DOWNLOAD",
    };
    frame.render_widget(
        Paragraph::new(label)
            .style(fill)
            .alignment(Alignment::Center),
        inner,
    );
}



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
    .split(inner);
    app.hits.trim_in = cols[0];
    app.hits.trim_out = cols[2];
    app.hits.trim_bar = cols[1];

    let inn_s = if app.trim_in.is_empty() {
        "IN 0:00"
    } else {
        "IN"
    };
    let out_s = if app.trim_out.is_empty() {
        "OUT END"
    } else {
        "OUT"
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(inn_s, theme::dim())),
            Line::from(field_line(&app.trim_in, 10, app.focus == Focus::TrimIn)),
        ]),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(out_s.to_string(), theme::dim())).right_aligned(),
            Line::from(field_line(&app.trim_out, 10, app.focus == Focus::TrimOut)).alignment(Alignment::Right),
        ]),
        cols[2],
    );

    let bar = Rect {
        x: cols[1].x,
        y: cols[1].y.saturating_add(1),
        width: cols[1].width,
        height: 1.min(cols[1].height.saturating_sub(1)),
    };
    if bar.width > 0 {
        if let Some((inn, out, dur)) = app.trim_range() {
            let w = bar.width;
            app.sync_trim_view(w);
            let view = app.trim_view.round();
            let view_end = view + f64::from(util::trim_span(w));
            let mut x0 = util::trim_x(inn, view, w);
            let mut x1 = util::trim_x(out, view, w);
            if let Some(drag) = app.trim_drag {
                match drag.handle {
                    TrimHandle::In => x0 = Some(drag.x.min(w.saturating_sub(1))),
                    TrimHandle::Out => x1 = Some(drag.x.min(w.saturating_sub(1))),
                }
            }
            let fill_lo = if out >= view && inn <= view_end {
                Some(((inn.max(view) - view).round() as u16).min(w.saturating_sub(1)))
            } else {
                None
            };
            let fill_hi = if out >= view && inn <= view_end {
                Some(((out.min(view_end) - view).round() as u16).min(w.saturating_sub(1)))
            } else {
                None
            };
            for i in 0..w {
                let on_in = x0 == Some(i);
                let on_out = x1 == Some(i);
                let in_fill = match (fill_lo, fill_hi) {
                    (Some(lo), Some(hi)) if i >= lo && i <= hi => true,
                    _ => false,
                };
                let ch = if on_in {
                    '●'
                } else if on_out {
                    '○'
                } else if in_fill && i != fill_lo.unwrap_or(u16::MAX) && i != fill_hi.unwrap_or(u16::MAX) {
                    '━'
                } else {
                    '─'
                };
                let fg = if in_fill || on_in || on_out {
                    theme::RED
                } else {
                    theme::FG_GHOST
                };
                if let Some(cell) = frame.buffer_mut().cell_mut((bar.x + i, bar.y)) {
                    cell.set_char(ch);
                    cell.fg = fg;
                    cell.bg = theme::BG;
                }
            }
            if cols[1].height > 2 {
                let live = if app.focus == Focus::TrimOut {
                    out
                } else {
                    inn
                };
                let caption = slider_caption(
                    w as usize,
                    &format_timestamp(view),
                    &format_timestamp(live),
                    &format_timestamp(view_end.min(dur)),
                );
                frame.render_widget(
                    Paragraph::new(caption).style(theme::ghost()),
                    Rect {
                        x: cols[1].x,
                        y: cols[1].y.saturating_add(2),
                        width: cols[1].width,
                        height: 1,
                    },
                );
            }
        } else {
            frame.render_widget(
                Paragraph::new("PASTE URL TO CUT").style(theme::ghost()),
                cols[1],
            );
        }
    }
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

fn draw_right(frame: &mut Frame, area: Rect, app: &mut App) {
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(3),
    ])
    .split(area);

    app.hits.playlist = rows[0];
    draw_labeled(
        frame,
        rows[0],
        "PLAYLIST",
        app.focus == Focus::Playlist,
        if app.playlist { "YES" } else { "NO" },
    );

    app.hits.output = rows[1];
    draw_input(
        frame,
        rows[1],
        "OUTPUT",
        &app.output,
        app.focus == Focus::Output,
        "directory",
    );

    let fire = panel("ACTION", false);
    let inner = fire.inner(rows[2]);
    frame.render_widget(fire, rows[2]);
    let action = match app.phase {
        Phase::Extracting => "LIVE LINK",
        Phase::Probing => "LOCKING",
        Phase::Done => "LOCKED",
        Phase::Failed => "FAULT",
        _ => "F6 EXTRACT",
    };
    let sty = if app.phase == Phase::Done {
        theme::lock()
    } else {
        theme::accent_bold()
    };
    frame.render_widget(
        Paragraph::new(action)
            .style(sty)
            .alignment(Alignment::Center),
        inner,
    );
}

fn draw_choice(frame: &mut Frame, area: Rect, title: &str, focused: bool, items: &[(&str, bool)]) {
    let block = panel(title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if items.is_empty() || inner.width == 0 {
        return;
    }
    let selected = items.iter().position(|(_, on)| *on).unwrap_or(0);
    let chips: Vec<(String, bool)> = items
        .iter()
        .map(|(label, on)| {
            let text = if *on {
                format!("[{label}]")
            } else {
                (*label).to_string()
            };
            (text, *on)
        })
        .collect();
    let widths: Vec<usize> = chips.iter().map(|(t, _)| t.len()).collect();
    let width = inner.width as usize;
    let (mut lo, hi) = util::choice_window(&widths, selected, width.saturating_sub(2));
    let mut spans = Vec::new();
    if lo > 0 {
        spans.push(Span::styled("<", theme::accent()));
        if lo == hi && widths.get(lo).copied().unwrap_or(0) + 2 > width {
            lo = selected;
        }
    } else {
        spans.push(Span::raw(" "));
    }
    for i in lo..=hi {
        if i > lo {
            spans.push(Span::raw(" "));
        }
        let (text, on) = &chips[i];
        let sty = if *on {
            Style::new()
                .fg(theme::BG)
                .bg(theme::RED)
                .add_modifier(Modifier::BOLD)
        } else {
            theme::dim()
        };
        spans.push(Span::styled(text.clone(), sty));
    }
    if hi + 1 < chips.len() {
        spans.push(Span::styled(">", theme::accent()));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), inner);
}

fn draw_labeled(frame: &mut Frame, area: Rect, title: &str, focused: bool, value: &str) {
    let block = panel(title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let sty = if focused {
        theme::accent_bold()
    } else {
        theme::bold()
    };
    frame.render_widget(Paragraph::new(value).style(sty), inner);
}

fn draw_input(frame: &mut Frame, area: Rect, title: &str, field: &Field, focused: bool, hint: &str) {
    let block = panel(title, focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if field.is_empty() && !focused {
        frame.render_widget(Paragraph::new(hint).style(theme::ghost()), inner);
        return;
    }
    let width = inner.width.max(1) as usize;
    frame.render_widget(Paragraph::new(field_line(field, width, focused)), inner);
    if focused && inner.width > 0 {
        let (_, cur_x) = field.visible(width);
        frame.set_cursor_position(Position::new(
            inner.x + cur_x as u16,
            inner.y,
        ));
    }
}

fn field_line(field: &Field, width: usize, focused: bool) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }
    let (start, cur) = field.visible(width);
    let chars = field.chars();
    let mut spans = Vec::new();
    for i in 0..width {
        let idx = start + i;
        let ch = if idx < chars.len() { chars[idx] } else { ' ' };
        let is_cur = focused && i == cur;
        let sty = if is_cur {
            Style::new().fg(theme::BG).bg(theme::RED)
        } else if focused {
            theme::input(true)
        } else {
            theme::root()
        };
        spans.push(Span::styled(ch.to_string(), sty));
    }
    Line::from(spans)
}

fn draw_telemetry(frame: &mut Frame, area: Rect, app: &App) {
    let block = panel("TELEMETRY", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if app.logs.is_empty() {
        frame.render_widget(Paragraph::new("SILENT").style(theme::ghost()), inner);
        return;
    }
    let h = inner.height as usize;
    let start = app.logs.len().saturating_sub(h);
    let lines: Vec<Line> = app
        .logs
        .iter()
        .skip(start)
        .map(|l| {
            let sty = if l.contains("ERROR") || l.contains("FAULT") || l.contains("MISSING") {
                theme::accent()
            } else {
                theme::dim()
            };
            Line::from(Span::styled(
                util::truncate(l, inner.width as usize),
                sty,
            ))
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_progress(frame: &mut Frame, area: Rect, app: &mut App) {
    let live = app.phase == Phase::Extracting;
    let done = app.phase == Phase::Done;
    let failed = app.phase == Phase::Failed;
    let title = if live {
        "LINK"
    } else if done {
        "LOCK"
    } else if failed {
        "FAULT"
    } else {
        "METER"
    };
    let block = panel(title, live);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 {
        return;
    }
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .split(inner);

    let (pct, skin) = match app.phase {
        Phase::Extracting => (app.progress.percent, MeterSkin::Live),
        Phase::Done => (100.0, MeterSkin::Ok),
        Phase::Failed => (100.0, MeterSkin::Fault),
        Phase::Probing => (((app.tick % 40) as f64) * 2.5, MeterSkin::Live),
        _ => (100.0, MeterSkin::Idle),
    };
    effects::render_plasma(frame.buffer_mut(), rows[0], pct, app.tick, skin);

    if live && rows[0].width > 0 {
        let lead = rows[0].x as f32
            + (app.progress.percent / 100.0 * f64::from(rows[0].width)) as f32;
        if app.tick % 2 == 0 {
            effects::spawn_sparks(
                &mut app.particles,
                &mut app.rng,
                lead,
                rows[0].y as f32,
                3,
            );
        }
    }

    let stamp = match app.phase {
        Phase::Extracting if app.progress.percent < 1.0 => app.progress.stage.label().to_string(),
        Phase::Extracting if app.progress.percent < 10.0 => {
            format!("{:.1}%", app.progress.percent)
        }
        Phase::Extracting => format!("{:.0}%", app.progress.percent.clamp(0.0, 100.0)),
        Phase::Done => "COMPLETE".into(),
        Phase::Failed => "ERROR".into(),
        _ => String::new(),
    };
    if !stamp.is_empty() {
        stamp_progress(frame.buffer_mut(), rows[0], &stamp, pct / 100.0, app.phase);
    }

    let meta = match app.phase {
        Phase::Extracting => format!(
            "{:.0}%  {}  ETA {}  {}",
            app.progress.percent.clamp(0.0, 100.0),
            if app.progress.speed.is_empty() {
                "—"
            } else {
                &app.progress.speed
            },
            if app.progress.eta.is_empty() {
                "—"
            } else {
                &app.progress.eta
            },
            app.progress.total
        ),
        Phase::Done => app
            .last_path
            .as_ref()
            .map(|p| format!("COMPLETE  {}", p.display()))
            .unwrap_or_else(|| "COMPLETE".into()),
        Phase::Failed => app
            .last_error
            .clone()
            .unwrap_or_else(|| "ERROR".into()),
        Phase::Probing => "LOCKING TARGET".into(),
        _ => "STANDBY".into(),
    };
    frame.render_widget(Paragraph::new(meta).style(theme::dim()), rows[1]);
    if rows.len() > 2 {
        effects::render_waveform(
            frame.buffer_mut(),
            rows[2],
            &app.speed_hist,
            app.tick,
            live,
        );
    }
}

fn stamp_progress(buf: &mut ratatui::buffer::Buffer, area: Rect, text: &str, fill: f64, phase: Phase) {
    if area.width == 0 || text.is_empty() {
        return;
    }
    let chars: Vec<char> = text.chars().collect();
    let w = chars.len() as u16;
    let start = area.x + area.width.saturating_sub(w) / 2;
    let lead = area.x + (fill.clamp(0.0, 1.0) * f64::from(area.width)).round() as u16;
    for (i, ch) in chars.into_iter().enumerate() {
        let x = start.saturating_add(i as u16);
        if x >= area.x.saturating_add(area.width) {
            break;
        }
        let Some(cell) = buf.cell_mut((x, area.y)) else {
            continue;
        };
        cell.set_char(ch);
        match phase {
            Phase::Done => {
                cell.fg = theme::BG;
                cell.bg = theme::GREEN;
            }
            Phase::Failed => {
                cell.fg = theme::WHITE;
                cell.bg = theme::RED;
            }
            _ => {
                if x < lead {
                    cell.fg = theme::WHITE;
                    cell.bg = theme::RED;
                } else {
                    cell.fg = theme::RED;
                    cell.bg = theme::BG;
                }
            }
        }
    }
}

fn draw_keys(frame: &mut Frame, area: Rect, app: &App) {
    let text = if app.phase == Phase::Extracting {
        " ESC ABORT   Q QUIT   /// LIVE DATA LINK /// "
    } else {
        if app.update_available.is_some() {
            " U UPDATE  TAB FOCUS  ENTER LOCK  F5 LOCK  F6 EXTRACT  M TYPE  [ ] TRIM  ? HELP  Q QUIT "
        } else {
            " TAB FOCUS  ENTER LOCK  F5 LOCK  F6 EXTRACT  M TYPE  [ ] TRIM  ? HELP  Q QUIT "
        }
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(theme::ghost())
            .alignment(Alignment::Center),
        area,
    );
}

fn draw_boot(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered(area, 64, 16);
    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .border_style(theme::accent())
        .title(" BIOS ")
        .title_style(theme::accent_bold())
        .style(theme::root());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let elapsed = app.started.elapsed().as_millis();
    let mut lines = vec![
        Line::from(Span::styled("N I T R A T E", theme::accent_bold())),
        Line::from(Span::styled("VIDEO EXTRACTION CONSOLE", theme::bold())),
        Line::from(""),
    ];
    let checks = [
        (200, "PHOSPHOR DRIVER", true),
        (400, "CROSSTERM BACKEND", true),
        (700, "YT-DLP BRIDGE", app.tools.ytdlp.is_some()),
        (1000, "FFMPEG BRIDGE", app.tools.ffmpeg.is_some()),
        (1300, "AWAITING TARGET URL", true),
    ];
    for (at, name, ok) in checks {
        if elapsed >= at {
            let mark = if ok { "OK" } else { "FAIL" };
            let sty = if ok { theme::dim() } else { theme::accent() };
            lines.push(Line::from(vec![
                Span::styled(format!("{name:<22}"), theme::root()),
                Span::styled(".... ", theme::ghost()),
                Span::styled(mark, sty),
            ]));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("PRESS ANY KEY", theme::ghost())));
    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let popup = centered(area, 72, 21);
    frame.render_widget(Clear, popup);
    let block = Block::bordered()
        .border_style(theme::accent())
        .title(" MANUAL ")
        .title_style(theme::accent_bold())
        .style(theme::root());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let text = vec![
        Line::from(Span::styled("NITRATE  /  YOUTUBE  X  FACEBOOK", theme::accent_bold())),
        Line::from(""),
        Line::from("F5 / ENTER ON URL     LOCK metadata via yt-dlp"),
        Line::from("F6 / DOWNLOAD          start extract with quality + trim"),
        Line::from("TAB / SHIFT-TAB       cycle focus"),
        Line::from("M  TYPE               VIDEO or AUDIO"),
        Line::from("← → on QUALITY        MAX..360P or BEST..64K"),
        Line::from("← → on FORMAT         MP4 MKV WEBM MOV / MP3 M4A OPUS FLAC WAV OGG AAC"),
        Line::from("[ ] { }               nudge trim 1s / 5s"),
        Line::from("DRAG ● / ○            1 second per cell   RIGHT-CLICK sets OUT"),
        Line::from("CTRL-U / CTRL-W       clear field / kill word"),
        Line::from("U / nitrate update    apply latest GitHub release"),
        Line::from("ESC                   abort live job"),
        Line::from("Q / CTRL-C            quit"),
        Line::from(""),
        Line::from(Span::styled(
            "TRIM uses yt-dlp --download-sections. ffmpeg required to merge.",
            theme::ghost(),
        )),
    ];
    frame.render_widget(Paragraph::new(text), inner);
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

    #[test]
    fn boot_renders_title() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        terminal
            .draw(|frame| draw(frame, &mut app))
            .unwrap();
        let text = dump(terminal.backend().buffer());
        assert!(text.contains("NITRATE"), "{text}");
        assert!(text.contains("BIOS") || text.contains("VIDEO"), "{text}");
    }

    #[test]
    fn console_renders_after_boot() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.skip_boot();
        terminal
            .draw(|frame| draw(frame, &mut app))
            .unwrap();
        let text = dump(terminal.backend().buffer());
        assert!(text.contains("NITRATE"), "{text}");
        assert!(text.contains("URL"), "{text}");
        assert!(text.contains("IDENT"), "{text}");
        assert!(text.contains("TYPE"), "{text}");
        assert!(text.contains("FORMAT"), "{text}");
        assert!(text.contains("QUALITY"), "{text}");
        assert!(text.contains("DOWNLOAD"), "{text}");
        assert!(!text.contains("SIGNAL"), "{text}");
        assert!(!text.contains("COOKIES"), "{text}");
    }

    #[test]
    fn ident_shows_signal_title() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.skip_boot();
        app.info = Some(crate::engine::VideoInfo {
            id: "abc".into(),
            title: "A VERY LONG VIDEO TITLE THAT MUST SLIDE IN IDENT".into(),
            duration: Some(90.0),
            extractor: "Youtube".into(),
            webpage_url: "https://www.youtube.com/watch?v=abcdefghijklmnop".into(),
            platform: crate::util::Platform::YouTube,
        });
        terminal
            .draw(|frame| draw(frame, &mut app))
            .unwrap();
        let text = dump(terminal.backend().buffer());
        assert!(text.contains("TTL"), "{text}");
        assert!(text.contains("IDENT"), "{text}");
        assert!(text.contains("abc"), "{text}");
        assert!(text.contains("[MAX]"), "{text}");
    }

    #[test]
    fn progress_bar_shows_percent_complete_and_error() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.skip_boot();

        app.phase = Phase::Extracting;
        app.progress.percent = 45.0;
        terminal
            .draw(|frame| draw(frame, &mut app))
            .unwrap();
        let text = dump(terminal.backend().buffer());
        assert!(text.contains("45%"), "{text}");

        app.phase = Phase::Done;
        app.progress.percent = 100.0;
        terminal
            .draw(|frame| draw(frame, &mut app))
            .unwrap();
        let text = dump(terminal.backend().buffer());
        assert!(text.contains("COMPLETE"), "{text}");

        app.phase = Phase::Failed;
        terminal
            .draw(|frame| draw(frame, &mut app))
            .unwrap();
        let text = dump(terminal.backend().buffer());
        assert!(text.contains("ERROR"), "{text}");
    }

    #[test]
    fn trim_slider_shows_after_url_without_probe() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new();
        app.skip_boot();
        app.url.set("https://youtu.be/abc");
        terminal
            .draw(|frame| draw(frame, &mut app))
            .unwrap();
        let text = dump(terminal.backend().buffer());
        assert!(text.contains("TRIM"), "{text}");
        assert!(!text.contains("PASTE URL TO CUT"), "{text}");
        assert!(!text.contains("LOCK TARGET"), "{text}");
        assert!(app.trim_range().is_some());
    }
}
