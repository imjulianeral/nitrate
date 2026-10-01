use crate::util::Platform;
use ratatui::style::{Color, Modifier, Style};

// Warm, low-contrast base: a deck sitting in a dim living room.
pub const BG: Color = Color::Rgb(16, 15, 14);
pub const BG_DECK: Color = Color::Rgb(11, 11, 10);
pub const BG_ELEVATED: Color = Color::Rgb(25, 24, 22);
pub const BG_INPUT: Color = Color::Rgb(33, 28, 22);
pub const BG_SELECT: Color = Color::Rgb(34, 31, 27);
pub const FG: Color = Color::Rgb(230, 222, 206);
pub const FG_DIM: Color = Color::Rgb(146, 138, 124);
pub const FG_GHOST: Color = Color::Rgb(86, 81, 74);
pub const BORDER: Color = Color::Rgb(50, 47, 43);
pub const WHITE: Color = Color::Rgb(250, 246, 238);

// One accent for interaction, plus the three lamps every VCR has.
pub const AMBER: Color = Color::Rgb(232, 168, 84);
pub const AMBER_DIM: Color = Color::Rgb(122, 88, 46);
pub const REC: Color = Color::Rgb(222, 78, 62);
pub const REC_DEEP: Color = Color::Rgb(58, 28, 24);
pub const OK: Color = Color::Rgb(150, 196, 128);
pub const VFD: Color = Color::Rgb(134, 222, 206);

// Label stripes, the kind printed on 80s blank tapes.
pub const STRIPES: [Color; 4] = [
    Color::Rgb(186, 80, 52),
    Color::Rgb(220, 130, 58),
    Color::Rgb(226, 184, 88),
    Color::Rgb(84, 142, 132),
];

// Cassette materials.
pub const SHELL: Color = Color::Rgb(31, 30, 29);
pub const SHELL_HI: Color = Color::Rgb(47, 45, 43);
pub const SHELL_LO: Color = Color::Rgb(21, 20, 19);
pub const GLASS: Color = Color::Rgb(15, 15, 16);
pub const PACK: Color = Color::Rgb(52, 40, 33);
pub const PACK_HI: Color = Color::Rgb(86, 68, 54);
pub const HUB: Color = Color::Rgb(206, 200, 188);
pub const HUB_LO: Color = Color::Rgb(62, 58, 54);
pub const TAPE: Color = Color::Rgb(64, 48, 38);
pub const TAPE_HI: Color = Color::Rgb(112, 86, 64);
pub const PAPER: Color = Color::Rgb(226, 216, 194);
pub const INK: Color = Color::Rgb(42, 36, 30);
pub const INK_DIM: Color = Color::Rgb(122, 110, 94);

// The blue screen a VCR shows with no signal.
pub const BLUE: Color = Color::Rgb(28, 46, 128);

pub fn root() -> Style {
    Style::new().fg(FG).bg(BG)
}

pub fn dim() -> Style {
    Style::new().fg(FG_DIM).bg(BG)
}

pub fn ghost() -> Style {
    Style::new().fg(FG_GHOST).bg(BG)
}

pub fn accent() -> Style {
    Style::new().fg(AMBER).bg(BG)
}

pub fn accent_bold() -> Style {
    Style::new().fg(AMBER).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn bold() -> Style {
    Style::new().fg(FG).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn rec() -> Style {
    Style::new().fg(REC).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn ok() -> Style {
    Style::new().fg(OK).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn border(focused: bool) -> Style {
    if focused {
        Style::new().fg(AMBER_DIM).bg(BG)
    } else {
        Style::new().fg(BORDER).bg(BG)
    }
}

pub fn title(focused: bool) -> Style {
    if focused {
        Style::new().fg(AMBER).bg(BG).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(FG_DIM).bg(BG)
    }
}

pub fn input(focused: bool) -> Style {
    if focused {
        Style::new().fg(FG).bg(BG_INPUT)
    } else {
        Style::new().fg(FG).bg(BG_ELEVATED)
    }
}

pub fn platform(p: Platform) -> Color {
    match p {
        Platform::YouTube => STRIPES[0],
        Platform::X => Color::Rgb(150, 156, 166),
        Platform::Facebook => Color::Rgb(96, 130, 186),
        Platform::Unknown => STRIPES[2],
    }
}

pub fn rgb(c: Color) -> [f32; 3] {
    match c {
        Color::Rgb(r, g, b) => [f32::from(r), f32::from(g), f32::from(b)],
        _ => rgb(BG),
    }
}

pub fn from_rgb(c: [f32; 3]) -> Color {
    let q = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    Color::Rgb(q(c[0]), q(c[1]), q(c[2]))
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let (a, b) = (rgb(a), rgb(b));
    let t = t.clamp(0.0, 1.0);
    from_rgb([
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ])
}

pub fn scale(c: Color, k: f32) -> Color {
    let c = rgb(c);
    from_rgb([c[0] * k, c[1] * k, c[2] * k])
}
