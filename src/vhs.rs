//! The cassette deck: tape transport state and a half-block pixel renderer.
//!
//! Every terminal cell holds two square-ish pixels (`▀` with fg = top, bg =
//! bottom), so circles stay round and the reels can turn smoothly.

use crate::theme;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::f32::consts::TAU;

/// Hub radius as a fraction of a full tape pack.
const HUB: f32 = 0.36;

#[derive(Clone, Copy, Debug, Default)]
pub struct Deck {
    /// Share of tape wound onto the take-up reel, 0..1.
    pub pos: f64,
    /// Hub angle of the supply and take-up reels, radians.
    pub spin: [f64; 2],
    /// Distance the exposed tape has travelled, pixels.
    pub travel: f64,
    /// Linear tape speed, eased toward the transport's target.
    pub speed: f64,
    /// 0 = ejected, 1 = seated in the deck.
    pub seated: f64,
}

impl Deck {
    pub fn step(&mut self, target_pos: f64, target_speed: f64, loaded: bool) {
        self.speed += (target_speed - self.speed) * 0.1;
        let before = self.pos;
        let target_pos = target_pos.clamp(0.0, 1.0);
        self.pos += (target_pos - self.pos) * 0.06;
        if (target_pos - self.pos).abs() < 1e-4 {
            self.pos = target_pos;
        }
        let v = self.speed + (self.pos - before) * 60.0;
        let radii = pack_radii(self.pos);
        for (spin, r) in self.spin.iter_mut().zip(radii) {
            // Constant linear speed means the emptier reel turns faster.
            let step = (v * 0.07 / f64::from(r)).clamp(-0.5, 0.5);
            *spin = (*spin + step).rem_euclid(std::f64::consts::TAU);
        }
        self.travel += v * 0.6;
        let seat = if loaded { 1.0 } else { 0.0 };
        self.seated += (seat - self.seated) * 0.14;
        if (self.seated - seat).abs() < 0.01 {
            self.seated = seat;
        }
    }
}

/// Normalised supply / take-up pack radii; tape area is conserved.
fn pack_radii(pos: f64) -> [f32; 2] {
    let p = pos.clamp(0.0, 1.0) as f32;
    let h2 = HUB * HUB;
    [
        (h2 + (1.0 - p) * (1.0 - h2)).sqrt(),
        (h2 + p * (1.0 - h2)).sqrt(),
    ]
}

pub struct Canvas {
    w: usize,
    h: usize,
    px: Vec<[f32; 3]>,
}

impl Canvas {
    pub fn new(cols: u16, rows: u16, fill: Color) -> Self {
        let (w, h) = (cols as usize, rows as usize * 2);
        Self {
            w,
            h,
            px: vec![theme::rgb(fill); w * h],
        }
    }

    /// Paint every pixel `f` covers, 3×3 supersampled for soft edges.
    pub fn shade(&mut self, f: impl Fn(f32, f32) -> Option<[f32; 3]>) {
        const S: [f32; 3] = [1.0 / 6.0, 0.5, 5.0 / 6.0];
        for y in 0..self.h {
            for x in 0..self.w {
                let base = self.px[y * self.w + x];
                let mut acc = [0.0f32; 3];
                let mut hit = 0;
                for sy in S {
                    for sx in S {
                        let c = f(x as f32 + sx, y as f32 + sy);
                        if c.is_some() {
                            hit += 1;
                        }
                        let c = c.unwrap_or(base);
                        acc[0] += c[0];
                        acc[1] += c[1];
                        acc[2] += c[2];
                    }
                }
                if hit > 0 {
                    self.px[y * self.w + x] = [acc[0] / 9.0, acc[1] / 9.0, acc[2] / 9.0];
                }
            }
        }
    }

    pub fn blit(&self, buf: &mut Buffer, area: Rect) {
        for row in 0..(self.h / 2).min(area.height as usize) {
            for x in 0..self.w.min(area.width as usize) {
                let top = theme::from_rgb(self.px[row * 2 * self.w + x]);
                let bot = theme::from_rgb(self.px[(row * 2 + 1) * self.w + x]);
                if let Some(cell) = buf.cell_mut((area.x + x as u16, area.y + row as u16)) {
                    if top == bot {
                        cell.set_char(' ');
                    } else {
                        cell.set_char('▀');
                    }
                    cell.fg = top;
                    cell.bg = bot;
                    cell.modifier = Modifier::empty();
                }
            }
        }
    }
}

