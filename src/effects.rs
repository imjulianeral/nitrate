use crate::theme;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;


#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: u16,
    pub ch: char,
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn f32(&mut self) -> f32 {
        (self.next_u64() as f32) / (u64::MAX as f32)
    }

    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    pub fn range_u16(&mut self, lo: u16, hi: u16) -> u16 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() as u16) % (hi - lo)
    }
}

pub fn step_particles(particles: &mut Vec<Particle>, bounds: Rect) {
    for p in particles.iter_mut() {
        p.x += p.vx;
        p.y += p.vy;
        p.vy += 0.04;
        p.life = p.life.saturating_sub(1);
    }
    particles.retain(|p| {
        p.life > 0
            && p.x >= bounds.x as f32
            && p.y >= bounds.y as f32
            && p.x < (bounds.x + bounds.width) as f32
            && p.y < (bounds.y + bounds.height) as f32
    });
}

pub fn spawn_sparks(
    particles: &mut Vec<Particle>,
    rng: &mut Rng,
    x: f32,
    y: f32,
    n: usize,
) {
    const CHARS: [char; 4] = ['·', '+', '*', 'o'];
    for _ in 0..n {
        if particles.len() >= 120 {
            break;
        }
        particles.push(Particle {
            x: x + rng.range_f32(-0.4, 0.4),
            y: y + rng.range_f32(-0.2, 0.2),
            vx: rng.range_f32(-0.35, 0.55),
            vy: rng.range_f32(-0.45, 0.05),
            life: rng.range_u16(8, 28),
            ch: CHARS[(rng.next_u64() as usize) % CHARS.len()],
        });
    }
}

pub fn render_particles(buf: &mut Buffer, particles: &[Particle]) {
    for p in particles {
        let x = p.x.round() as u16;
        let y = p.y.round() as u16;
        if let Some(cell) = buf.cell_mut((x, y)) {
            if cell.symbol() == " " {
                cell.set_char(p.ch);
            }
            cell.fg = if p.life > 14 {
                theme::WHITE
            } else {
                theme::RED
            };
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeterSkin {
    Idle,
    Live,
    Ok,
    Fault,
}

pub fn render_plasma(buf: &mut Buffer, area: Rect, pct: f64, tick: u64, skin: MeterSkin) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let pct = match skin {
        MeterSkin::Ok | MeterSkin::Fault => 1.0,
        _ => pct.clamp(0.0, 100.0) / 100.0,
    };
    let lead = (pct * f64::from(area.width)).round() as i32;
    let t = tick as f64;
    let live = skin == MeterSkin::Live;
    let (fill, fill_dim, fill_bg) = match skin {
        MeterSkin::Ok => (
            theme::GREEN,
            theme::dim_rgb(theme::GREEN, 70),
            theme::dim_rgb(theme::GREEN, 170),
        ),
        MeterSkin::Fault => (theme::RED, theme::RED_DIM, theme::RED_DEEP),
        _ => (theme::RED, theme::RED_DIM, theme::RED_DEEP),
    };
    for i in 0..area.width {
        let x = area.x + i;
        let filled = i32::from(i) < lead;
        let wave = ((f64::from(i) * 0.42 + t * 0.16).sin() + 1.0) * 0.5;
        let dist = (i32::from(i) - lead).unsigned_abs();
        let spark = live && dist < 2;
        let ch = if spark {
            '▓'
        } else if filled {
            if wave > 0.75 {
                '█'
            } else if wave > 0.45 {
                '▓'
            } else {
                '▒'
            }
        } else if live && wave > 0.92 {
            '░'
        } else {
            '─'
        };
        let fg = if spark {
            theme::WHITE
        } else if filled {
            if wave > 0.7 {
                fill
            } else {
                fill_dim
            }
        } else {
            theme::FG_GHOST
        };
        let bg = if filled { fill_bg } else { theme::BG };
        for dy in 0..area.height {
            if let Some(cell) = buf.cell_mut((x, area.y + dy)) {
                if dy == 0 || area.height == 1 {
                    cell.set_char(ch);
                    cell.fg = fg;
                    cell.bg = bg;
                } else {
                    cell.set_char(if filled { '▄' } else { ' ' });
                    cell.fg = theme::dim_rgb(fg, 40);
                    cell.bg = bg;
                }
            }
        }
    }
}

pub fn render_waveform(buf: &mut Buffer, area: Rect, samples: &[u64], tick: u64, live: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let max = samples.iter().copied().max().unwrap_or(1).max(1);
    for i in 0..area.width {
        let ch = if !samples.is_empty() {
            let idx = if samples.len() >= area.width as usize {
                samples.len() - area.width as usize + i as usize
            } else if (i as usize) < samples.len() {
                i as usize
            } else {
                usize::MAX
            };
            if idx < samples.len() {
                let n = ((samples[idx] as f64 / max as f64) * 8.0).round() as usize;
                BLOCKS[n.min(8)]
            } else if live {
                let wave = (f64::from(i) * 0.31 + tick as f64 * 0.11).sin();
                let n = ((wave + 1.0) * 3.5).round() as usize;
                BLOCKS[n.min(8)]
            } else {
                '▁'
            }
        } else if live {
            let wave = (f64::from(i) * 0.31 + tick as f64 * 0.11).sin();
            let n = ((wave + 1.0) * 3.5).round() as usize;
            BLOCKS[n.min(8)]
        } else {
            '▁'
        };
        if let Some(cell) = buf.cell_mut((area.x + i, area.y)) {
            cell.set_char(ch);
            cell.fg = if live { theme::RED } else { theme::FG_GHOST };
            cell.bg = theme::BG;
        }
    }
}

pub fn render_hazard(buf: &mut Buffer, area: Rect, tick: u64) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let off = (tick / 2) as u16;
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let on = ((x + off) / 2) % 2 == 0;
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(if on { '▀' } else { ' ' });
                cell.fg = theme::RED;
                cell.bg = if on { theme::RED_DEEP } else { theme::BG };
            }
        }
    }
}


