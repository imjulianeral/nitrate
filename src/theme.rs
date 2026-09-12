use ratatui::style::{Color, Modifier, Style};

pub const BG: Color = Color::Rgb(10, 10, 10);
pub const BG_ELEVATED: Color = Color::Rgb(18, 18, 18);
pub const BG_INPUT: Color = Color::Rgb(24, 12, 12);
pub const FG: Color = Color::Rgb(234, 234, 234);
pub const FG_DIM: Color = Color::Rgb(118, 118, 118);
pub const FG_GHOST: Color = Color::Rgb(70, 70, 70);
pub const RED: Color = Color::Rgb(230, 25, 25);
pub const RED_DIM: Color = Color::Rgb(118, 18, 18);
pub const RED_DEEP: Color = Color::Rgb(48, 10, 10);
pub const GREEN: Color = Color::Rgb(74, 246, 38);
pub const WHITE: Color = Color::Rgb(255, 255, 255);
pub const BORDER: Color = Color::Rgb(52, 52, 52);

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
    Style::new().fg(RED).bg(BG)
}

pub fn accent_bold() -> Style {
    Style::new().fg(RED).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn bold() -> Style {
    Style::new().fg(FG).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn lock() -> Style {
    Style::new().fg(GREEN).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn border(focused: bool) -> Style {
    if focused {
        Style::new().fg(RED).bg(BG)
    } else {
        Style::new().fg(BORDER).bg(BG)
    }
}

pub fn title(focused: bool) -> Style {
    if focused {
        Style::new().fg(RED).bg(BG).add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(FG_DIM).bg(BG).add_modifier(Modifier::BOLD)
    }
}

pub fn input(focused: bool) -> Style {
    if focused {
        Style::new().fg(FG).bg(BG_INPUT)
    } else {
        Style::new().fg(FG).bg(BG_ELEVATED)
    }
}

pub fn dim_rgb(color: Color, amt: u8) -> Color {
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            r.saturating_sub(amt),
            g.saturating_sub(amt),
            b.saturating_sub(amt),
        ),
        Color::Reset => Color::Rgb(10, 10, 10),
        Color::White => Color::Rgb(
            234u8.saturating_sub(amt),
            234u8.saturating_sub(amt),
            234u8.saturating_sub(amt),
        ),
        other => other,
    }
}
