// ── ui.rs ── Render entry point + shared rendering utilities ──

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::app::App;
use crate::engine::characters::CharacterDef;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Main render entry point
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn render(frame: &mut Frame, app: &App) {
    crate::ui_broadcast::render(frame, app);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Shared utilities (used by all themes)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Format an i64 dollar amount with comma separators.
pub(crate) fn fmt_dollars(amount: i64) -> String {
    let mut s = amount.abs().to_string();
    let mut out = String::new();
    while s.len() > 3 {
        let tail = s.split_off(s.len() - 3);
        if out.is_empty() {
            out = tail;
        } else {
            out = format!("{},{}", tail, out);
        }
    }
    if out.is_empty() {
        out = s;
    } else if !s.is_empty() {
        out = format!("{},{}", s, out);
    }
    if amount < 0 { format!("-{}", out) } else { out }
}

/// Truncate a string to `max` chars, appending "..." if truncated.
/// Split text so each sentence starts on its own line.
pub(crate) fn sentences_to_lines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut first = true;
    for sentence in s.split(". ") {
        if !first {
            out.push('\n');
        }
        out.push_str(sentence.trim());
        if !sentence.ends_with('.') && !sentence.ends_with('!') && !sentence.ends_with('?') {
            out.push('.');
        }
        first = false;
    }
    out
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    if max < 4 {
        return s[..max].to_string();
    }
    format!("{}...", &s[..max - 3])
}

/// Center a rect of given size within an area.
pub(crate) fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

/// Repeat a character `n` times.
pub(crate) fn repeat_char(ch: char, n: u16) -> String {
    std::iter::repeat_n(ch, n as usize).collect()
}

/// Build a colored bar: "████░░░░" with `filled` of one color, rest of another.
pub(crate) fn bar_spans(
    value: i32,
    width: usize,
    fill_color: Color,
    empty_color: Color,
    bg: Color,
) -> Vec<Span<'static>> {
    let v = value.clamp(0, 100);
    let filled = (v as usize * width) / 100;
    let empty = width.saturating_sub(filled);
    vec![
        Span::styled("█".repeat(filled), Style::default().fg(fill_color).bg(bg)),
        Span::styled("░".repeat(empty), Style::default().fg(empty_color).bg(bg)),
    ]
}

/// Small item sprite for a choice card (5 lines, ~8 chars wide).
pub(crate) fn choice_item_sprite(
    personal: &crate::engine::events::PersonalEvent,
    choice_idx: usize,
    _accent: Color,
    _dim: Color,
    bg: Color,
) -> Vec<Line<'static>> {
    crate::sprites::choice_contextual_sprite(personal, choice_idx, bg)
}

