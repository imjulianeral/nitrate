use crate::theme;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

/// Cheap stable noise in 0..1 for (x, y, t).
pub fn hash(x: u32, y: u32, t: u32) -> f32 {
    let mut h = x
        .wrapping_mul(0x27d4_eb2d)
        ^ y.wrapping_mul(0x1656_67b1)
        ^ t.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h & 0xffff) as f32 / 65536.0
}

/// A VHS tracking band: a couple of rows roll down, torn sideways and
/// peppered with noise, the way a misaligned head looks on a CRT.
pub fn render_tracking(buf: &mut Buffer, area: Rect, tick: u64) {
    if area.width < 4 || area.height == 0 {
        return;
    }
    let span = i64::from(area.height) + 6;
    let head = (tick as i64 / 2) % span - 3;
    for band in 0..2i64 {
        let row = head + band;
        if row < 0 || row >= i64::from(area.height) {
            continue;
        }
        let y = area.y + row as u16;
        let t = tick as u32;
        let tear = 1 + (hash(row as u32, 7, t / 3) * 2.0) as u16;
        let x0 = area.x;
        let x1 = area.x + area.width;
        for x in (x0 + tear..x1).rev() {
            let src = buf[(x - tear, y)].clone();
            buf[(x, y)] = src;
        }
        for x in x0..x1 {
            let n = hash(u32::from(x), u32::from(y), t);
            let cell = &mut buf[(x, y)];
            if x < x0 + tear {
                cell.set_char(' ');
                cell.bg = theme::BG_DECK;
            }
            let lift = if band == 0 { 0.10 } else { 0.05 };
            cell.bg = theme::mix(cell.bg, theme::WHITE, lift * n);
            cell.fg = theme::mix(cell.fg, theme::WHITE, 0.25 * n);
            if n > 0.93 && cell.symbol() == " " {
                cell.set_char(if n > 0.97 { '▀' } else { '▄' });
                cell.fg = theme::mix(cell.bg, theme::WHITE, 0.35);
            }
        }
    }
}