/// Signed distance to a rounded rectangle; negative inside.
fn rrect(x: f32, y: f32, r: [f32; 4], radius: f32) -> f32 {
    let (cx, cy) = ((r[0] + r[2]) * 0.5, (r[1] + r[3]) * 0.5);
    let hx = (r[2] - r[0]) * 0.5 - radius;
    let hy = (r[3] - r[1]) * 0.5 - radius;
    let qx = (x - cx).abs() - hx;
    let qy = (y - cy).abs() - hy;
    let (ox, oy) = (qx.max(0.0), qy.max(0.0));
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - radius
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn mul3(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// Text printed on the cassette's paper label.
pub struct LabelText<'a> {
    pub title: &'a str,
    pub detail: &'a str,
    pub brand: &'a str,
}

/// Cassette layout in pixels, anchored so the label lands on whole cells.
struct Geo {
    w: f32,
    h: f32,
    label: [f32; 4],
    text_rows: u16,
    stripe_row: Option<u16>,
    stripe_rows: u16,
    stripe_x: f32,
    window: [f32; 4],
    reels: [(f32, f32); 2],
    pack_max: f32,
    strip_y: Option<f32>,
    guides: [f32; 2],
}

impl Geo {
    fn new(cols: u16, rows: u16) -> Self {
        let w = f32::from(cols);
        let h = f32::from(rows) * 2.0;
        let text_rows: u16 = if rows >= 12 { 2 } else { 1 };
        let stripe_rows: u16 = match rows {
            14.. => 2,
            8.. => 1,
            _ => 0,
        };
        let label_rows = text_rows + stripe_rows;
        let gap: u16 = if rows >= 11 { 1 } else { 0 };
        let bottom: u16 = if rows >= 9 { 2 } else { 1 };
        let label = [
            (w * 0.06).round(),
            2.0,
            (w * 0.94).round(),
            2.0 + f32::from(label_rows) * 2.0,
        ];
        let win_top = label[3] + f32::from(gap) * 2.0 + 1.0;
        let win_bot = (h - f32::from(bottom) * 2.0 - 0.5).max(win_top + 2.0);
        let window = [w * 0.15, win_top, w * 0.85, win_bot];
        let win_h = win_bot - win_top;
        let cy = win_top + win_h * 0.58;
        let pack_max = (w * 0.19).min(win_h * 0.95).max(2.0);
        Self {
            w,
            h,
            label,
            text_rows,
            stripe_row: (stripe_rows > 0).then_some(1 + text_rows),
            stripe_rows,
            stripe_x: label[0] + (label[2] - label[0]) * 0.56,
            window,
            reels: [(w * 0.315, cy), (w * 0.685, cy)],
            pack_max,
            strip_y: (rows >= 9).then_some(h - 2.5),
            guides: [w * 0.09, w * 0.91],
        }
    }

    fn pixel(&self, x: f32, y: f32, deck: &Deck) -> Option<[f32; 3]> {
        let shell = rrect(x, y, [0.0, 0.0, self.w, self.h], 2.2);
        if shell > 0.0 {
            return None;
        }

        // Paper label with printed stripes.
        if rrect(x, y, self.label, 0.8) < 0.0 {
            if let Some(row) = self.stripe_row {
                // One pixel per band, slanted like a racing stripe.
                let top = f32::from(row) * 2.0;
                let bands = f32::from(self.stripe_rows) * 2.0;
                let lean = self.stripe_x + (bands - (y - top)) * 0.9;
                if x >= lean && x < self.label[2] - 1.0 && y >= top && y < top + bands {
                    let band = ((y - top) as usize).min(bands as usize - 1);
                    let idx = band * theme::STRIPES.len() / bands as usize;
                    return Some(theme::rgb(theme::STRIPES[idx]));
                }
            }
            let edge = rrect(x, y, self.label, 0.8);
            let paper = theme::rgb(theme::PAPER);
            return Some(if edge > -0.6 { mul3(paper, 0.86) } else { paper });
        }

        // Smoked window with both reels behind it.
        let win = rrect(x, y, self.window, 1.6);
        if win < 0.0 {
            let radii = pack_radii(deck.pos);
            for (i, &(cx, cy)) in self.reels.iter().enumerate() {
                let (dx, dy) = (x - cx, y - cy);
                let d = (dx * dx + dy * dy).sqrt();
                let hub = self.pack_max * HUB;
                let pack = self.pack_max * radii[i];
                let a = dy.atan2(dx) - deck.spin[i] as f32;
                if d < hub {
                    return Some(hub_pixel(d / hub, a));
                }
                if d < pack {
                    let rings = 0.92 + 0.08 * (d * 2.3).sin();
                    let sheen = 1.0 + 0.05 * a.cos();
                    let base = if d > pack - 0.7 {
                        theme::rgb(theme::PACK_HI)
                    } else {
                        theme::rgb(theme::PACK)
                    };
                    return Some(mul3(base, rings * sheen));
                }
            }
            let glass = theme::rgb(theme::GLASS);
            let t = (x - self.window[0]) + (y - self.window[1]) * 1.3;
            let glare = t.rem_euclid(26.0);
            return Some(if glare < 1.6 || (4.0..4.8).contains(&glare) {
                lerp3(glass, theme::rgb(theme::SHELL_HI), 0.45)
            } else {
                glass
            });
        }
        if win < 0.9 {
            return Some(theme::rgb(theme::SHELL_LO));
        }

        // Exposed tape across the front edge, running between two guides.
        if let Some(sy) = self.strip_y {
            for gx in self.guides {
                let (dx, dy) = (x - gx, y - sy);
                if dx * dx + dy * dy < 1.4 {
                    return Some(theme::rgb(theme::HUB_LO));
                }
            }
            if (y - sy).abs() < 0.55 && x > self.guides[0] && x < self.guides[1] {
                let phase = (x - deck.travel as f32).rem_euclid(6.0);
                return Some(if phase < 1.0 {
                    theme::rgb(theme::TAPE_HI)
                } else {
                    theme::rgb(theme::TAPE)
                });
            }
        }

        // Grip ridges on either side of the window.
        let (wy0, wy1) = (self.window[1], self.window[3]);
        let side = x < self.window[0] - 1.5 && x > self.w * 0.045
            || x > self.window[2] + 1.5 && x < self.w * 0.955;
        if side && y > wy0 + 0.5 && y < wy1 - 0.5 && (x.floor() as i32) % 2 == 0 {
            return Some(theme::rgb(theme::SHELL_LO));
        }

        if shell > -0.8 {
            return Some(theme::rgb(theme::SHELL_HI));
        }
        let t = (y / self.h).clamp(0.0, 1.0);
        Some(lerp3(
            lerp3(theme::rgb(theme::SHELL), theme::rgb(theme::SHELL_HI), 0.35),
            theme::rgb(theme::SHELL),
            t,
        ))
    }
}

/// White plastic hub with three spokes; `r` is 0..1 across the hub.
fn hub_pixel(r: f32, angle: f32) -> [f32; 3] {
    let hub = theme::rgb(theme::HUB);
    let lo = theme::rgb(theme::HUB_LO);
    if r < 0.26 {
        return theme::rgb(theme::GLASS);
    }
    if r > 0.8 {
        return mul3(hub, 0.9);
    }
    let sector = (angle / (TAU / 3.0)).rem_euclid(1.0);
    if (sector - 0.5).abs() < 0.24 {
        lo
    } else {
        hub
    }
}

/// Size of the cassette that fits `area`, keeping a real VHS aspect ratio.
fn fit(area: Rect) -> (u16, u16) {
    const ASPECT: f32 = 1.8;
    let mut rows = area.height;
    let mut cols = (f32::from(rows) * 2.0 * ASPECT).round() as u16;
    if cols > area.width.saturating_sub(2) {
        cols = area.width.saturating_sub(2);
        rows = ((f32::from(cols) / ASPECT / 2.0).round() as u16).min(area.height);
    }
    (cols, rows)
}

pub fn render_cassette(buf: &mut Buffer, area: Rect, deck: &Deck, label: &LabelText, tick: u64) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    let (cols, rows) = fit(area);
    let ox = (area.width - cols) / 2;
    let rest = area.height - rows;
    let oy = rest / 2;
    // Slide in from below; whole cells so the label text stays aligned.
    let drop = ((1.0 - deck.seated) * f64::from(rows + rest - oy + 1)).round() as u16;
    let top = oy + drop;
    if top >= area.height {
        return;
    }
    let geo = Geo::new(cols, rows);
    let mut canvas = Canvas::new(area.width, area.height, theme::BG_DECK);
    let (fx, fy) = (f32::from(ox), f32::from(top) * 2.0);
    canvas.shade(|x, y| geo.pixel(x - fx, y - fy, deck));
    canvas.blit(buf, area);

    // Printed label text sits on whole cells of paper.
    let x0 = area.x + ox + geo.label[0] as u16 + 1;
    let x1 = area.x + ox + geo.label[2] as u16 - 1;
    let paper = |fg: Color| Style::new().fg(fg).bg(theme::PAPER);
    let put = |buf: &mut Buffer, row: u16, x: u16, max: u16, text: &str, style: Style| {
        let y = area.y + top + row;
        if y < area.y + area.height && max > 0 {
            buf.set_stringn(x, y, text, max as usize, style);
        }
    };
    let full = x1.saturating_sub(x0);
    let title = crate::util::marquee(label.title, full as usize, tick);
    put(
        buf,
        1,
        x0,
        full,
        &title,
        paper(theme::INK).add_modifier(Modifier::BOLD),
    );
    if geo.text_rows > 1 {
        put(buf, 2, x0, full, label.detail, paper(theme::INK_DIM));
    }
    if let Some(row) = geo.stripe_row {
        let stripe_x = area.x + ox + geo.stripe_x as u16;
        let room = stripe_x.saturating_sub(x0 + 1);
        put(
            buf,
            row,
            x0,
            room,
            label.brand,
            paper(theme::INK).add_modifier(Modifier::BOLD),
        );
    }
}