/// Net cashflow per turn.
pub(crate) fn net_cashflow(game: &crate::engine::game::Game) -> (i64, i64, i64, i64) {
    let ms = game.state.money_scale() as i64;
    let income = game.state.income_per_turn * ms;
    let rent = game.state.rent_per_turn * ms;
    let sub = game.state.subscription.cost_per_turn() * ms;
    let net = income - rent - sub;
    (income, rent, sub, net)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Title ASCII art — block-letter "PERMANENT UNDERCLASS"
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub(crate) const TITLE_ART_PERMANENT: [&str; 3] = [
    "█▀█ █▀▀ █▀█ █▄▀▄█ █▀█ █▄ █ █▀▀ █▄ █ ▀█▀",
    "█▀▀ █▄▄ █▀▄ █ ▀ █ █▀█ █ ▀█ █▄▄ █ ▀█  █ ",
    "▀   ▀▀▀ ▀ ▀ ▀   ▀ ▀ ▀ ▀  ▀ ▀▀▀ ▀  ▀  ▀ ",
];

pub(crate) const TITLE_ART_UNDERCLASS: [&str; 3] = [
    "█ █ █▄ █ █▀▄ █▀▀ █▀█ █▀▀ █   █▀█ █▀▀ █▀▀ ▀▀█",
    "█ █ █ ▀█ █▄▀ █▄▄ █▀▄ █   █   █▀█ ▀▀█ ▀▀█  ▀▀",
    "▀▀▀ ▀  ▀ ▀▀  ▀▀▀ ▀ ▀ ▀▀▀ ▀▀▀ ▀ ▀ ▀▀▀ ▀▀▀  ▀ ",
];

/// Render arbitrary text as 3-row block-letter ASCII art (same font as title).
/// Returns exactly 3 `String` rows. Unknown chars are replaced with a space glyph.
pub(crate) fn block_text(s: &str) -> [String; 3] {
    // Each glyph is (row0, row1, row2). Widths may vary per letter.
    fn glyph(ch: char) -> [&'static str; 3] {
        match ch {
            'A' => ["█▀█", "█▀█", "▀ ▀"],
            'B' => ["█▀▄", "█▀▄", "▀▀▀"],
            'C' => ["█▀▀", "█  ", "▀▀▀"],
            'D' => ["█▀▄", "█▄▀", "▀▀ "],
            'E' => ["█▀▀", "█▄▄", "▀▀▀"],
            'F' => ["█▀▀", "█▀ ", "▀  "],
            'G' => ["█▀▀", "█ █", "▀▀▀"],
            'H' => ["█ █", "█▀█", "▀ ▀"],
            'I' => ["▀█▀", " █ ", "▀▀▀"],
            'J' => ["  █", "  █", "▀▀▀"],
            'K' => ["█ █", "█▀▄", "▀ ▀"],
            'L' => ["█  ", "█  ", "▀▀▀"],
            'M' => ["█▄▀▄█", "█ ▀ █", "▀   ▀"],
            'N' => ["█▄ █", "█ ▀█", "▀  ▀"],
            'O' => ["█▀█", "█ █", "▀▀▀"],
            'P' => ["█▀█", "█▀▀", "▀  "],
            'Q' => ["█▀█", "█▄█", "▀▀▀"],
            'R' => ["█▀█", "█▀▄", "▀ ▀"],
            'S' => ["█▀▀", "▀▀█", "▀▀▀"],
            'T' => ["▀█▀", " █ ", " ▀ "],
            'U' => ["█ █", "█ █", "▀▀▀"],
            'V' => ["█ █", "█ █", " ▀ "],
            'W' => ["█   █", "█ █ █", " ▀ ▀ "],
            'X' => ["▀▄▀", " █ ", "▀ ▀"],
            'Y' => ["█ █", " █ ", " ▀ "],
            'Z' => ["▀▀█", " █ ", "█▀▀"],
            '0' => ["█▀█", "█ █", "▀▀▀"],
            '1' => ["▄█ ", " █ ", "▀▀▀"],
            '2' => ["▀▀█", "█▀▀", "▀▀▀"],
            '3' => ["▀▀█", " ▀█", "▀▀▀"],
            '4' => ["█ █", "▀▀█", "  ▀"],
            '5' => ["█▀▀", "▀▀█", "▀▀▀"],
            '6' => ["█▀▀", "█▀█", "▀▀▀"],
            '7' => ["▀▀█", "  █", "  ▀"],
            '8' => ["█▀█", "█▀█", "▀▀▀"],
            '9' => ["█▀█", "▀▀█", "▀▀▀"],
            '?' => ["▀▀█", " ▀▀", " ▀ "],
            _ => ["   ", "   ", "   "], // space / unknown
        }
    }

    let mut rows = [String::new(), String::new(), String::new()];
    for (i, ch) in s.chars().enumerate() {
        if i > 0 {
            // One-space gap between glyphs.
            for row in &mut rows {
                row.push(' ');
            }
        }
        let g = glyph(ch.to_ascii_uppercase());
        rows[0].push_str(g[0]);
        rows[1].push_str(g[1]);
        rows[2].push_str(g[2]);
    }
    rows
}

/// Render text as block-letter art, word-wrapping into lines that fit `max_width`.
/// Returns a `Vec` of 3-row groups (each group is one line of block words).
pub(crate) fn block_text_wrapped(s: &str, max_width: usize) -> Vec<[String; 3]> {
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.is_empty() {
        return vec![];
    }

    let mut result: Vec<[String; 3]> = Vec::new();
    let mut current_words: Vec<&str> = Vec::new();

    for word in &words {
        // Try adding this word to the current line.
        let mut test = current_words.clone();
        test.push(word);
        let test_str = test.join(" ");
        let art = block_text(&test_str);
        // Width is measured by the first row (all rows are equal width in char count).
        if art[0].chars().count() <= max_width || current_words.is_empty() {
            current_words.push(word);
        } else {
            // Flush current line, start new one.
            let line_str = current_words.join(" ");
            result.push(block_text(&line_str));
            current_words = vec![word];
        }
    }

    if !current_words.is_empty() {
        let line_str = current_words.join(" ");
        result.push(block_text(&line_str));
    }

    result
}

/// Tempo-aware variant: `glitch_every` controls frequency, `glitch_rate` controls density.
pub(crate) fn glitch_line_tempo(
    s: &str,
    tick: u64,
    seed: u64,
    glitch_every: u64,
    glitch_rate: u64,
) -> String {
    if glitch_every == 0 || !tick.is_multiple_of(glitch_every) {
        return s.to_string();
    }

    const BLOCK_CHARS: &[char] = &['░', '▒', '▓', '█', '▀', '▄', '▐', '▌'];
    let rate = glitch_rate.max(1);

    // Simple deterministic hash for pseudo-random decisions.
    let hash = |i: u64| -> u64 {
        let mut h = seed.wrapping_mul(6364136223846793005).wrapping_add(i);
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51afd7ed558ccd);
        h ^= h >> 33;
        h
    };

    let mut result: String = s
        .chars()
        .enumerate()
        .map(|(i, ch)| {
            if ch == ' ' {
                ch
            } else {
                let h = hash(tick.wrapping_add(i as u64));
                if h % rate == 0 {
                    BLOCK_CHARS[(h / rate) as usize % BLOCK_CHARS.len()]
                } else {
                    ch
                }
            }
        })
        .collect();

    // ~10% chance to shift the line by ±1 char.
    let shift_h = hash(tick.wrapping_add(seed).wrapping_add(9999));
    if shift_h % 10 == 0 {
        if shift_h % 20 == 0 {
            result.insert(0, ' ');
        } else if !result.is_empty() {
            // Shift left: remove first char if it's a space, else prepend nothing.
            if result.starts_with(' ') {
                result.remove(0);
            }
        }
    }

    result
}

