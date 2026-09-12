use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    YouTube,
    X,
    Facebook,
    Unknown,
}

impl Platform {
    pub fn label(self) -> &'static str {
        match self {
            Self::YouTube => "YOUTUBE",
            Self::X => "X",
            Self::Facebook => "FACEBOOK",
            Self::Unknown => "GENERIC",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Field {
    chars: Vec<char>,
    cursor: usize,
}

impl Field {
    pub fn from_str(s: &str) -> Self {
        let chars: Vec<char> = s.chars().collect();
        let cursor = chars.len();
        Self { chars, cursor }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor.min(self.chars.len())
    }

    pub fn insert(&mut self, c: char) {
        let i = self.cursor();
        self.chars.insert(i, c);
        self.cursor = i + 1;
    }

    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' || c == '\r' || c == '\t' {
                continue;
            }
            self.insert(c);
        }
    }

    pub fn backspace(&mut self) {
        let i = self.cursor();
        if i == 0 {
            return;
        }
        self.chars.remove(i - 1);
        self.cursor = i - 1;
    }

    pub fn delete(&mut self) {
        let i = self.cursor();
        if i < self.chars.len() {
            self.chars.remove(i);
        }
    }

    pub fn left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.chars.len();
    }

    pub fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
    }

    pub fn set(&mut self, s: &str) {
        self.chars = s.chars().collect();
        self.cursor = self.chars.len();
    }

    pub fn kill_word(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut i = self.cursor();
        while i > 0 && self.chars[i - 1].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !self.chars[i - 1].is_whitespace() {
            i -= 1;
        }
        self.chars.drain(i..self.cursor());
        self.cursor = i;
    }

    pub fn visible(&self, width: usize) -> (usize, usize) {
        if width == 0 {
            return (0, 0);
        }
        let cursor = self.cursor();
        let start = if cursor + 1 > width {
            cursor + 1 - width
        } else {
            0
        };
        (start, cursor.saturating_sub(start))
    }

    pub fn chars(&self) -> &[char] {
        &self.chars
    }
}

pub fn detect_platform(url: &str) -> Platform {
    let host = url_host(url);
    if host.contains("youtube.com")
        || host.contains("youtu.be")
        || host.contains("youtube-nocookie.com")
    {
        Platform::YouTube
    } else if host.contains("twitter.com") || host == "x.com" || host.ends_with(".x.com") {
        Platform::X
    } else if host.contains("facebook.com")
        || host.contains("fb.watch")
        || host.contains("fb.com")
        || host.contains("fbcdn.net")
    {
        Platform::Facebook
    } else {
        Platform::Unknown
    }
}

fn url_host(url: &str) -> String {
    let url = url.trim();
    let rest = url
        .split_once("://")
        .map(|(_, r)| r)
        .unwrap_or(url);
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .trim()
        .trim_start_matches("www.")
        .to_ascii_lowercase()
}

pub fn parse_timestamp(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if s.eq_ignore_ascii_case("end") || s.eq_ignore_ascii_case("inf") {
        return Some(f64::INFINITY);
    }
    let parts: Vec<&str> = s.split(':').collect();
    match parts.as_slice() {
        [sec] => sec.parse().ok(),
        [m, sec] => {
            let m: f64 = m.parse().ok()?;
            let sec: f64 = sec.parse().ok()?;
            Some(m * 60.0 + sec)
        }
        [h, m, sec] => {
            let h: f64 = h.parse().ok()?;
            let m: f64 = m.parse().ok()?;
            let sec: f64 = sec.parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + sec)
        }
        _ => None,
    }
}

pub fn format_timestamp(secs: f64) -> String {
    if !secs.is_finite() {
        return "END".into();
    }
    let s = secs.max(0.0).round() as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{h}:{m:02}:{sec:02}")
    } else {
        format!("{m}:{sec:02}")
    }
}

pub fn trim_span(width: u16) -> u16 {
    width.saturating_sub(1).max(1)
}

pub fn trim_x(t: f64, view: f64, width: u16) -> Option<u16> {
    if width == 0 {
        return None;
    }
    let rel = t.round() - view.round();
    if rel < 0.0 || rel > f64::from(trim_span(width)) {
        None
    } else {
        Some(rel as u16)
    }
}

pub fn trim_t(x: u16, view: f64, dur: f64) -> f64 {
    (view.round() + f64::from(x)).round().clamp(0.0, dur.max(0.0))
}