/// An empty deck: the loading slot with its flap closed.
pub fn render_slot(buf: &mut Buffer, area: Rect, tick: u64) {
    if area.width < 8 || area.height < 3 {
        return;
    }
    let w = f32::from(area.width);
    let h = f32::from(area.height) * 2.0;
    let slot_w = (w * 0.66).min(64.0);
    let x0 = ((w - slot_w) * 0.5).round();
    let y0 = (h * 0.5 - 4.0).round().max(0.0);
    let bezel = [x0, y0, x0 + slot_w, y0 + 6.0];
    let mouth = [x0 + 2.0, y0 + 2.0, x0 + slot_w - 2.0, y0 + 4.0];
    let mut canvas = Canvas::new(area.width, area.height, theme::BG_DECK);
    canvas.shade(|x, y| {
        if rrect(x, y, mouth, 0.6) < 0.0 {
            let flap = y < mouth[1] + 0.8;
            return Some(theme::rgb(if flap { theme::SHELL_HI } else { theme::GLASS }));
        }
        let d = rrect(x, y, bezel, 2.0);
        if d < 0.0 {
            return Some(theme::rgb(if d > -0.7 { theme::SHELL_HI } else { theme::SHELL }));
        }
        None
    });
    canvas.blit(buf, area);

    let text_y = area.y + ((y0 + 8.0) / 2.0).ceil() as u16;
    let blink = (tick / 30).is_multiple_of(2);
    let lines: [(&str, Style); 2] = [
        (
            "NO TAPE",
            Style::new()
                .fg(if blink { theme::FG_DIM } else { theme::FG_GHOST })
                .bg(theme::BG_DECK)
                .add_modifier(Modifier::BOLD),
        ),
        (
            "paste a video link to load a cassette",
            Style::new().fg(theme::FG_GHOST).bg(theme::BG_DECK),
        ),
    ];
    for (i, (text, style)) in lines.iter().enumerate() {
        let y = text_y + i as u16;
        if y >= area.y + area.height {
            break;
        }
        let len = text.chars().count() as u16;
        let x = area.x + area.width.saturating_sub(len) / 2;
        buf.set_stringn(x, y, text, area.width as usize, *style);
    }
}