/// Broadband static, for the moment the set warms up.
pub fn render_snow(buf: &mut Buffer, area: Rect, tick: u64, strength: f32) {
    const GRAIN: [char; 4] = [' ', '░', '▒', '▓'];
    let t = tick as u32;
    for y in area.y..area.y + area.height {
        // Each scanline breathes a little, like an untuned channel.
        let line = 0.6 + 0.4 * hash(0, u32::from(y), t / 2);
        for x in area.x..area.x + area.width {
            let n = hash(u32::from(x), u32::from(y), t) * line * strength;
            let cell = &mut buf[(x, y)];
            cell.set_char(GRAIN[((n * 4.0) as usize).min(3)]);
            let v = (40.0 + n * 150.0) as u8;
            cell.fg = Color::Rgb(v, v, v.saturating_add(6));
            cell.bg = Color::Rgb(12, 12, 14);
            cell.modifier = Modifier::empty();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Meter {
    Idle,
    Seek,
    Rec,
    /// Post-processing with a known completion.
    Work,
    /// Post-processing that reports nothing; sweep to show it is alive.
    Busy,
    Done,
    Fault,
}

/// Tape position meter with eighth-block precision.
pub fn render_meter(buf: &mut Buffer, area: Rect, pct: f64, tick: u64, meter: Meter) {
    const EIGHTHS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
    if area.width == 0 || area.height == 0 {
        return;
    }
    let track = theme::BG_ELEVATED;
    let fill = match meter {
        Meter::Done => theme::OK,
        Meter::Fault => theme::REC,
        Meter::Work | Meter::Busy => theme::VFD,
        _ => theme::AMBER,
    };
    let w = f64::from(area.width);
    let (from, to) = match meter {
        Meter::Idle => (0.0, 0.0),
        Meter::Done | Meter::Fault => (0.0, w),
        Meter::Rec | Meter::Work => (0.0, pct.clamp(0.0, 100.0) / 100.0 * w),
        Meter::Seek | Meter::Busy => {
            // A read head sweeping back and forth while the tape is cued.
            let span = (w * 0.18).max(3.0);
            let travel = (w - span).max(0.0);
            let phase = (tick as f64 * 0.035).sin() * 0.5 + 0.5;
            let a = phase * travel;
            (a, a + span)
        }
    };
    let y = area.y;
    for i in 0..area.width {
        let cx = f64::from(i);
        let cover = ((to - cx).min(1.0) - (from - cx).max(0.0)).clamp(0.0, 1.0);
        let lit = match meter {
            Meter::Rec | Meter::Work => {
                let wave = ((cx * 0.22 - tick as f64 * 0.12).sin() * 0.5 + 0.5) as f32;
                theme::mix(theme::scale(fill, 0.82), fill, wave)
            }
            _ => fill,
        };
        let cell = &mut buf[(area.x + i, y)];
        cell.modifier = Modifier::empty();
        if cover >= 1.0 {
            cell.set_char(' ');
            cell.bg = lit;
        } else if cover <= 0.0 {
            cell.set_char(' ');
            cell.bg = track;
        } else if from > cx {
            // Leading edge of the seek window: fill from the right.
            cell.set_char(EIGHTHS[((1.0 - cover) * 8.0).round().clamp(0.0, 7.0) as usize]);
            cell.fg = track;
            cell.bg = lit;
        } else {
            cell.set_char(EIGHTHS[(cover * 8.0).round().clamp(0.0, 7.0) as usize]);
            cell.fg = lit;
            cell.bg = track;
        }
    }
}

/// Stamp `text` centred on the meter, inverted where it crosses the fill.
pub fn stamp_meter(buf: &mut Buffer, area: Rect, text: &str, fill: f64) {
    if area.width == 0 || text.is_empty() {
        return;
    }
    let len = text.chars().count() as u16;
    let start = area.x + area.width.saturating_sub(len) / 2;
    let lead = area.x as f64 + fill.clamp(0.0, 1.0) * f64::from(area.width);
    for (i, ch) in text.chars().enumerate() {
        let x = start + i as u16;
        if x >= area.x + area.width {
            break;
        }
        let cell = &mut buf[(x, area.y)];
        let on_fill = f64::from(x) + 0.5 < lead;
        cell.set_char(ch);
        cell.modifier = Modifier::BOLD;
        if on_fill {
            cell.fg = theme::BG;
        } else {
            cell.fg = theme::FG;
            cell.bg = theme::BG_ELEVATED;
        }
    }
}

/// Transfer-rate history as a thin trace under the meter.
pub fn render_waveform(buf: &mut Buffer, area: Rect, samples: &[u64], live: bool) {
    const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if area.width == 0 || area.height == 0 {
        return;
    }
    let max = samples.iter().copied().max().unwrap_or(1).max(1);
    let w = area.width as usize;
    let n = samples.len();
    for i in 0..w {
        // Stretch the history over the full width, interpolating between
        // samples, so the trace spans the meter however few samples exist.
        let level = match n {
            0 => 1,
            1 => 8,
            _ => {
                let t = i as f64 / (w - 1).max(1) as f64 * (n - 1) as f64;
                let (a, frac) = (t.floor() as usize, t.fract());
                let b = (a + 1).min(n - 1);
                let v = samples[a] as f64 * (1.0 - frac) + samples[b] as f64 * frac;
                ((v / max as f64 * 8.0).round() as usize).max(1)
            }
        };
        let cell = &mut buf[(area.x + i as u16, area.y)];
        cell.set_char(BLOCKS[level.min(8)]);
        cell.fg = if live && n > 0 {
            theme::AMBER_DIM
        } else {
            theme::BORDER
        };
        cell.bg = theme::BG;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waveform_spans_full_width_with_few_samples() {
        let area = Rect::new(0, 0, 150, 1);
        let mut buf = Buffer::empty(area);
        let samples: Vec<u64> = (0..96).map(|i| 100 + (i % 7) * 40).collect();
        render_waveform(&mut buf, area, &samples, true);
        for x in 0..area.width {
            assert_eq!(buf[(x, 0)].fg, theme::AMBER_DIM, "column {x}");
        }
        let tail: Vec<&str> = (140..150).map(|x| buf[(x, 0)].symbol()).collect();
        assert!(tail.iter().any(|s| *s != "▁"), "{tail:?}");
    }
}