pub fn format_mission(elapsed: std::time::Duration) -> String {
    let s = elapsed.as_secs();
    format!("T+{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

pub fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let mut w = 0usize;
    let mut out = String::new();
    for c in s.chars() {
        let cw = UnicodeWidthChar::width(c).unwrap_or(1).max(1);
        if w + cw > max {
            break;
        }
        out.push(c);
        w += cw;
    }
    out
}

pub fn marquee(text: &str, width: usize, tick: u64) -> String {
    if width == 0 {
        return String::new();
    }
    let units: Vec<(char, usize)> = text
        .chars()
        .map(|c| (c, UnicodeWidthChar::width(c).unwrap_or(1).max(1)))
        .collect();
    let text_w: usize = units.iter().map(|(_, w)| *w).sum();
    if text_w <= width {
        return truncate(text, width);
    }
    let mut looped = units;
    looped.extend([(' ', 1), (' ', 1), (' ', 1)]);
    let loop_w: usize = looped.iter().map(|(_, w)| *w).sum();
    let offset = ((tick / 4) as usize) % loop_w.max(1);
    let mut skipped = 0usize;
    let mut used = 0usize;
    let mut out = String::new();
    for &(c, cw) in looped.iter().chain(looped.iter()) {
        if skipped < offset {
            skipped += cw;
            continue;
        }
        if used + cw > width {
            break;
        }
        out.push(c);
        used += cw;
    }
    out
}


pub fn choice_window(chip_widths: &[usize], selected: usize, width: usize) -> (usize, usize) {
    if chip_widths.is_empty() {
        return (0, 0);
    }
    let selected = selected.min(chip_widths.len() - 1);
    let mut lo = selected;
    let mut hi = selected;
    let mut used = chip_widths[selected];
    loop {
        let mut grew = false;
        if lo > 0 {
            let extra = chip_widths[lo - 1] + 1;
            if used + extra <= width {
                lo -= 1;
                used += extra;
                grew = true;
            }
        }
        if hi + 1 < chip_widths.len() {
            let extra = chip_widths[hi + 1] + 1;
            if used + extra <= width {
                hi += 1;
                used += extra;
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    (lo, hi)
}

pub fn parse_yt_size(s: &str) -> Option<u64> {
    let s = s.trim().trim_end_matches("/s");
    if s.is_empty() || s.eq_ignore_ascii_case("unknown") {
        return None;
    }
    let n: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if n.is_empty() {
        return None;
    }
    let val: f64 = n.parse().ok()?;
    let rest = s[n.len()..].trim().to_ascii_uppercase();
    let mul = if rest.is_empty() || rest == "B" {
        1.0
    } else if rest.starts_with("KI") || rest == "KB" || rest == "K" {
        1024.0
    } else if rest.starts_with("MI") || rest == "MB" || rest == "M" {
        1024.0 * 1024.0
    } else if rest.starts_with("GI") || rest == "GB" || rest == "G" {
        1024.0 * 1024.0 * 1024.0
    } else {
        1.0
    };
    Some((val * mul) as u64)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_youtube() {
        assert_eq!(
            detect_platform("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Platform::YouTube
        );
        assert_eq!(
            detect_platform("https://youtu.be/dQw4w9WgXcQ"),
            Platform::YouTube
        );
    }

    #[test]
    fn platform_x_and_facebook() {
        assert_eq!(
            detect_platform("https://x.com/user/status/123"),
            Platform::X
        );
        assert_eq!(
            detect_platform("https://twitter.com/user/status/123"),
            Platform::X
        );
        assert_eq!(
            detect_platform("https://www.facebook.com/watch/?v=1"),
            Platform::Facebook
        );
        assert_eq!(detect_platform("https://fb.watch/abc"), Platform::Facebook);
    }

    #[test]
    fn timestamps() {
        assert_eq!(parse_timestamp("90"), Some(90.0));
        assert_eq!(parse_timestamp("1:30"), Some(90.0));
        assert_eq!(parse_timestamp("01:01:05"), Some(3665.0));
        assert_eq!(parse_timestamp("end"), Some(f64::INFINITY));
        assert_eq!(format_timestamp(90.0), "1:30");
        assert_eq!(format_timestamp(3665.0), "1:01:05");
    }

    #[test]
    fn trim_maps_one_cell_to_one_second() {
        assert_eq!(trim_span(21), 20);
        assert_eq!(trim_x(0.0, 0.0, 21), Some(0));
        assert_eq!(trim_x(20.0, 0.0, 21), Some(20));
        assert_eq!(trim_x(21.0, 0.0, 21), None);
        assert_eq!(trim_x(50.0, 40.0, 21), Some(10));
        assert_eq!(trim_t(0, 0.0, 100.0), 0.0);
        assert_eq!(trim_t(10, 40.0, 100.0), 50.0);
        for x in 0..21u16 {
            let t = trim_t(x, 0.0, 100.0);
            assert_eq!(trim_x(t, 0.0, 21), Some(x), "x={x} t={t}");
        }
    }

    #[test]
    fn marquee_slides_overflow_and_holds_short_text() {
        assert_eq!(marquee("ab", 4, 99), "ab");
        assert_eq!(marquee("abcdefghij", 4, 0), "abcd");
        assert_eq!(marquee("abcdefghij", 4, 4), "bcde");
        assert_eq!(marquee("", 3, 8), "");
        assert_eq!(marquee("xyz", 0, 1), "");
    }

    #[test]
    fn sizes() {
        assert_eq!(parse_yt_size("54.21MiB"), Some((54.21 * 1024.0 * 1024.0) as u64));
        assert_eq!(parse_yt_size("234.56KiB/s"), Some(240189));
    }

    #[test]
    fn field_edit() {
        let mut f = Field::from_str("ab");
        f.home();
        f.insert('z');
        assert_eq!(f.text(), "zab");
        f.end();
        f.backspace();
        assert_eq!(f.text(), "za");
        f.insert_str(" https://x.com/a ");
        assert!(f.text().contains("https://x.com/a"));
    }


    #[test]
    fn choice_window_keeps_selection() {
        let w = [4, 4, 4, 4, 4];
        assert_eq!(choice_window(&w, 0, 9), (0, 1));
        assert_eq!(choice_window(&w, 4, 9), (3, 4));
        assert_eq!(choice_window(&w, 2, 14), (1, 3));
        let (lo, hi) = choice_window(&w, 2, 4);
        assert_eq!((lo, hi), (2, 2));
    }
}