/// Chunky pixel wordmark for the power-on screen.
pub fn render_wordmark(buf: &mut Buffer, area: Rect, word: &str, fg: Color, bg: Color) -> Rect {
    const GLYPHS: &[(char, [&str; 5])] = &[
        ('N', ["X..X", "XX.X", "X.XX", "X..X", "X..X"]),
        ('I', ["XXX", ".X.", ".X.", ".X.", "XXX"]),
        ('T', ["XXX", ".X.", ".X.", ".X.", ".X."]),
        ('R', ["XXX.", "X..X", "XXX.", "X.X.", "X..X"]),
        ('A', [".XX.", "X..X", "XXXX", "X..X", "X..X"]),
        ('E', ["XXXX", "X...", "XXX.", "X...", "XXXX"]),
    ];
    let glyphs: Vec<&[&str; 5]> = word
        .chars()
        .filter_map(|c| GLYPHS.iter().find(|(g, _)| *g == c).map(|(_, rows)| rows))
        .collect();
    let natural: usize = glyphs.iter().map(|g| g[0].len() + 1).sum::<usize>().saturating_sub(1);
    let scale = if natural * 2 + 2 <= area.width as usize && area.height >= 6 { 2 } else { 1 };
    let pw = natural * scale;
    let ph = 5 * scale;
    let cols = pw as u16;
    let rows = ph.div_ceil(2) as u16;
    if cols > area.width || rows > area.height {
        return Rect::default();
    }
    let rect = Rect {
        x: area.x + (area.width - cols) / 2,
        y: area.y,
        width: cols,
        height: rows,
    };
    let mut on = vec![false; pw * rows as usize * 2];
    let mut gx = 0;
    for g in &glyphs {
        for (ry, line) in g.iter().enumerate() {
            for (rx, ch) in line.chars().enumerate() {
                if ch == 'X' {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            on[(ry * scale + sy) * pw + (gx + rx) * scale + sx] = true;
                        }
                    }
                }
            }
        }
        gx += g[0].len() + 1;
    }
    let mut canvas = Canvas::new(cols, rows, bg);
    canvas.shade(|x, y| {
        let (ix, iy) = (x as usize, y as usize);
        on.get(iy * pw + ix)
            .copied()
            .unwrap_or(false)
            .then(|| theme::rgb(fg))
    });
    canvas.blit(buf, rect);
    rect
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_trade_tape_but_keep_area() {
        let [s0, t0] = pack_radii(0.0);
        let [s1, t1] = pack_radii(1.0);
        assert!((s0 - 1.0).abs() < 1e-5 && (t0 - HUB).abs() < 1e-5);
        assert!((s1 - HUB).abs() < 1e-5 && (t1 - 1.0).abs() < 1e-5);
        let [s, t] = pack_radii(0.5);
        assert!((s * s + t * t - (1.0 + HUB * HUB)).abs() < 1e-4);
    }

    #[test]
    fn reels_turn_only_while_tape_moves() {
        let mut deck = Deck::default();
        deck.step(0.0, 0.0, true);
        assert_eq!(deck.spin, [0.0, 0.0]);
        for _ in 0..30 {
            deck.step(0.0, 1.0, true);
        }
        assert!(deck.speed > 0.5);
        // The empty take-up reel spins faster than the full supply reel.
        assert!(deck.spin[1] != deck.spin[0]);
    }

    #[test]
    fn cassette_fits_small_areas() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 8));
        let deck = Deck {
            seated: 1.0,
            ..Deck::default()
        };
        let label = LabelText {
            title: "hello",
            detail: "world",
            brand: "NITRATE",
        };
        let area = buf.area;
        render_cassette(&mut buf, area, &deck, &label, 0);
        render_cassette(&mut buf, Rect::new(0, 0, 9, 3), &deck, &label, 0);
    }
}