/// Spinner frame for a given UI tick.
///
/// Frames are from `unicode-animations` (`braille` spinner):
/// https://www.npmjs.com/package/unicode-animations
pub(crate) fn spinner(tick: u64) -> &'static str {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    // App ticks at ~180ms; advance ~2 frames per tick to approximate the package's 80ms interval.
    let idx = ((tick * 2) as usize) % FRAMES.len();
    FRAMES[idx]
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Character visuals — sprites, tags, quotes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub(crate) fn character_tag(character: &CharacterDef) -> &str {
    character.presentation.tag.as_str()
}

pub(crate) fn character_quote(character: &CharacterDef) -> &str {
    character.presentation.quote.as_str()
}

pub(crate) fn character_backstory(character: &CharacterDef) -> &str {
    character.presentation.backstory.as_str()
}

/// Per-character accent color (used by themes that want distinct character colors).
pub(crate) fn character_accent(character: &CharacterDef) -> Color {
    let [r, g, b] = character.presentation.accent_rgb;
    Color::Rgb(r, g, b)
}

fn sprite_style(character: &CharacterDef) -> &str {
    character
        .presentation
        .sprite_style
        .as_deref()
        .unwrap_or(character.id.as_str())
}

/// 1.5x character sprite (12 lines, ~15 chars wide).
pub(crate) fn character_sprite_1_5x(
    character: &CharacterDef,
    accent: Color,
    hair: Color,
    dim: Color,
    bright: Color,
    bg: Color,
) -> Vec<Line<'static>> {
    let a = move |t: &str| Span::styled(t.to_string(), Style::default().fg(accent).bg(bg));
    let d = move |t: &str| Span::styled(t.to_string(), Style::default().fg(dim).bg(bg));
    let b = move |t: &str| Span::styled(t.to_string(), Style::default().fg(bright).bg(bg));
    let h = move |t: &str| Span::styled(t.to_string(), Style::default().fg(hair).bg(bg));

    match sprite_style(character) {
        "junior_dev" => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("▓▓▓▓▓"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b("◉"), d("   "), b("◉"), a("├│")]),
            Line::from(vec![a("    │└"), d("  ▽  "), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("HACK "), d("├┐")]),
            Line::from(vec![d("    │┤"), a("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
        "marketing_manager" => vec![
            Line::from(vec![d("     ╭─────╮")]),
            Line::from(vec![a("     │"), h("╱╲╱╲╱"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b("◠"), d("   "), b("◠"), a("├│")]),
            Line::from(vec![a("    │└"), d(" ─── "), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ╭┤"), a("BUZZ "), d("├╮")]),
            Line::from(vec![d("    │╰"), d("     "), d("╯│")]),
            Line::from(vec![d("    │╰┬───┬╯│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ╯ "), d("└───┘"), a(" ╰")]),
        ],
        "ex_faang_engineer" => vec![
            Line::from(vec![d("     ╔═════╗")]),
            Line::from(vec![a("     ║"), h("░░░░░"), a("║")]),
            Line::from(vec![a("    ╔╣"), d("     "), a("╠╗")]),
            Line::from(vec![a("    ║╣"), b("▬"), d("   "), b("▬"), a("╠║")]),
            Line::from(vec![a("    ║╚"), d("  ─  "), a("╝║")]),
            Line::from(vec![a("    ║╔═════╗║")]),
            Line::from(vec![d("    ╔╣"), a("GOOG "), d("╠╗")]),
            Line::from(vec![d("    ║╣"), d("     "), d("╠║")]),
            Line::from(vec![d("    ║╚╤═══╤╝║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![a("    ╝ "), d("╘═══╛"), a(" ╚")]),
        ],
        "product_designer" => vec![
            Line::from(vec![d("     ╭─────╮")]),
            Line::from(vec![a("     │"), h("▒▒▒▒▒"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b("◠"), d("   "), b("◠"), a("├│")]),
            Line::from(vec![a("    │└"), d("  ◡  "), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ╭┤"), a("FIGMA"), d("├╮")]),
            Line::from(vec![d("    │╰"), d("     "), d("╯│")]),
            Line::from(vec![d("    │╰┬───┬╯│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ╯ "), d("└───┘"), a(" ╰")]),
        ],
        "podcaster" => vec![
            Line::from(vec![d("    ┌───────┐")]),
            Line::from(vec![a("    │"), h(" ░░░░░ "), a("│")]),
            Line::from(vec![a("   ┌┤"), d("       "), a("├┐")]),
            Line::from(vec![a("   │┤"), b(" ☉"), d("   "), b("☉ "), a("├│")]),
            Line::from(vec![a("   │└"), d("   ▽   "), a("┘│")]),
            Line::from(vec![a("   │┌───────┐│")]),
            Line::from(vec![d("   ╭┤"), a("░MIC░  "), d("├╮")]),
            Line::from(vec![d("   │╰"), d("       "), d("╯│")]),
            Line::from(vec![d("   │╰┬─────┬╯│")]),
            Line::from(vec![d("   │ │     │ │")]),
            Line::from(vec![d("   │ │     │ │")]),
            Line::from(vec![a("   ╯ "), d("└─────┘"), a(" ╰")]),
        ],
        "plumber" => vec![
            Line::from(vec![d("     ╔═════╗")]),
            Line::from(vec![a("     ║"), h("▀▀▀▀▀"), a("║")]),
            Line::from(vec![a("    ╔╣"), d("     "), a("╠╗")]),
            Line::from(vec![a("    ║╣"), b("●"), d("   "), b("●"), a("╠║")]),
            Line::from(vec![a("    ║╚"), d(" ▄▄▄ "), a("╝║")]),
            Line::from(vec![a("    ║╔═════╗║")]),
            Line::from(vec![d("    ╔╣"), a("PIPE "), d("╠╗")]),
            Line::from(vec![d("    ║╣"), d("     "), d("╠║")]),
            Line::from(vec![d("    ║╚╤═══╤╝║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![a("    ╝ "), d("╘═══╛"), a(" ╚")]),
        ],
        "stealth_founder" => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("░▓░▓░"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b("◉"), d("   "), b("◉"), a("├│")]),
            Line::from(vec![a("    │└"), d("  ▽  "), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("SHIP "), d("├┐")]),
            Line::from(vec![d("    │┤"), a("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
        "angel_investor" => vec![
            Line::from(vec![d("     ╔═════╗")]),
            Line::from(vec![a("     ║"), h(" ░░░ "), a("║")]),
            Line::from(vec![a("    ╔╣"), d("     "), a("╠╗")]),
            Line::from(vec![a("    ║╣"), b("◠"), d("   "), b("◠"), a("╠║")]),
            Line::from(vec![a("    ║╚"), d(" ─── "), a("╝║")]),
            Line::from(vec![a("    ║╔═════╗║")]),
            Line::from(vec![d("    ╔╣"), a(" $$$"), d(" "), d("╠╗")]),
            Line::from(vec![d("    ║╣"), d("     "), d("╠║")]),
            Line::from(vec![d("    ║╚╤═══╤╝║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![a("    ╝ "), d("╘═══╛"), a(" ╚")]),
        ],
        "product_guy" => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("▓░▓░▓"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b("◠"), d("   "), b("◠"), a("├│")]),
            Line::from(vec![a("    │└"), d(" ─── "), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ╭┤"), a("JIRA "), d("├╮")]),
            Line::from(vec![d("    │╰"), d("     "), d("╯│")]),
            Line::from(vec![d("    │╰┬───┬╯│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ╯ "), d("└───┘"), a(" ╰")]),
        ],
        "twitter_anon" => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("▒▒▒▒▒"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("  ?  "), a("├┐")]),
            Line::from(vec![a("    │┤"), b("◉"), d("   "), b("◉"), a("├│")]),
            Line::from(vec![a("    │└"), d("  ▽  "), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("ANON "), d("├┐")]),
            Line::from(vec![d("    │┤"), a("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
        "ai_researcher" => vec![
            Line::from(vec![d("     ╔═════╗")]),
            Line::from(vec![a("     ║"), h("░▓█▓░"), a("║")]),
            Line::from(vec![a("    ╔╣"), d("     "), a("╠╗")]),
            Line::from(vec![a("    ║╣"), b("◎"), d("   "), b("◎"), a("╠║")]),
            Line::from(vec![a("    ║╚"), d("  ─  "), a("╝║")]),
            Line::from(vec![a("    ║╔═════╗║")]),
            Line::from(vec![d("    ╔╣"), a("ARXIV"), d("╠╗")]),
            Line::from(vec![d("    ║╣"), d("     "), d("╠║")]),
            Line::from(vec![d("    ║╚╤═══╤╝║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![a("    ╝ "), d("╘═══╛"), a(" ╚")]),
        ],
        "crypto_bro" => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("▄▀▄▀▄"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b("◉"), d("   "), b("◉"), a("├│")]),
            Line::from(vec![a("    │└"), d("  ▽  "), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("HODL "), d("├┐")]),
            Line::from(vec![d("    │┤"), a("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
        "hr_lady" => vec![
            Line::from(vec![d("     ╭─────╮")]),
            Line::from(vec![a("     │"), h("█▓█▓█"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b("◠"), d("   "), b("◠"), a("├│")]),
            Line::from(vec![a("    │└"), d(" ─── "), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ╭┤"), a(" PIP "), d("├╮")]),
            Line::from(vec![d("    │╰"), d("     "), d("╯│")]),
            Line::from(vec![d("    │╰┬───┬╯│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ╯ "), d("└───┘"), a(" ╰")]),
        ],
        "digital_nomad" => vec![
            Line::from(vec![d("     ╭─────╮")]),
            Line::from(vec![a("     │"), h("▒▓▒▓▒"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b("◉"), d("   "), b("◉"), a("├│")]),
            Line::from(vec![a("    │└"), d("  ◡  "), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("WIFI "), d("├┐")]),
            Line::from(vec![d("    │┤"), a("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
        "chief_of_staff" => vec![
            Line::from(vec![d("     ╔═════╗")]),
            Line::from(vec![a("     ║"), h(" ▒▒▒ "), a("║")]),
            Line::from(vec![a("    ╔╣"), d("     "), a("╠╗")]),
            Line::from(vec![a("    ║╣"), b("◠"), d("   "), b("◠"), a("╠║")]),
            Line::from(vec![a("    ║╚"), d(" ─── "), a("╝║")]),
            Line::from(vec![a("    ║╔═════╗║")]),
            Line::from(vec![d("    ╔╣"), a("MEMO "), d("╠╗")]),
            Line::from(vec![d("    ║╣"), d("     "), d("╠║")]),
            Line::from(vec![d("    ║╚╤═══╤╝║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![a("    ╝ "), d("╘═══╛"), a(" ╚")]),
        ],
        _ => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("░░░░░"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b("◉"), d("   "), b("◉"), a("├│")]),
            Line::from(vec![a("    │└"), d("  ▽  "), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("CHAR "), d("├┐")]),
            Line::from(vec![d("    │┤"), d("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
    }
}

/// Animated 1.5x character sprite with eye blink + mouth cycle.
pub(crate) fn character_sprite_1_5x_animated(
    character: &CharacterDef,
    tick: u64,
    accent: Color,
    hair: Color,
    dim: Color,
    bright: Color,
    bg: Color,
) -> Vec<Line<'static>> {
    let a = move |t: &str| Span::styled(t.to_string(), Style::default().fg(accent).bg(bg));
    let d = move |t: &str| Span::styled(t.to_string(), Style::default().fg(dim).bg(bg));
    let b = move |t: &str| Span::styled(t.to_string(), Style::default().fg(bright).bg(bg));
    let h = move |t: &str| Span::styled(t.to_string(), Style::default().fg(hair).bg(bg));

    let blinking = tick % 22 >= 20;

    let (normal_eye, mouth_frames): (&str, [&str; 4]) = match sprite_style(character) {
        "junior_dev" => ("◉", ["▽", "─", "▽", "◡"]),
        "marketing_manager" => ("◠", ["───", "─ ─", "───", " ◡ "]),
        "ex_faang_engineer" => ("▬", ["─", "▽", "─", "◡"]),
        "product_designer" => ("◠", ["◡", "─", "◡", "▽"]),
        "podcaster" => ("☉", ["▽", "─", "▽", "◡"]),
        "plumber" => ("●", ["▄▄▄", "───", "▄▄▄", " ◡ "]),
        "stealth_founder" => ("◉", ["▽", "◡", "▽", "─"]),
        "angel_investor" => ("◠", ["───", " ◡ ", "───", "─ ─"]),
        "product_guy" => ("◠", ["───", "─ ─", "───", " ◡ "]),
        "twitter_anon" => ("◉", ["▽", "─", "◡", "▽"]),
        "ai_researcher" => ("◎", ["─", "▽", "─", "◡"]),
        "crypto_bro" => ("◉", ["▽", "─", "▽", "◡"]),
        "hr_lady" => ("◠", ["───", "─ ─", "───", " ◡ "]),
        "digital_nomad" => ("◉", ["◡", "─", "▽", "◡"]),
        "chief_of_staff" => ("◠", ["───", " ◡ ", "───", "─ ─"]),
        _ => ("◉", ["▽", "─", "▽", "◡"]),
    };

    let eye = if blinking { "─" } else { normal_eye };
    let mouth_idx = ((tick / 5) % 4) as usize;
    let mouth = mouth_frames[mouth_idx];

    match sprite_style(character) {
        "junior_dev" => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("▓▓▓▓▓"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b(eye), d("   "), b(eye), a("├│")]),
            Line::from(vec![a("    │└"), d(&format!("  {}  ", mouth)), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("HACK "), d("├┐")]),
            Line::from(vec![d("    │┤"), a("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
        "marketing_manager" => vec![
            Line::from(vec![d("     ╭─────╮")]),
            Line::from(vec![a("     │"), h("╱╲╱╲╱"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b(eye), d("   "), b(eye), a("├│")]),
            Line::from(vec![a("    │└"), d(&format!(" {} ", mouth)), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ╭┤"), a("BUZZ "), d("├╮")]),
            Line::from(vec![d("    │╰"), d("     "), d("╯│")]),
            Line::from(vec![d("    │╰┬───┬╯│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ╯ "), d("└───┘"), a(" ╰")]),
        ],
        "ex_faang_engineer" => vec![
            Line::from(vec![d("     ╔═════╗")]),
            Line::from(vec![a("     ║"), h("░░░░░"), a("║")]),
            Line::from(vec![a("    ╔╣"), d("     "), a("╠╗")]),
            Line::from(vec![a("    ║╣"), b(eye), d("   "), b(eye), a("╠║")]),
            Line::from(vec![a("    ║╚"), d(&format!("  {}  ", mouth)), a("╝║")]),
            Line::from(vec![a("    ║╔═════╗║")]),
            Line::from(vec![d("    ╔╣"), a("GOOG "), d("╠╗")]),
            Line::from(vec![d("    ║╣"), d("     "), d("╠║")]),
            Line::from(vec![d("    ║╚╤═══╤╝║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![a("    ╝ "), d("╘═══╛"), a(" ╚")]),
        ],
        "product_designer" => vec![
            Line::from(vec![d("     ╭─────╮")]),
            Line::from(vec![a("     │"), h("▒▒▒▒▒"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b(eye), d("   "), b(eye), a("├│")]),
            Line::from(vec![a("    │└"), d(&format!("  {}  ", mouth)), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ╭┤"), a("FIGMA"), d("├╮")]),
            Line::from(vec![d("    │╰"), d("     "), d("╯│")]),
            Line::from(vec![d("    │╰┬───┬╯│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ╯ "), d("└───┘"), a(" ╰")]),
        ],
        "podcaster" => vec![
            Line::from(vec![d("    ┌───────┐")]),
            Line::from(vec![a("    │"), h(" ░░░░░ "), a("│")]),
            Line::from(vec![a("   ┌┤"), d("       "), a("├┐")]),
            Line::from(vec![
                a("   │┤"),
                b(&format!(" {} ", eye)),
                d("   "),
                b(&format!("{} ", eye)),
                a("├│"),
            ]),
            Line::from(vec![a("   │└"), d(&format!("   {}   ", mouth)), a("┘│")]),
            Line::from(vec![a("   │┌───────┐│")]),
            Line::from(vec![d("   ╭┤"), a("░MIC░  "), d("├╮")]),
            Line::from(vec![d("   │╰"), d("       "), d("╯│")]),
            Line::from(vec![d("   │╰┬─────┬╯│")]),
            Line::from(vec![d("   │ │     │ │")]),
            Line::from(vec![d("   │ │     │ │")]),
            Line::from(vec![a("   ╯ "), d("└─────┘"), a(" ╰")]),
        ],
        "plumber" => vec![
            Line::from(vec![d("     ╔═════╗")]),
            Line::from(vec![a("     ║"), h("▀▀▀▀▀"), a("║")]),
            Line::from(vec![a("    ╔╣"), d("     "), a("╠╗")]),
            Line::from(vec![a("    ║╣"), b(eye), d("   "), b(eye), a("╠║")]),
            Line::from(vec![a("    ║╚"), d(&format!(" {} ", mouth)), a("╝║")]),
            Line::from(vec![a("    ║╔═════╗║")]),
            Line::from(vec![d("    ╔╣"), a("PIPE "), d("╠╗")]),
            Line::from(vec![d("    ║╣"), d("     "), d("╠║")]),
            Line::from(vec![d("    ║╚╤═══╤╝║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![a("    ╝ "), d("╘═══╛"), a(" ╚")]),
        ],
        "stealth_founder" => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("░▓░▓░"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b(eye), d("   "), b(eye), a("├│")]),
            Line::from(vec![a("    │└"), d(&format!("  {}  ", mouth)), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("SHIP "), d("├┐")]),
            Line::from(vec![d("    │┤"), a("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
        "angel_investor" => vec![
            Line::from(vec![d("     ╔═════╗")]),
            Line::from(vec![a("     ║"), h(" ░░░ "), a("║")]),
            Line::from(vec![a("    ╔╣"), d("     "), a("╠╗")]),
            Line::from(vec![a("    ║╣"), b(eye), d("   "), b(eye), a("╠║")]),
            Line::from(vec![a("    ║╚"), d(&format!(" {} ", mouth)), a("╝║")]),
            Line::from(vec![a("    ║╔═════╗║")]),
            Line::from(vec![d("    ╔╣"), a(" $$$"), d(" "), d("╠╗")]),
            Line::from(vec![d("    ║╣"), d("     "), d("╠║")]),
            Line::from(vec![d("    ║╚╤═══╤╝║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![a("    ╝ "), d("╘═══╛"), a(" ╚")]),
        ],
        "product_guy" => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("▓░▓░▓"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b(eye), d("   "), b(eye), a("├│")]),
            Line::from(vec![a("    │└"), d(&format!(" {} ", mouth)), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ╭┤"), a("JIRA "), d("├╮")]),
            Line::from(vec![d("    │╰"), d("     "), d("╯│")]),
            Line::from(vec![d("    │╰┬───┬╯│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ╯ "), d("└───┘"), a(" ╰")]),
        ],
        "twitter_anon" => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("▒▒▒▒▒"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("  ?  "), a("├┐")]),
            Line::from(vec![a("    │┤"), b(eye), d("   "), b(eye), a("├│")]),
            Line::from(vec![a("    │└"), d(&format!("  {}  ", mouth)), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("ANON "), d("├┐")]),
            Line::from(vec![d("    │┤"), a("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
        "ai_researcher" => vec![
            Line::from(vec![d("     ╔═════╗")]),
            Line::from(vec![a("     ║"), h("░▓█▓░"), a("║")]),
            Line::from(vec![a("    ╔╣"), d("     "), a("╠╗")]),
            Line::from(vec![a("    ║╣"), b(eye), d("   "), b(eye), a("╠║")]),
            Line::from(vec![a("    ║╚"), d(&format!("  {}  ", mouth)), a("╝║")]),
            Line::from(vec![a("    ║╔═════╗║")]),
            Line::from(vec![d("    ╔╣"), a("ARXIV"), d("╠╗")]),
            Line::from(vec![d("    ║╣"), d("     "), d("╠║")]),
            Line::from(vec![d("    ║╚╤═══╤╝║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![a("    ╝ "), d("╘═══╛"), a(" ╚")]),
        ],
        "crypto_bro" => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("▄▀▄▀▄"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b(eye), d("   "), b(eye), a("├│")]),
            Line::from(vec![a("    │└"), d(&format!("  {}  ", mouth)), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("HODL "), d("├┐")]),
            Line::from(vec![d("    │┤"), a("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
        "hr_lady" => vec![
            Line::from(vec![d("     ╭─────╮")]),
            Line::from(vec![a("     │"), h("█▓█▓█"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b(eye), d("   "), b(eye), a("├│")]),
            Line::from(vec![a("    │└"), d(&format!(" {} ", mouth)), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ╭┤"), a(" PIP "), d("├╮")]),
            Line::from(vec![d("    │╰"), d("     "), d("╯│")]),
            Line::from(vec![d("    │╰┬───┬╯│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ╯ "), d("└───┘"), a(" ╰")]),
        ],
        "digital_nomad" => vec![
            Line::from(vec![d("     ╭─────╮")]),
            Line::from(vec![a("     │"), h("▒▓▒▓▒"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b(eye), d("   "), b(eye), a("├│")]),
            Line::from(vec![a("    │└"), d(&format!("  {}  ", mouth)), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("WIFI "), d("├┐")]),
            Line::from(vec![d("    │┤"), a("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
        "chief_of_staff" => vec![
            Line::from(vec![d("     ╔═════╗")]),
            Line::from(vec![a("     ║"), h(" ▒▒▒ "), a("║")]),
            Line::from(vec![a("    ╔╣"), d("     "), a("╠╗")]),
            Line::from(vec![a("    ║╣"), b(eye), d("   "), b(eye), a("╠║")]),
            Line::from(vec![a("    ║╚"), d(&format!(" {} ", mouth)), a("╝║")]),
            Line::from(vec![a("    ║╔═════╗║")]),
            Line::from(vec![d("    ╔╣"), a("MEMO "), d("╠╗")]),
            Line::from(vec![d("    ║╣"), d("     "), d("╠║")]),
            Line::from(vec![d("    ║╚╤═══╤╝║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![d("    ║ │   │ ║")]),
            Line::from(vec![a("    ╝ "), d("╘═══╛"), a(" ╚")]),
        ],
        _ => vec![
            Line::from(vec![d("     ┌─────┐")]),
            Line::from(vec![a("     │"), h("░░░░░"), a("│")]),
            Line::from(vec![a("    ┌┤"), d("     "), a("├┐")]),
            Line::from(vec![a("    │┤"), b(eye), d("   "), b(eye), a("├│")]),
            Line::from(vec![a("    │└"), d(&format!("  {}  ", mouth)), a("┘│")]),
            Line::from(vec![a("    │┌─────┐│")]),
            Line::from(vec![d("    ┌┤"), a("CHAR "), d("├┐")]),
            Line::from(vec![d("    │┤"), d("     "), d("├│")]),
            Line::from(vec![d("    │└┬───┬┘│")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![d("    │ │   │ │")]),
            Line::from(vec![a("    ┘ "), d("└───┘"), a(" └")]),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn stat_bar_spans_fill_width() {
        let color = Color::Yellow;
        let empty = Color::Gray;
        let bg = Color::Black;
        for val in [0, 25, 50, 75, 100] {
            let spans = bar_spans(val, 10, color, empty, bg);
            let total_chars: usize = spans.iter().map(|s| s.content.chars().count()).sum();
            assert_eq!(total_chars, 10, "bar_spans(val={}) should be 10 chars", val);
        }
    }

    #[test]
    fn sprite_1_5x_has_12_lines() {
        use crate::engine::characters::CharacterCatalog;
        let bg = Color::Black;
        let catalog = CharacterCatalog::embedded().expect("character catalog");
        for character in catalog.all() {
            let sprite = character_sprite_1_5x(
                character,
                Color::Green,
                Color::Gray,
                Color::DarkGray,
                Color::White,
                bg,
            );
            assert_eq!(
                sprite.len(),
                12,
                "{:?} 1.5x sprite should be 12 lines",
                character.id
            );

            let animated = character_sprite_1_5x_animated(
                character,
                0,
                Color::Green,
                Color::Gray,
                Color::DarkGray,
                Color::White,
                bg,
            );
            assert_eq!(
                animated.len(),
                12,
                "{:?} 1.5x animated sprite should be 12 lines",
                character.id
            );
        }
    }

    #[test]
    fn sprite_1_5x_blink_changes_eyes() {
        use crate::engine::characters::CharacterCatalog;
        let catalog = CharacterCatalog::embedded().expect("character catalog");
        let id = catalog.parse_id("junior_dev").expect("junior_dev");
        let character = catalog.get(&id).expect("junior_dev character");
        let bg = Color::Black;
        let normal = character_sprite_1_5x_animated(
            character,
            0,
            Color::Green,
            Color::Gray,
            Color::DarkGray,
            Color::White,
            bg,
        );
        let blink = character_sprite_1_5x_animated(
            character,
            20,
            Color::Green,
            Color::Gray,
            Color::DarkGray,
            Color::White,
            bg,
        );
        // Eye line is index 3 for 1.5x sprites
        let normal_eye: String = normal[3].spans.iter().map(|s| s.content.as_ref()).collect();
        let blink_eye: String = blink[3].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_ne!(
            normal_eye, blink_eye,
            "1.5x eyes should change during blink"
        );
    }
}
