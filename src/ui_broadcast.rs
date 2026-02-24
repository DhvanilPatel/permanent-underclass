// ── ui_broadcast.rs ── Permanent Underclass: saturated CRT on navy ──
//
// Navy background, neon accents. Dense financial data, scrolling tickers.
// Classified-document borders. The feel of watching the world end from
// a CRT terminal while trying to make personal decisions.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::app::{App, AudioPrefs, InputMode, Screen};
use crate::engine::characters::{CharacterCatalog, CharacterDef, CharacterId};
use crate::engine::endings::EndingId;
use crate::engine::events::{TurnEvents, TurnOutcome};
use crate::engine::subscription::SubscriptionTier;
use crate::engine::tempo::Tempo;
use crate::ui;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Palette — navy background, saturated neon accents (CRT aesthetic)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

const BG: Color = Color::Rgb(0x06, 0x06, 0x0a);
const BG_PANEL: Color = Color::Rgb(0x0a, 0x0a, 0x10);
const BG_TICKER: Color = Color::Rgb(0x0a, 0x0a, 0x10);
const BORDER: Color = Color::Rgb(0x28, 0x28, 0x38);
const BORDER_HI: Color = Color::Rgb(0x44, 0x44, 0x66);
const TEXT: Color = Color::Rgb(0xcc, 0xdd, 0xee);
const TEXT_DIM: Color = Color::Rgb(0x58, 0x58, 0x6a);
const TEXT_HI: Color = Color::Rgb(0xff, 0xff, 0xff);
const AMBER: Color = Color::Rgb(0xff, 0xb0, 0x00);
const RED: Color = Color::Rgb(0xff, 0x44, 0x44);
const GREEN: Color = Color::Rgb(0x00, 0xff, 0x41);
const CYAN: Color = Color::Rgb(0x00, 0xdd, 0xff);
const MAGENTA: Color = Color::Rgb(0xbb, 0x88, 0xff);
const WHITE: Color = Color::Rgb(0xff, 0xff, 0xff);
const FOCUS_MODAL_W: u16 = 56;
const FOCUS_MODAL_H: u16 = 15;
const MAIN_LEFT_PCT: u16 = 72;
const MAIN_RIGHT_PCT: u16 = 28;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Entry point
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Paint the whole screen black first.
    frame.render_widget(Paragraph::new("").style(Style::default().bg(BG)), area);

    // Min terminal check.
    if area.width < 80 || area.height < 24 {
        let msg = Paragraph::new("Terminal too small. Need 80x24 minimum.")
            .style(Style::default().fg(RED).bg(BG))
            .alignment(Alignment::Center);
        let r = ui::centered(area, 50, 1);
        frame.render_widget(msg, r);
        return;
    }

    match app.screen() {
        Screen::Title => render_title(frame, app, area),
        Screen::TitleSettings { selected } => render_title_settings(frame, app, area, *selected),
        Screen::Credits => render_credits(frame, app, area),
        Screen::PickClass { selected } => render_pick_class(frame, app, area, *selected),
        Screen::Turn { events, input } => render_turn(frame, app, area, events, input),
        Screen::ManageSub {
            selected,
            events,
            input,
        } => render_manage_sub(frame, app, area, *selected, events, input),
        Screen::Settings {
            selected,
            events,
            input,
        } => render_settings(frame, app, area, *selected, events, input),
        Screen::Outcome { events, outcome } => render_outcome(frame, app, area, events, outcome),
        Screen::StatPause { events, .. } => {
            // Show the turn layout (stats visible) with no modal, so the player can see changes.
            let pause_input = crate::app::InputState {
                mode: InputMode::Cards,
                buffer: String::new(),
                selected: usize::MAX,
            };
            render_turn(frame, app, area, events, &pause_input);
        }
        Screen::Ending {
            ending,
            preview_idx,
        } => render_ending(frame, app, area, ending, *preview_idx),
    }

    // LLM overlay on top of everything.
    if let Some(msg) = app.llm_overlay() {
        render_overlay(frame, area, msg, app.tick);
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Title screen
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_title(frame: &mut Frame, app: &App, area: Rect) {
    let inner = render_title_modal(frame, app, area, None);
    let vt = app.visual_tempo();
    let selected = app.title_menu_selected();
    let menu_items: [&str; 5] = ["PLAY GAME", "SETTINGS", "CREDITS", "SOURCE CODE", "EXIT"];

    let mut lines: Vec<Line> = Vec::new();

    // Restore the glitchy block-art headline.
    let pad = "  ";
    for (i, art_line) in ui::TITLE_ART_PERMANENT.iter().enumerate() {
        let text = format!("{}{}", pad, art_line);
        lines.push(Line::from(Span::styled(
            ui::glitch_line_tempo(
                &text,
                app.tick,
                11 + (i as u64),
                vt.glitch_every,
                vt.glitch_rate,
            ),
            Style::default()
                .fg(TEXT_HI)
                .bg(BG)
                .add_modifier(Modifier::BOLD),
        )));
    }
    for (i, art_line) in ui::TITLE_ART_UNDERCLASS.iter().enumerate() {
        let text = format!("{}{}", pad, art_line);
        lines.push(Line::from(Span::styled(
            ui::glitch_line_tempo(
                &text,
                app.tick,
                14 + (i as u64),
                vt.glitch_every,
                vt.glitch_rate,
            ),
            Style::default()
                .fg(TEXT_HI)
                .bg(BG)
                .add_modifier(Modifier::BOLD),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  singularity simulator v0.2",
        Style::default().fg(TEXT_DIM).bg(BG),
    )));
    lines.push(Line::from(Span::styled(
        "  survive 2025–2027",
        Style::default().fg(TEXT_DIM).bg(BG),
    )));
    lines.push(Line::from(Span::styled(
        "  median outcome -- terminal velocity",
        Style::default().fg(TEXT_DIM).bg(BG),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  freeform mode uses your local Claude/Codex CLI.",
        Style::default().fg(TEXT_DIM).bg(BG),
    )));
    lines.push(Line::from(Span::styled(
        "  configure it in settings.",
        Style::default().fg(TEXT_DIM).bg(BG),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  all characters and events are fictional.",
        Style::default().fg(TEXT_DIM).bg(BG),
    )));
    lines.push(Line::from(""));

    for (i, item) in menu_items.iter().enumerate() {
        let marker = if i == selected { "▸ " } else { "  " };
        let fg = if i == selected { CYAN } else { TEXT };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(CYAN).bg(BG)),
            Span::styled(
                item.to_string(),
                Style::default().fg(fg).bg(BG).add_modifier(Modifier::BOLD),
            ),
        ]));
        if i == 0 {
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Use ↑/↓ and [Enter]",
        Style::default().fg(TEXT_DIM).bg(BG),
    )));

    frame.render_widget(Paragraph::new(lines).style(Style::default().bg(BG)), inner);
}

fn render_title_settings(frame: &mut Frame, app: &App, area: Rect, selected: usize) {
    let inner = render_title_modal(frame, app, area, Some(" SETTINGS "));
    let prefs = &app.audio_prefs;
    let narrator_label = app.narrator_label();
    let cheat_mode_label = app.cheat_mode_label();
    let tempo_label = app.tempo_label();
    let shepard_preset_label = app.shepard_preset_label();
    let items: [(&str, &str); 8] = [
        (
            "BACKGROUND SCORE",
            AudioPrefs::LEVEL_LABELS[prefs.score_level as usize],
        ),
        ("SCORE TRACK", prefs.score_track.label()),
        (
            "SHEPARD TONE",
            AudioPrefs::LEVEL_LABELS[prefs.shepard_level as usize],
        ),
        ("SHEPARD PRESET", &shepard_preset_label),
        ("SELECTION SFX", if prefs.sfx_on { "ON" } else { "OFF" }),
        ("LLM SHOWRUNNER", narrator_label),
        ("CHEAT MODE", cheat_mode_label),
        ("RUN LENGTH", &tempo_label),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  Sound and gameplay settings.",
            Style::default().fg(TEXT_DIM).bg(BG),
        )),
        Line::from(""),
    ];

    for (i, (label, value)) in items.iter().enumerate() {
        let marker = if i == selected { "> " } else { "  " };
        let fg = if i == selected { TEXT_HI } else { TEXT };
        let mut spans: Vec<Span> = vec![
            Span::styled(marker, Style::default().fg(CYAN).bg(BG)),
            Span::styled(format!("{:<20}", label), Style::default().fg(fg).bg(BG)),
        ];

        if i == 0 || i == 2 {
            let level = if i == 0 {
                prefs.score_level
            } else {
                prefs.shepard_level
            };
            let bar: String = (0..AudioPrefs::NUM_LEVELS)
                .map(|j| if j < level { '█' } else { '░' })
                .collect();
            spans.push(Span::styled(
                format!("{} ", bar),
                Style::default().fg(CYAN).bg(BG),
            ));
        }

        let val_color = if *value == "OFF" { TEXT_DIM } else { CYAN };
        spans.push(Span::styled(
            value.to_string(),
            Style::default()
                .fg(val_color)
                .bg(BG)
                .add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ↑/↓ select  ←/→ or [Enter] change  [Esc] back",
        Style::default().fg(TEXT_DIM).bg(BG),
    )));

    frame.render_widget(Paragraph::new(lines).style(Style::default().bg(BG)), inner);
}

fn render_credits(frame: &mut Frame, app: &App, area: Rect) {
    let inner = render_title_modal(frame, app, area, Some(" CREDITS "));
    let lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "Permanent Underclass",
            Style::default()
                .fg(TEXT_HI)
                .bg(BG)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "A terminal game about surviving the AI transition.",
            Style::default().fg(TEXT).bg(BG),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "made by @dhvanil, claude and codex",
            Style::default().fg(TEXT).bg(BG),
        )),
        Line::from(Span::styled(
            "Engine: Rust + ratatui",
            Style::default().fg(TEXT).bg(BG),
        )),
        Line::from(Span::styled(
            "Optional freeform narration: Codex / Claude CLIs",
            Style::default().fg(TEXT).bg(BG),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Code license: MIT.",
            Style::default().fg(TEXT_DIM).bg(BG),
        )),
        Line::from(Span::styled(
            "Non-code assets use separate terms; see ASSET_LICENSES.md.",
            Style::default().fg(TEXT_DIM).bg(BG),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "[Esc]: Back",
            Style::default().fg(TEXT_DIM).bg(BG),
        )),
    ];

    let text_rect = Rect {
        x: inner.x + 1,
        y: inner.y,
        width: inner.width.saturating_sub(2),
        height: inner.height,
    };
    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(BG))
            .wrap(Wrap { trim: true }),
        text_rect,
    );
}

fn render_title_modal(frame: &mut Frame, app: &App, area: Rect, title: Option<&str>) -> Rect {
    let content_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height.saturating_sub(2),
    };
    let card = title_modal_rect(content_area);
    let vt = app.visual_tempo();
    render_matrix_rain(frame, app.tick, area, card, 1, vt.rain_speed, vt.rain_trail);
    render_title_ticker(
        frame,
        app.tick,
        Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        },
        vt.ticker_speed,
    );

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(BORDER_HI))
        .style(Style::default().bg(BG));

    if let Some(title) = title {
        block = block
            .title(Span::styled(
                title,
                Style::default()
                    .fg(AMBER)
                    .bg(BG)
                    .add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center);
    }

    frame.render_widget(block, card);

    Rect {
        x: card.x + 2,
        y: card.y + 2,
        width: card.width.saturating_sub(4),
        height: card.height.saturating_sub(4),
    }
}

/// Matrix rain background — falling columns of characters.
/// `occluder`: the card rect where rain should render much fainter.
fn render_matrix_rain(
    frame: &mut Frame,
    tick: u64,
    area: Rect,
    occluder: Rect,
    col_step: u64,
    rain_speed: f32,
    rain_trail: i64,
) {
    render_matrix_rain_colored(
        frame,
        tick,
        area,
        occluder,
        col_step,
        Color::Rgb(0x00, 0xff, 0x00),
        rain_speed,
        rain_trail,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_matrix_rain_colored(
    frame: &mut Frame,
    tick: u64,
    area: Rect,
    occluder: Rect,
    col_step: u64,
    color: Color,
    rain_speed: f32,
    rain_trail: i64,
) {
    const CHARS: &[char] = &[
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'ア', 'イ', 'ウ', 'エ', 'オ', 'カ', 'キ',
        'ク', 'ケ', 'コ', '░', '▒', '▓', '╌', '╎', '┊', '┆',
    ];
    let h = area.height as u64;
    let w = area.width as u64;
    if h == 0 || w == 0 {
        return;
    }

    let hash = |a: u64, b: u64| -> u64 {
        let mut v = a.wrapping_mul(6364136223846793005).wrapping_add(b);
        v ^= v >> 33;
        v = v.wrapping_mul(0xff51afd7ed558ccd);
        v ^= v >> 33;
        v
    };

    let trail_len: i64 = rain_trail;

    let step = col_step.max(1);
    let mut col = 0u64;
    while col < w {
        let seed = hash(col, 7919);
        let base_speed = 1 + (seed % 3);
        let speed = (((base_speed as f32) * rain_speed) as u64).max(1);
        let phase = seed % (h * 2);
        let head = ((tick * speed + phase) % (h * 2)) as i64;

        for row in 0..h as i64 {
            let dist = head - row;
            if dist < 0 || dist > trail_len {
                continue;
            }
            let ch_seed = hash(col * 1000 + (row as u64), tick / 2);
            let ch = CHARS[(ch_seed % (CHARS.len() as u64)) as usize];

            // Extract base RGB from the color parameter.
            let (cr, cg, cb) = match color {
                Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
                _ => (0.0, 255.0, 0.0), // fallback green
            };
            let (r, g, b_val) = if dist == 0 {
                // Head: bright, washed-out version of the color
                let mix = |c: f32| -> u8 { (c * 0.8 + 255.0 * 0.2).min(255.0) as u8 };
                (mix(cr), mix(cg), mix(cb))
            } else if dist <= 4 {
                let intensity =
                    ((0x44 + (0x66u64).saturating_sub((dist as u64) * 0x14)) as f32) / 255.0;
                (
                    (cr * intensity) as u8,
                    (cg * intensity) as u8,
                    (cb * intensity) as u8,
                )
            } else {
                let fade = ((dist - 4) as f32).min(16.0);
                let intensity = ((0x22 as f32) - fade).max(0.0) / 255.0;
                (
                    (cr * intensity) as u8,
                    (cg * intensity) as u8,
                    (cb * intensity) as u8,
                )
            };

            let x = area.x + (col as u16);
            let y = area.y + (row as u16);

            // Skip rendering inside the card area
            if x >= occluder.x
                && x < occluder.x + occluder.width
                && y >= occluder.y
                && y < occluder.y + occluder.height
            {
                continue;
            }

            frame.render_widget(
                Paragraph::new(Span::styled(
                    ch.to_string(),
                    Style::default().fg(Color::Rgb(r, g, b_val)).bg(BG),
                )),
                Rect {
                    x,
                    y,
                    width: 1,
                    height: 1,
                },
            );
        }
        col += step;
    }
}

/// Scrolling ticker tape for the title screen bottom.
fn render_title_ticker(frame: &mut Frame, tick: u64, area: Rect, scroll_speed: usize) {
    const HEADLINES: &[&str] = &[
        "AI REPLACES 40% OF ENTRY-LEVEL JOBS",
        "RENT UP 23% IN MAJOR TECH HUBS",
        "LAYOFFS HIT 150K THIS QUARTER",
        "GPT-7 PASSES BAR EXAM WITH 99.8%",
        "REMOTE WORK BAN SPREADS TO MORE COMPANIES",
        "JUNIOR DEV SALARIES FLAT FOR 3RD YEAR",
        "AI STARTUP VALUED AT $50B WITH 12 EMPLOYEES",
        "CODING BOOTCAMP ENROLLMENT DOWN 60%",
        "LINKEDIN: \"OPEN TO WORK\" BADGES UP 340%",
        "MEDIAN HOME PRICE HITS $620K NATIONWIDE",
        "TECH WORKERS MOVING TO CHEAPER CITIES IN RECORD NUMBERS",
        "MANAGER-TO-IC RATIO REACHES ALL-TIME HIGH",
        "\"VIBE CODING\" DECLARED OFFICIAL DEVELOPMENT METHODOLOGY",
        "FREELANCE RATES DROP 30% AS AI TOOLS FLOOD MARKET",
        "COMPANY REPLACES ENTIRE QA TEAM WITH AI AGENT",
        "SIDE HUSTLE ECONOMY GROWS TO $1.4 TRILLION",
        "AVERAGE STUDENT LOAN DEBT NOW $47K",
        "INTERVIEW LOOPS NOW 8 ROUNDS ON AVERAGE",
        "CEO PAY RATIO HITS 400:1",
        "SUBSCRIPTION FATIGUE: AVERAGE HOUSEHOLD PAYS $237/MO",
        "AI CHATBOT PASSES GOOGLE L5 INTERVIEW",
        "HOUSING WAITLISTS IN SF NOW 18 MONTHS",
        "PLUMBER EARNS MORE THAN SENIOR ENGINEER IN 12 STATES",
        "MENTAL HEALTH APPS UP 200% AMONG TECH WORKERS",
        "RETURN-TO-OFFICE MANDATES TRIGGER MASS RESIGNATIONS",
    ];

    let sep = "  \u{25b8}  ";
    let full: String = HEADLINES.iter().map(|h| format!("{}{}", h, sep)).collect();
    let full_len = full.len();
    if full_len == 0 {
        return;
    }

    let w = area.width as usize;
    let offset = ((tick as usize) * scroll_speed) % full_len;
    let doubled = format!("{}{}", full, full); // wrap around
    let visible: String = doubled.chars().skip(offset).take(w).collect();

    frame.render_widget(
        Paragraph::new(Span::styled(
            visible,
            Style::default().fg(AMBER).bg(Color::Rgb(0x04, 0x04, 0x10)),
        )),
        area,
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Character selection
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_pick_class(frame: &mut Frame, app: &App, area: Rect, selected: usize) {
    let Ok(catalog) = CharacterCatalog::embedded() else {
        let msg = Paragraph::new(Span::styled(
            "Failed to load character catalog. Check data/characters.jsonl.",
            Style::default().fg(RED).bg(BG),
        ))
        .alignment(Alignment::Center);
        frame.render_widget(msg, ui::centered(area, 70, 1));
        return;
    };
    let characters = catalog.all();
    if characters.is_empty() {
        return;
    }

    let num = characters.len();
    let sel_idx = selected.min(num.saturating_sub(1));
    let def: &CharacterDef = &characters[sel_idx];
    let s = &def.starting_stats;
    let accent = ui::character_accent(def);
    let bg = BG_PANEL;

    // Modal sizing: wider to support roster + dossier columns.
    let mw = area.width.saturating_sub(10).min(150);
    let mh = area.height.saturating_sub(4).min(40);
    let mx = area.x + area.width.saturating_sub(mw) / 2;
    let my = area.y + area.height.saturating_sub(mh) / 2;
    let modal = Rect {
        x: mx,
        y: my,
        width: mw,
        height: mh,
    };

    let vt = app.visual_tempo();
    dim_outside_modal(frame, app.tick, area, modal, vt.rain_speed, vt.rain_trail);

    frame.render_widget(Paragraph::new("").style(Style::default().bg(bg)), modal);

    let inner = Rect {
        x: modal.x + 1,
        y: modal.y + 1,
        width: modal.width.saturating_sub(2),
        height: modal.height.saturating_sub(2),
    };
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(1)])
        .split(inner);
    let body = rows[0];
    let footer = rows[1];

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(68),
            Constraint::Length(1),
            Constraint::Percentage(32),
        ])
        .split(body);
    let roster_rect = cols[0];
    let dossier_rect = cols[2];

    let roster_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(BORDER_HI))
        .title(Span::styled(
            " SELECT YOUR CHARACTER ",
            Style::default()
                .fg(AMBER)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(bg));
    frame.render_widget(roster_block, roster_rect);

    let roster_inner = Rect {
        x: roster_rect.x + 1,
        y: roster_rect.y + 1,
        width: roster_rect.width.saturating_sub(2),
        height: roster_rect.height.saturating_sub(2),
    };

    if roster_inner.width > 0 && roster_inner.height > 0 {
        let grid_cols = (5usize).min(num).max(1);
        let grid_rows = num.div_ceil(grid_cols);
        let cell_w = (roster_inner.width / (grid_cols as u16)).max(1);
        let cell_h = (roster_inner.height / (grid_rows as u16)).max(1);
        let leftover_h = roster_inner
            .height
            .saturating_sub(cell_h.saturating_mul(grid_rows as u16));

        for (idx, character) in characters.iter().enumerate() {
            let row = idx / grid_cols;
            let col = idx % grid_cols;

            let x = roster_inner.x + (col as u16) * cell_w;
            // Distribute leftover lines between rows so the grid reaches the bottom.
            let row_offset = (row as u16).min(leftover_h);
            let y = roster_inner.y + (row as u16) * cell_h + row_offset;
            let width = if col == grid_cols - 1 {
                roster_inner
                    .width
                    .saturating_sub((col as u16).saturating_mul(cell_w))
            } else {
                cell_w
            };
            let height = cell_h;
            if width == 0 || height == 0 {
                continue;
            }

            let is_sel = idx == sel_idx;
            let cell_accent = ui::character_accent(character);
            let card = Rect {
                x,
                y,
                width,
                height,
            };

            frame.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(if is_sel {
                        BorderType::Double
                    } else {
                        BorderType::Plain
                    })
                    .border_style(Style::default().fg(if is_sel { cell_accent } else { BORDER }))
                    .style(Style::default().bg(bg)),
                card,
            );

            let card_inner = Rect {
                x: card.x + 1,
                y: card.y + 1,
                width: card.width.saturating_sub(2),
                height: card.height.saturating_sub(2),
            };
            if card_inner.width == 0 || card_inner.height == 0 {
                continue;
            }

            // Keep labels compact and dedicate most of each tile to portrait.
            let label_h = 1;
            let label_y = card_inner.y + card_inner.height.saturating_sub(label_h);
            let portrait_zone_h = card_inner.height.saturating_sub(label_h);

            if portrait_zone_h > 0 {
                let mut portrait_lines = if is_sel {
                    ui::character_sprite_1_5x(
                        character,
                        cell_accent,
                        TEXT_DIM,
                        BORDER_HI,
                        TEXT_HI,
                        bg,
                    )
                } else {
                    ui::character_sprite_1_5x(character, TEXT_DIM, TEXT_DIM, BORDER, TEXT_DIM, bg)
                };
                let portrait_h = portrait_zone_h.min(10);
                portrait_lines.truncate(portrait_h as usize);
                let portrait_y = card_inner.y + portrait_zone_h.saturating_sub(portrait_h) / 2;
                frame.render_widget(
                    Paragraph::new(Text::from(portrait_lines)).alignment(Alignment::Center),
                    Rect {
                        x: card_inner.x,
                        y: portrait_y,
                        width: card_inner.width,
                        height: portrait_h,
                    },
                );
            }

            let mut label_lines: Vec<Line> = Vec::new();
            label_lines.push(Line::from(Span::styled(
                ui::truncate(character.label.as_str(), card_inner.width as usize),
                Style::default()
                    .fg(if is_sel { TEXT_HI } else { TEXT })
                    .bg(bg)
                    .add_modifier(if is_sel {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )));
            frame.render_widget(
                Paragraph::new(label_lines).alignment(Alignment::Center),
                Rect {
                    x: card_inner.x,
                    y: label_y,
                    width: card_inner.width,
                    height: label_h,
                },
            );
        }
    }

    let dossier_title = ui::truncate(
        &format!(" {} {} ", ui::character_tag(def), def.label),
        dossier_rect.width.saturating_sub(4) as usize,
    );
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(accent))
            .title(Span::styled(
                dossier_title,
                Style::default()
                    .fg(accent)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(bg)),
        dossier_rect,
    );

    let dossier_inner = Rect {
        x: dossier_rect.x + 2,
        y: dossier_rect.y + 2,
        width: dossier_rect.width.saturating_sub(4),
        height: dossier_rect.height.saturating_sub(4),
    };

    if dossier_inner.width > 0 && dossier_inner.height > 0 {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(16), Constraint::Min(3)])
            .split(dossier_inner);
        let summary_rect = sections[0];
        let story_rect = sections[1];

        let mut summary_lines: Vec<Line> = Vec::new();
        let summary_w = summary_rect.width as usize;
        let prefix_w = 15usize;
        let right_margin = 0usize;
        let bar_w = summary_w.saturating_sub(prefix_w + right_margin);
        let kv_row = |label: &str, value: String, value_style: Style| -> Line<'static> {
            let left = label.to_string();
            let gap = summary_w.saturating_sub(left.chars().count() + value.chars().count());
            Line::from(vec![
                Span::styled(left, Style::default().fg(TEXT_DIM).bg(bg)),
                Span::styled(" ".repeat(gap), Style::default().bg(bg)),
                Span::styled(value, value_style),
            ])
        };
        let stat_bar = |label: &str, val: i32, color: Color| -> Line<'static> {
            let prefix = format!("{:<10} {:>3} ", label, val);
            let mut spans = vec![Span::styled(prefix, Style::default().fg(color).bg(bg))];
            spans.extend(ui::bar_spans(val, bar_w, color, TEXT_DIM, bg));
            Line::from(spans)
        };

        summary_lines.push(Line::from(Span::styled(
            ui::character_quote(def),
            Style::default().fg(TEXT_DIM).bg(bg),
        )));
        summary_lines.push(Line::from(""));
        summary_lines.push(kv_row(
            "SAVINGS",
            format!("${}", ui::fmt_dollars(s.money)),
            Style::default()
                .fg(GREEN)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ));
        summary_lines.push(kv_row(
            "INCOME",
            format!("+${}/turn", ui::fmt_dollars(def.starting_income_per_turn)),
            Style::default().fg(GREEN).bg(bg),
        ));
        summary_lines.push(kv_row(
            "EXPENSES",
            format!("-${}/turn", ui::fmt_dollars(def.starting_rent_per_turn)),
            Style::default().fg(RED).bg(bg),
        ));
        summary_lines.push(Line::from(""));
        summary_lines.push(stat_bar("SANITY:", s.sanity, AMBER));
        summary_lines.push(stat_bar("AUDIENCE:", s.audience, MAGENTA));
        summary_lines.push(stat_bar("AI SKILL:", s.ai_skill, CYAN));
        summary_lines.push(stat_bar("FRIENDS:", s.friends, GREEN));
        summary_lines.push(kv_row(
            "AI SUBSCRIPTION",
            def.starting_subscription.label().to_string(),
            Style::default().fg(TEXT).bg(bg),
        ));

        frame.render_widget(
            Paragraph::new(summary_lines),
            Rect {
                x: summary_rect.x,
                y: summary_rect.y,
                width: summary_rect.width,
                height: summary_rect.height,
            },
        );

        let story = Text::from(vec![
            Line::from(Span::styled(
                "BACKSTORY",
                Style::default()
                    .fg(accent)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                ui::character_backstory(def),
                Style::default().fg(TEXT).bg(bg),
            )),
        ]);
        let sprite_lines = ui::character_sprite_1_5x(def, accent, TEXT_DIM, BORDER_HI, TEXT_HI, bg);
        let sprite_h = sprite_lines.len() as u16;
        let show_sprite = story_rect.height >= sprite_h.saturating_add(6);
        if show_sprite {
            frame.render_widget(
                Paragraph::new(Text::from(sprite_lines)).alignment(Alignment::Center),
                Rect {
                    x: story_rect.x,
                    y: story_rect.y,
                    width: story_rect.width,
                    height: sprite_h.min(story_rect.height),
                },
            );
            frame.render_widget(
                Paragraph::new(story).wrap(Wrap { trim: true }),
                Rect {
                    x: story_rect.x,
                    y: story_rect.y + sprite_h + 1,
                    width: story_rect.width,
                    height: story_rect.height.saturating_sub(sprite_h + 1),
                },
            );
        } else {
            frame.render_widget(
                Paragraph::new(story).wrap(Wrap { trim: true }),
                Rect {
                    x: story_rect.x,
                    y: story_rect.y,
                    width: story_rect.width,
                    height: story_rect.height,
                },
            );
        }
    }

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            app.help_line(),
            Style::default().fg(TEXT_DIM).bg(bg),
        ))),
        Rect {
            x: footer.x,
            y: footer.y,
            width: footer.width,
            height: 1,
        },
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Turn screen (main gameplay)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_turn(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    events: &TurnEvents,
    input: &crate::app::InputState,
) {
    let game = match &app.game {
        Some(g) => g,
        None => {
            return;
        }
    };

    // 1 row top padding.
    let area = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: area.height.saturating_sub(2),
    };

    // Layout: main content (fill) | ticker(1) | footer(1).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    // Row 1: Main content split left/right.
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(MAIN_LEFT_PCT),
            Constraint::Percentage(MAIN_RIGHT_PCT),
        ])
        .split(rows[0]);

    render_turn_left(frame, app, cols[0], game, events, input);
    render_sidebar(frame, app, cols[1], game);

    // Row 2: Ticker bar.
    render_ticker_bar(frame, app, rows[1], game);

    // Row 3: Footer bar.
    render_footer(frame, app, rows[2], game);
}

fn render_ticker_bar(frame: &mut Frame, _app: &App, area: Rect, game: &crate::engine::game::Game) {
    let scrolling = game.state.ticker.view(area.width as usize);

    let para = Paragraph::new(Line::from(vec![Span::styled(
        scrolling,
        Style::default().fg(AMBER).bg(BG_TICKER),
    )]));
    frame.render_widget(para, area);
}

fn render_turn_left(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    game: &crate::engine::game::Game,
    events: &TurnEvents,
    input: &crate::app::InputState,
) {
    let block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default().bg(BG_PANEL));
    frame.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width.saturating_sub(2),
        height: area.height,
    };

    let w = inner.width as usize;
    let mut y = inner.y;

    // ── Block-letter title (glitchy, muted) ──
    {
        for i in 0..3 {
            let art = format!(
                "{}  {}",
                ui::TITLE_ART_PERMANENT[i],
                ui::TITLE_ART_UNDERCLASS[i]
            );
            let vt = app.visual_tempo();
            let glitched = ui::glitch_line_tempo(
                &art,
                app.tick,
                20 + (i as u64),
                vt.glitch_every,
                vt.glitch_rate,
            );
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    glitched,
                    Style::default()
                        .fg(Color::Rgb(0x1a, 0x1a, 0x2a))
                        .bg(BG_PANEL),
                ))),
                Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                },
            );
            y += 1;
        }
        y += 1; // spacing below title
    }

    // ── TURN EVENT box (personal event) ──
    {
        let min_after = 12u16; // choice cards
        let available_h = inner.height.saturating_sub(y - inner.y + min_after);

        // Sprite dimensions.
        let sprite_w = 24u16;
        let sprite_h = 10u16;
        let show_icon = available_h >= sprite_h + 2 && inner.width >= sprite_w + 20;

        // Box height: sprite + 2 border rows, or 5 rows for text-only + 2 border.
        let box_h = if show_icon {
            (sprite_h + 4).min(available_h)
        } else {
            (5u16).min(available_h)
        };

        if box_h >= 4 {
            let turn_prefix = format!(
                " Turn {} / {} \u{2500} ",
                game.state.turn, game.state.max_turns
            );
            let event_title_max = w.saturating_sub(turn_prefix.chars().count() + 1);
            let title_text = format!(
                "{}{} ",
                turn_prefix,
                ui::truncate(&events.personal.title, event_title_max)
            );
            let event_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(BORDER_HI))
                .title(Span::styled(
                    title_text,
                    Style::default()
                        .fg(AMBER)
                        .bg(BG_PANEL)
                        .add_modifier(Modifier::BOLD),
                ))
                .title_alignment(Alignment::Left)
                .style(Style::default().bg(BG_PANEL));
            let box_rect = Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: box_h,
            };
            frame.render_widget(event_block, box_rect);

            let box_inner = Rect {
                x: inner.x + 2,
                y: y + 1,
                width: inner.width.saturating_sub(4),
                height: box_h.saturating_sub(2),
            };
            let body_pad_x = 1u16;
            let body_pad_top = 1u16;

            if show_icon {
                let text_w = box_inner.width.saturating_sub(sprite_w + 1);
                let text_rect = Rect {
                    x: box_inner.x + body_pad_x,
                    y: box_inner.y + body_pad_top,
                    width: text_w.saturating_sub(body_pad_x * 2),
                    height: box_inner.height.saturating_sub(body_pad_top),
                };
                let body_text = ui::sentences_to_lines(&events.personal.body);

                if text_rect.width > 0 && text_rect.height > 0 {
                    let p = Paragraph::new(body_text)
                        .style(Style::default().fg(TEXT).bg(BG_PANEL))
                        .alignment(Alignment::Left)
                        .wrap(Wrap { trim: true });
                    frame.render_widget(p, text_rect);
                }

                let sprite = crate::sprites::personal_event_sprite(&events.personal, BG_PANEL);
                frame.render_widget(
                    Paragraph::new(Text::from(sprite))
                        .style(Style::default().bg(BG_PANEL))
                        .alignment(ratatui::layout::Alignment::Center),
                    Rect {
                        x: box_inner.x + text_w + 1,
                        y: box_inner.y + box_inner.height.saturating_sub(sprite_h) / 2,
                        width: sprite_w.min(box_inner.width.saturating_sub(text_w + 1)),
                        height: sprite_h.min(box_inner.height),
                    },
                );
            } else {
                let text_rect = Rect {
                    x: box_inner.x + body_pad_x,
                    y: box_inner.y + body_pad_top,
                    width: box_inner.width.saturating_sub(body_pad_x * 2),
                    height: box_inner.height.saturating_sub(body_pad_top),
                };
                let body_text = ui::sentences_to_lines(&events.personal.body);
                let p = Paragraph::new(body_text)
                    .style(Style::default().fg(TEXT).bg(BG_PANEL))
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: true });
                frame.render_widget(p, text_rect);
            }

            y += box_h;
        }
    }

    // ── Choice cards ──
    let remaining = inner.height.saturating_sub(y - inner.y);
    if remaining >= 12 {
        render_choice_cards(
            frame,
            app,
            game,
            events,
            inner.x,
            y,
            inner.width,
            remaining,
            input,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_choice_cards(
    frame: &mut Frame,
    app: &App,
    game: &crate::engine::game::Game,
    events: &TurnEvents,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    input: &crate::app::InputState,
) {
    let choices = &events.personal.choices;
    let num_choices = choices.len().min(3);
    if num_choices == 0 {
        return;
    }

    let has_freeform = events.personal.allow_freeform;
    let total_cards = num_choices + (if has_freeform { 1 } else { 0 });
    let card_w = width / (total_cards as u16);
    // Cards fill available space minus 1 row for the ability status line below.
    let card_h = height.saturating_sub(1);

    let cy = y;

    // Render choice cards.
    for (i, choice) in choices.iter().take(3).enumerate() {
        let cx = x + (i as u16) * card_w;
        let is_sel = input.mode == InputMode::Cards && input.selected == i;
        let available = choice.is_available(game.state.stats.money);

        let border_color = if is_sel && available {
            AMBER
        } else if !available {
            BORDER
        } else if input.mode == InputMode::Cards {
            BORDER_HI
        } else {
            BORDER
        };

        let border_title = format!("[{}]", choice.key);

        let title_color = if is_sel && available {
            WHITE
        } else if available {
            AMBER
        } else {
            TEXT_DIM
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(if is_sel {
                BorderType::Double
            } else {
                BorderType::Plain
            })
            .border_style(Style::default().fg(border_color).bg(BG_PANEL))
            .title(Span::styled(
                border_title,
                Style::default().fg(title_color).bg(BG_PANEL),
            ))
            .title_alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().bg(BG_PANEL));

        let actual_w = card_w.min(width.saturating_sub(cx - x));
        let card_rect = Rect {
            x: cx,
            y: cy,
            width: actual_w,
            height: card_h,
        };
        frame.render_widget(block, card_rect);

        let ih = card_h.saturating_sub(2);
        let text_pad_x = 1u16;

        if !available {
            // Locked card: show requirement
            let mut lock_lines: Vec<Line> = vec![Line::from(vec![Span::styled(
                "LOCKED",
                Style::default()
                    .fg(RED)
                    .bg(BG_PANEL)
                    .add_modifier(Modifier::BOLD),
            )])];
            if let Some(min) = choice.min_money {
                lock_lines.push(Line::from(vec![Span::styled(
                    format!("REQ: ${}", ui::fmt_dollars(min)),
                    Style::default().fg(RED).bg(BG_PANEL),
                )]));
            }
            frame.render_widget(
                Paragraph::new(Text::from(lock_lines)),
                Rect {
                    x: cx + 1 + text_pad_x,
                    y: cy + 1,
                    width: actual_w.saturating_sub(2 + text_pad_x * 2),
                    height: ih,
                },
            );
        } else {
            let accent_color = if is_sel { AMBER } else { TEXT };
            let sprite =
                ui::choice_item_sprite(&events.personal, i, accent_color, TEXT_DIM, BG_PANEL);

            let card_lines: Vec<Line> = if is_sel {
                sprite
            } else {
                // Desaturate sprite to grayscale when not selected
                sprite
                    .into_iter()
                    .map(|line| {
                        Line::from(
                            line.spans
                                .into_iter()
                                .map(|span| {
                                    let fg = match span.style.fg {
                                        Some(Color::Rgb(r, g, b)) => {
                                            let lum =
                                                (((r as u16) + (g as u16) + (b as u16)) / 3) as u8;
                                            // Dampen further toward dark
                                            let v = (lum / 2).max(0x20);
                                            Color::Rgb(v, v, v)
                                        }
                                        other => other.unwrap_or(TEXT_DIM),
                                    };
                                    Span::styled(span.content, span.style.fg(fg))
                                })
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect()
            };

            // Fixed card layout:
            // sprite region (top) + 1 row gap + fixed text region (6 rows).
            let title_style = if is_sel {
                Style::default()
                    .fg(WHITE)
                    .bg(BG_PANEL)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TEXT_DIM).bg(BG_PANEL)
            };
            let sprite_h = card_lines.len() as u16;
            let inner_w = actual_w.saturating_sub(2);
            let title_rows = 6u16;
            let title_top_pad = 1u16;
            let title_h = title_rows.min(ih);
            let sprite_region_h = ih.saturating_sub(title_top_pad + title_h);
            let vis_sprite_h = sprite_h.min(sprite_region_h);
            let sprite_y = cy + 1 + sprite_region_h.saturating_sub(vis_sprite_h) / 2;

            frame.render_widget(
                Paragraph::new(Text::from(card_lines))
                    .alignment(ratatui::layout::Alignment::Center),
                Rect {
                    x: cx + 1,
                    y: sprite_y,
                    width: inner_w,
                    height: vis_sprite_h,
                },
            );

            let title_x = cx + 1 + text_pad_x;
            let title_y = cy + 1 + sprite_region_h + title_top_pad;
            let title_w = inner_w.saturating_sub(text_pad_x * 2);
            if title_h > 0 && title_w > 0 {
                frame.render_widget(
                    Paragraph::new(choice.title.clone())
                        .style(title_style)
                        .wrap(Wrap { trim: true })
                        .alignment(ratatui::layout::Alignment::Center),
                    Rect {
                        x: title_x,
                        y: title_y,
                        width: title_w,
                        height: title_h,
                    },
                );
            }
        }
    }

    // ── 4th card: freeform input ──
    if has_freeform {
        let i = num_choices;
        let cx = x + (i as u16) * card_w;
        let is_sel = input.mode == InputMode::Cards && input.selected == i;
        let is_active = input.mode == InputMode::Freeform;

        let border_color = if is_sel || is_active {
            AMBER
        } else {
            BORDER_HI
        };
        let title_color = if is_sel || is_active { WHITE } else { AMBER };

        let actual_w = card_w.min(width.saturating_sub(cx - x));
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(if is_sel || is_active {
                BorderType::Double
            } else {
                BorderType::Plain
            })
            .border_style(Style::default().fg(border_color).bg(BG_PANEL))
            .title(Span::styled(
                "[4]",
                Style::default().fg(title_color).bg(BG_PANEL),
            ))
            .title_alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().bg(BG_PANEL));

        let card_rect = Rect {
            x: cx,
            y: cy,
            width: actual_w,
            height: card_h,
        };
        frame.render_widget(block, card_rect);

        let ih = card_h.saturating_sub(2);
        let text_pad_x = 1u16;
        let title_rows = 6u16;
        let title_top_pad = 1u16;
        let text_h = title_rows.min(ih);
        let sprite_region_h = ih.saturating_sub(title_top_pad + text_h);
        let inner_rect = Rect {
            x: cx + 1 + text_pad_x,
            y: cy + 1 + sprite_region_h + title_top_pad,
            width: actual_w.saturating_sub(2 + text_pad_x * 2),
            height: text_h,
        };

        let mut lines: Vec<Line> = Vec::new();
        if is_active {
            let cursor = if app.tick % 6 < 3 { "_" } else { " " };
            lines.push(Line::from(vec![
                Span::styled("> ", Style::default().fg(AMBER).bg(BG_PANEL)),
                Span::styled(&input.buffer, Style::default().fg(TEXT_HI).bg(BG_PANEL)),
                Span::styled(cursor, Style::default().fg(AMBER).bg(BG_PANEL)),
            ]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                "Press [Enter] to type",
                Style::default().fg(TEXT_DIM).bg(BG_PANEL),
            )]));
            lines.push(Line::from(vec![Span::styled(
                "your own action",
                Style::default().fg(TEXT_DIM).bg(BG_PANEL),
            )]));
        }
        let p = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: true });
        frame.render_widget(p, inner_rect);
    }

    // (ability status line removed)
    let info_y = cy + card_h;
    let info_remaining = height.saturating_sub(card_h);
    if info_remaining == 0 {
        return;
    }

    // Placeholder to keep layout stable; render nothing visible.
    let p = Paragraph::new(Text::from(vec![]));
    frame.render_widget(
        p,
        Rect {
            x,
            y: info_y,
            width,
            height: info_remaining,
        },
    );
}

fn render_sidebar(frame: &mut Frame, app: &App, area: Rect, game: &crate::engine::game::Game) {
    // ── AI PROGRESS in the header area (above character box) ──
    let top_pad = 4u16;
    {
        let ai_pct = game.state.ai.percent_string();
        let ai_x10 = game.state.ai.x10();
        let delta_x10 =
            crate::engine::ai::AiCapability::tick_delta_x10(game.state.turn, game.state.max_turns);
        let ai_rate = format!("+{}.{}%/turn", delta_x10 / 10, delta_x10 % 10);
        let bar_x = area.x + 1;
        let bar_w_px = area.width.saturating_sub(2);
        let bar_w = bar_w_px as usize;
        let filled = ((ai_x10 as usize) * bar_w) / 1000;
        let empty_count = bar_w.saturating_sub(filled);

        // Row 0: label + rate + percentage
        let label = "AI PROGRESS";
        let right = format!("{} {}", ai_rate, ai_pct);
        let gap = bar_w.saturating_sub(label.len() + right.len());
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    label.to_string(),
                    Style::default()
                        .fg(CYAN)
                        .bg(BG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ".repeat(gap), Style::default().bg(BG)),
                Span::styled(
                    ai_rate,
                    Style::default().fg(Color::Rgb(0x55, 0x55, 0x66)).bg(BG),
                ),
                Span::styled(format!(" {}", ai_pct), Style::default().fg(CYAN).bg(BG)),
            ])),
            Rect {
                x: bar_x,
                y: area.y,
                width: bar_w_px,
                height: 1,
            },
        );

        // Row 1: full-width progress bar
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("█".repeat(filled), Style::default().fg(CYAN).bg(BG)),
                Span::styled(
                    "░".repeat(empty_count),
                    Style::default().fg(TEXT_DIM).bg(BG),
                ),
            ])),
            Rect {
                x: bar_x,
                y: area.y + 1,
                width: bar_w_px,
                height: 1,
            },
        );
    }

    let bottom_pad = 1u16;
    let box_rect = Rect {
        x: area.x,
        y: area.y + top_pad,
        width: area.width,
        height: area.height.saturating_sub(top_pad + bottom_pad),
    };

    let character = CharacterDef::for_character(game.state.character.clone());
    let class_accent = ui::character_accent(&character);
    let class_title = format!(" {} ", character.label.to_uppercase());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(BORDER_HI))
        .title(Span::styled(
            class_title,
            Style::default()
                .fg(class_accent)
                .bg(BG_PANEL)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(BG_PANEL));
    frame.render_widget(block, box_rect);

    // Sidebar content padding: add a little breathing room without collapsing narrow layouts.
    let pad_x = if box_rect.width >= 44 { 3 } else { 2 };
    let pad_top = 2u16;
    let pad_bottom = 1u16;
    let inner = Rect {
        x: box_rect.x + pad_x,
        y: box_rect.y + pad_top,
        width: box_rect.width.saturating_sub(pad_x * 2),
        height: box_rect.height.saturating_sub(pad_top + pad_bottom),
    };

    let mut lines: Vec<Line> = Vec::new();
    let stats = &game.state.stats;
    let (income, rent, sub, _net) = ui::net_cashflow(game);

    // During stat-pause, withhold the delta so bars visually "move" when it expires.
    let pending = app.stat_pause_pending_delta();
    let d_money = pending.map_or(0, |d| d.money);
    let d_sanity = pending.map_or(0, |d| d.sanity);
    let d_audience = pending.map_or(0, |d| d.audience);
    let d_ai_skill = pending.map_or(0, |d| d.ai_skill);
    let d_friends = pending.map_or(0, |d| d.friends);
    let vis_money = stats.money - d_money;
    let vis_sanity = stats.sanity - d_sanity;
    let vis_audience = stats.audience - d_audience;
    let vis_ai_skill = stats.ai_skill - d_ai_skill;
    let vis_friends = stats.friends - d_friends;

    // ── Character sprite with idle animation ──
    let sprite_accent = ui::character_accent(&character);
    let sprite = ui::character_sprite_1_5x_animated(
        &character,
        app.tick,
        sprite_accent,
        TEXT_DIM,
        BORDER,
        TEXT_HI,
        BG,
    );
    let quote = ui::character_quote(&character);
    let bubble_line = sprite.len() / 3; // near the mouth area of sprite
    let bubble_line_w: usize = sprite
        .get(bubble_line)
        .map(|l| l.spans.iter().map(|s| s.content.chars().count()).sum())
        .unwrap_or(0);
    let bubble_prefix_w = bubble_line_w + 3; // sprite line + " ◂ "
    for (i, line) in sprite.iter().enumerate() {
        if i == bubble_line {
            let mut spans: Vec<Span<'static>> = line.spans.clone();
            let avail = (inner.width as usize).saturating_sub(bubble_prefix_w);
            if avail > 4 {
                spans.push(Span::styled(
                    " ◂ ",
                    Style::default().fg(class_accent).bg(BG),
                ));
                let full_chars: Vec<char> = quote.chars().collect();
                let text = if full_chars.len() <= avail {
                    quote.to_string()
                } else {
                    let mut t: String = full_chars[..avail.saturating_sub(1)].iter().collect();
                    t.push('…');
                    t
                };
                spans.push(Span::styled(text, Style::default().fg(TEXT_DIM).bg(BG)));
            }
            lines.push(Line::from(spans));
        } else {
            lines.push(line.clone());
        }
    }
    push_double_sep(&mut lines, inner.width);

    // ── All stats in one block ──
    let iw = inner.width as usize;

    // Money / income / expenses — labels left, amounts right-aligned
    let money_row = |label: &str, val_str: &str, bold: bool, color: Color| -> Line<'static> {
        let fill = iw.saturating_sub(label.len() + val_str.len());
        let mut style = Style::default().fg(color).bg(BG);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        Line::from(vec![
            Span::styled(label.to_string(), style),
            Span::styled(" ".repeat(fill), Style::default().bg(BG)),
            Span::styled(val_str.to_string(), style),
        ])
    };
    lines.push(money_row(
        "MONEY",
        &format!("${}", ui::fmt_dollars(vis_money)),
        true,
        if vis_money > 0 { GREEN } else { RED },
    ));
    lines.push(money_row(
        "income (/turn)",
        &format!("+${}", ui::fmt_dollars(income)),
        false,
        GREEN,
    ));
    lines.push(money_row(
        "expenses (/turn)",
        &format!("-${}", ui::fmt_dollars(rent + sub)),
        false,
        RED,
    ));

    push_double_sep(&mut lines, inner.width);

    let stat_bar = |label: &str, val: i32, color: Color| -> Line<'static> {
        let prefix = format!("{:<8} {:>3} ", label, val);
        let bar_w = iw.saturating_sub(13); // 8 label + 1 space + 3 val + 1 space
        let mut spans = vec![Span::styled(prefix, Style::default().fg(color).bg(BG))];
        spans.extend(ui::bar_spans(val, bar_w, color, TEXT_DIM, BG));
        Line::from(spans)
    };

    lines.push(stat_bar("SANITY", vis_sanity, AMBER));
    lines.push(stat_bar("AUDIENCE", vis_audience, MAGENTA));
    lines.push(stat_bar("AI SKILL", vis_ai_skill, CYAN));
    lines.push(stat_bar("FRIENDS", vis_friends, GREEN));

    // Divider.
    push_double_sep(&mut lines, inner.width);

    // ── Subscription (label left, value right) ──
    let sub_cost = game.state.subscription.cost_per_turn() * (game.state.money_scale() as i64);
    let sub_label = "AI SUB";
    let sub_tier = game.state.subscription.label();
    let sub_cost_str = format!("-${}/m", sub_cost);
    let sub_value_full = format!("{} {}", sub_tier, sub_cost_str);
    let fill = iw.saturating_sub(sub_label.len() + sub_value_full.len());
    lines.push(Line::from(vec![
        Span::styled(sub_label, Style::default().fg(TEXT_DIM).bg(BG)),
        Span::styled(" ".repeat(fill), Style::default().bg(BG)),
        Span::styled(sub_tier.to_string(), Style::default().fg(TEXT_HI).bg(BG)),
        Span::styled(" ", Style::default().bg(BG)),
        Span::styled(sub_cost_str, Style::default().fg(RED).bg(BG)),
    ]));

    if !game.state.run_memory.flags.is_empty() {
        let flag_registry = crate::engine::content::FlagRegistry::embedded().ok();
        let mut tags: Vec<&str> = game
            .state
            .run_memory
            .flags
            .iter()
            .map(|s| s.as_str())
            .collect();
        tags.sort_unstable();

        push_double_sep(&mut lines, inner.width);
        let mut row_spans: Vec<Span> = Vec::new();
        let mut row_width = 0usize;
        for tag in &tags {
            let tag_label = flag_registry
                .and_then(|registry| registry.label(tag))
                .map(str::to_string)
                .unwrap_or_else(|| tag.replace('_', " ").to_uppercase());
            let chip = format!("[{}]", tag_label);
            let chip_w = chip.chars().count();
            let needed = if row_spans.is_empty() {
                chip_w
            } else {
                chip_w + 1
            };
            if row_width + needed > iw && !row_spans.is_empty() {
                lines.push(Line::from(std::mem::take(&mut row_spans)));
                row_width = 0;
            }
            if row_width > 0 {
                row_spans.push(Span::styled(" ", Style::default().bg(BG)));
                row_width += 1;
            }
            row_spans.push(Span::styled(chip, Style::default().fg(TEXT_HI).bg(BG)));
            row_width += chip_w;
        }
        if !row_spans.is_empty() {
            lines.push(Line::from(row_spans));
        }
    }

    let text = Text::from(lines);
    let para = Paragraph::new(text);
    frame.render_widget(para, inner);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect, _game: &crate::engine::game::Game) {
    let mut left_spans: Vec<Span> = Vec::new();

    if let Some(status) = app.llm_status() {
        left_spans.push(Span::styled(
            format!(" {}", status),
            Style::default().fg(CYAN).bg(BG),
        ));
    }

    // Right side: help.
    let help = app.help_line();
    let left_len: usize = left_spans.iter().map(|s| s.content.len()).sum();
    let padding = (area.width as usize).saturating_sub(left_len + help.len() + 1);
    left_spans.push(Span::styled(" ".repeat(padding), Style::default().bg(BG)));
    left_spans.push(Span::styled(help, Style::default().fg(TEXT_DIM).bg(BG)));

    let footer = Paragraph::new(Line::from(left_spans));
    frame.render_widget(footer, area);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Outcome screen
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_outcome(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    events: &TurnEvents,
    outcome: &TurnOutcome,
) {
    if app.game.is_none() {
        return;
    }

    // Keep the gameplay screen visible underneath, then place outcome in a fixed-size modal.
    // Use out-of-range selected so no card appears highlighted behind the modal.
    let bg_input = crate::app::InputState {
        mode: InputMode::Cards,
        buffer: String::new(),
        selected: usize::MAX,
    };
    render_turn(frame, app, area, events, &bg_input);

    // Inner padding for the dynamic outcome modal.
    let content_pad_x = 1u16;
    let content_pad_top = 1u16;
    let content_pad_bottom = 1u16;
    let modal_w = FOCUS_MODAL_W.min(area.width.saturating_sub(4)).max(1);
    let inner_w = modal_w.saturating_sub(4 + content_pad_x * 2) as usize;
    let narrative_rows = wrap_text_rows(&outcome.narrative, inner_w.max(1));

    let mut lines: Vec<Line> = Vec::new();
    let bg = BG_PANEL;

    // ── Headline/description block ──
    for line_str in narrative_rows {
        lines.push(Line::from(vec![Span::styled(
            line_str,
            Style::default().fg(TEXT).bg(bg),
        )]));
    }

    lines.push(Line::from(""));

    // ── Compact delta summary: all stats, red/green, right-aligned ──
    let d = &outcome.delta;
    let iw = inner_w;
    let display_w = |s: &str| s.chars().count();

    let delta_row = |label: &str, dv: i64| -> Line<'static> {
        let (arrow, val_str, color) = if dv > 0 {
            ("▲", format!("+{}", dv), GREEN)
        } else if dv < 0 {
            ("▼", format!("{}", dv), RED)
        } else {
            ("─", "0".to_string(), TEXT_DIM)
        };
        let left = format!(" {} {}", arrow, label);
        let right = val_str;
        let dots_w = iw.saturating_sub(display_w(&left) + display_w(&right));
        let dots = "·".repeat(dots_w);
        vec![
            Span::styled(left, Style::default().fg(color).bg(bg)),
            Span::styled(dots, Style::default().fg(BORDER).bg(bg)),
            Span::styled(right, Style::default().fg(color).bg(bg)),
        ]
        .into()
    };

    // Money row with dollar formatting
    {
        let (arrow, val_str, color) = if d.money > 0 {
            ("▲", format!("+${}", ui::fmt_dollars(d.money)), GREEN)
        } else if d.money < 0 {
            ("▼", format!("-${}", ui::fmt_dollars(-d.money)), RED)
        } else {
            ("─", "$0".to_string(), TEXT_DIM)
        };
        let left = format!(" {} {}", arrow, "MONEY");
        let right = val_str;
        let dots_w = iw.saturating_sub(display_w(&left) + display_w(&right));
        let dots = "·".repeat(dots_w);
        lines.push(Line::from(vec![
            Span::styled(left, Style::default().fg(color).bg(bg)),
            Span::styled(dots, Style::default().fg(BORDER).bg(bg)),
            Span::styled(right, Style::default().fg(color).bg(bg)),
        ]));
    }
    lines.push(delta_row("SANITY", d.sanity as i64));
    lines.push(delta_row("AUDIENCE", d.audience as i64));
    lines.push(delta_row("AI SKILL", d.ai_skill as i64));
    lines.push(delta_row("FRIENDS", d.friends as i64));

    lines.push(Line::from(""));
    for row in wrap_text_rows(&outcome.summary_line, iw.max(1)) {
        lines.push(Line::from(vec![Span::styled(
            row,
            Style::default().fg(TEXT_DIM).bg(bg),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "[Enter] Continue",
        Style::default()
            .fg(AMBER)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
    )]));

    let content_rows = lines.len() as u16;
    let modal_h = (content_rows + 2 + content_pad_top + content_pad_bottom)
        .min(area.height.saturating_sub(2))
        .max(1);
    let modal = ui::centered(area, modal_w, modal_h);
    let vt = app.visual_tempo();
    dim_outside_modal(frame, app.tick, area, modal, vt.rain_speed, vt.rain_trail);
    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(AMBER))
        .title(Span::styled(
            " TURN RESULT ",
            Style::default()
                .fg(AMBER)
                .bg(BG_PANEL)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(BG_PANEL));
    frame.render_widget(block, modal);

    let inner = Rect {
        x: modal.x + 2 + content_pad_x,
        y: modal.y + 1 + content_pad_top,
        width: modal.width.saturating_sub(4 + content_pad_x * 2),
        height: modal
            .height
            .saturating_sub(2 + content_pad_top + content_pad_bottom),
    };

    let text = Text::from(lines);
    let para = Paragraph::new(text);
    frame.render_widget(para, inner);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Ending screen
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Per-ending accent color (used for rain + UI accents).
fn ending_style(id: EndingId) -> Color {
    match id {
        // Death — red
        EndingId::Broke | EndingId::BurnedOut => RED,
        // Secret — amber/brown
        EndingId::Luddite => Color::Rgb(0x88, 0x66, 0x44),
        // Triumph — bright green
        EndingId::Oligarch => Color::Rgb(0x00, 0xff, 0x88),
        EndingId::Irreplaceable => Color::Rgb(0x00, 0xcc, 0xff),
        EndingId::Builder => CYAN,
        // Bittersweet — purple/magenta
        EndingId::LonelyOligarch => Color::Rgb(0x99, 0x66, 0xff),
        EndingId::FamousForNoReason => MAGENTA,
        // Warm — amber/green
        EndingId::Connector => GREEN,
        EndingId::EnlightenedDropout => Color::Rgb(0x88, 0xff, 0x88),
        EndingId::Artist => Color::Rgb(0xff, 0x88, 0xcc),
        EndingId::Comfortable => AMBER,
        // Dark — dim/red
        EndingId::PermanentUnderclass => Color::Rgb(0xff, 0x66, 0x44),
        EndingId::Survivor => TEXT_DIM,
        // V1.1 endings
        EndingId::Sellout => Color::Rgb(0xff, 0x99, 0x33),
        EndingId::Whistleblower => Color::Rgb(0xff, 0x55, 0x55),
        EndingId::Phoenix => Color::Rgb(0xff, 0x77, 0x44),
        EndingId::Collaborator => Color::Rgb(0x00, 0xee, 0xdd),
        EndingId::Ghost => Color::Rgb(0x99, 0x99, 0xaa),
        EndingId::DebtSpiral => Color::Rgb(0xff, 0x44, 0x44),
        EndingId::Canceled => MAGENTA,
        EndingId::Expat => Color::Rgb(0x66, 0xcc, 0xff),
        EndingId::SlopMerchant => Color::Rgb(0xcc, 0x99, 0xff),
        EndingId::Unionized => Color::Rgb(0x66, 0xff, 0x66),
        EndingId::Scraped => Color::Rgb(0xff, 0xaa, 0x66),
    }
}

#[derive(Debug, Clone)]
struct EndingPreviewProfile {
    character: CharacterId,
    turn: u32,
    stats: crate::engine::types::Stats,
    ai_percent_x10: u32,
    subscription: SubscriptionTier,
}

#[derive(Debug, Clone)]
struct EndingDisplayProfile {
    origin_line: String,
    turn: u32,
    stats: crate::engine::types::Stats,
}

fn preview_profile_for_ending(id: EndingId) -> EndingPreviewProfile {
    use SubscriptionTier as S;

    let mut p = EndingPreviewProfile {
        character: CharacterId::new("junior_dev"),
        turn: Tempo::default().max_turns(),
        stats: crate::engine::types::Stats {
            money: 55_000,
            sanity: 55,
            audience: 50,
            ai_skill: 55,
            friends: 50,
        },
        ai_percent_x10: 920,
        subscription: S::Pro,
    };

    match id {
        EndingId::Broke => {
            p.stats.money = 0;
            p.stats.sanity = 28;
            p.stats.audience = 22;
            p.stats.ai_skill = 35;
            p.stats.friends = 30;
            p.ai_percent_x10 = 950;
            p.subscription = S::Ultra;
        }
        EndingId::BurnedOut => {
            p.character = CharacterId::new("podcaster");
            p.stats.money = 12_000;
            p.stats.sanity = 0;
            p.stats.audience = 58;
            p.stats.ai_skill = 61;
            p.stats.friends = 35;
            p.ai_percent_x10 = 665;
            p.subscription = S::Pro;
        }
        EndingId::Luddite => {
            p.character = CharacterId::new("plumber");
            p.stats.money = 18_000;
            p.stats.sanity = 62;
            p.stats.audience = 25;
            p.stats.ai_skill = 0;
            p.stats.friends = 48;
            p.ai_percent_x10 = 880;
            p.subscription = S::Free;
        }
        EndingId::Oligarch => {
            p.character = CharacterId::new("ex_faang_engineer");
            p.stats.money = 650_000;
            p.stats.sanity = 62;
            p.stats.audience = 68;
            p.stats.ai_skill = 83;
            p.stats.friends = 38;
            p.subscription = S::Ultra;
        }
        EndingId::LonelyOligarch => {
            p.character = CharacterId::new("ex_faang_engineer");
            p.stats.money = 690_000;
            p.stats.sanity = 40;
            p.stats.audience = 66;
            p.stats.ai_skill = 86;
            p.stats.friends = 8;
            p.subscription = S::Ultra;
        }
        EndingId::Irreplaceable => {
            p.character = CharacterId::new("plumber");
            p.stats.money = 145_000;
            p.stats.sanity = 74;
            p.stats.audience = 44;
            p.stats.ai_skill = 36;
            p.stats.friends = 64;
            p.subscription = S::Free;
            p.ai_percent_x10 = 900;
        }
        EndingId::Builder => {
            p.character = CharacterId::new("ex_faang_engineer");
            p.stats.money = 180_000;
            p.stats.sanity = 58;
            p.stats.audience = 64;
            p.stats.ai_skill = 79;
            p.stats.friends = 42;
            p.subscription = S::Pro;
        }
        EndingId::Connector => {
            p.character = CharacterId::new("marketing_manager");
            p.stats.money = 90_000;
            p.stats.sanity = 70;
            p.stats.audience = 72;
            p.stats.ai_skill = 52;
            p.stats.friends = 88;
            p.subscription = S::Pro;
        }
        EndingId::Comfortable => {
            p.character = CharacterId::new("junior_dev");
            p.stats.money = 55_000;
            p.stats.sanity = 66;
            p.stats.audience = 52;
            p.stats.ai_skill = 58;
            p.stats.friends = 57;
            p.subscription = S::Pro;
        }
        EndingId::EnlightenedDropout => {
            p.character = CharacterId::new("product_designer");
            p.stats.money = 15_000;
            p.stats.sanity = 88;
            p.stats.audience = 32;
            p.stats.ai_skill = 18;
            p.stats.friends = 84;
            p.subscription = S::Free;
        }
        EndingId::Artist => {
            p.character = CharacterId::new("product_designer");
            p.stats.money = 48_000;
            p.stats.sanity = 74;
            p.stats.audience = 76;
            p.stats.ai_skill = 34;
            p.stats.friends = 62;
            p.subscription = S::Pro;
        }
        EndingId::FamousForNoReason => {
            p.character = CharacterId::new("podcaster");
            p.stats.money = 800;
            p.stats.sanity = 42;
            p.stats.audience = 95;
            p.stats.ai_skill = 46;
            p.stats.friends = 34;
            p.subscription = S::Free;
        }
        EndingId::PermanentUnderclass => {
            p.character = CharacterId::new("junior_dev");
            p.stats.money = 3_200;
            p.stats.sanity = 33;
            p.stats.audience = 28;
            p.stats.ai_skill = 16;
            p.stats.friends = 29;
            p.subscription = S::Free;
        }
        EndingId::Survivor => {
            p.character = CharacterId::new("junior_dev");
            p.stats.money = 22_000;
            p.stats.sanity = 52;
            p.stats.audience = 45;
            p.stats.ai_skill = 44;
            p.stats.friends = 47;
            p.subscription = S::Pro;
        }
        EndingId::Sellout => {
            p.character = CharacterId::new("marketing_manager");
            p.stats.money = 310_000;
            p.stats.sanity = 42;
            p.stats.audience = 66;
            p.stats.ai_skill = 74;
            p.stats.friends = 18;
            p.subscription = S::Ultra;
        }
        EndingId::Whistleblower => {
            p.character = CharacterId::new("junior_dev");
            p.stats.money = 4_500;
            p.stats.sanity = 48;
            p.stats.audience = 82;
            p.stats.ai_skill = 57;
            p.stats.friends = 40;
            p.subscription = S::Free;
        }
        EndingId::Phoenix => {
            p.character = CharacterId::new("ex_faang_engineer");
            p.stats.money = 42_000;
            p.stats.sanity = 68;
            p.stats.audience = 49;
            p.stats.ai_skill = 58;
            p.stats.friends = 55;
            p.subscription = S::Pro;
        }
        EndingId::Collaborator => {
            p.character = CharacterId::new("product_designer");
            p.stats.money = 115_000;
            p.stats.sanity = 63;
            p.stats.audience = 61;
            p.stats.ai_skill = 78;
            p.stats.friends = 60;
            p.subscription = S::Pro;
        }
        EndingId::Ghost => {
            p.character = CharacterId::new("ex_faang_engineer");
            p.stats.money = 180_000;
            p.stats.sanity = 54;
            p.stats.audience = 10;
            p.stats.ai_skill = 64;
            p.stats.friends = 11;
            p.subscription = S::Free;
        }
        EndingId::DebtSpiral => {
            p.character = CharacterId::new("junior_dev");
            p.stats.money = 6_000;
            p.stats.sanity = 37;
            p.stats.audience = 30;
            p.stats.ai_skill = 44;
            p.stats.friends = 33;
            p.subscription = S::Ultra;
        }
        EndingId::Canceled => {
            p.character = CharacterId::new("podcaster");
            p.stats.money = 14_000;
            p.stats.sanity = 30;
            p.stats.audience = 88;
            p.stats.ai_skill = 56;
            p.stats.friends = 19;
            p.subscription = S::Pro;
        }
        EndingId::Expat => {
            p.character = CharacterId::new("product_designer");
            p.stats.money = 38_000;
            p.stats.sanity = 78;
            p.stats.audience = 40;
            p.stats.ai_skill = 55;
            p.stats.friends = 34;
            p.subscription = S::Free;
        }
        EndingId::SlopMerchant => {
            p.character = CharacterId::new("marketing_manager");
            p.stats.money = 160_000;
            p.stats.sanity = 34;
            p.stats.audience = 79;
            p.stats.ai_skill = 88;
            p.stats.friends = 24;
            p.subscription = S::Ultra;
        }
        EndingId::Unionized => {
            p.character = CharacterId::new("plumber");
            p.stats.money = 72_000;
            p.stats.sanity = 62;
            p.stats.audience = 52;
            p.stats.ai_skill = 41;
            p.stats.friends = 76;
            p.subscription = S::Free;
        }
        EndingId::Scraped => {
            p.character = CharacterId::new("product_designer");
            p.stats.money = 17_000;
            p.stats.sanity = 50;
            p.stats.audience = 67;
            p.stats.ai_skill = 45;
            p.stats.friends = 46;
            p.subscription = S::Pro;
        }
    }

    p
}

fn ending_display_profile(
    app: &App,
    ending_id: EndingId,
    preview_idx: Option<usize>,
) -> Option<EndingDisplayProfile> {
    // Prefer the real run profile whenever game state exists, even if a preview
    // index is active, so "normal" ending context is preserved in active runs.
    if let Some(game) = app.game.as_ref() {
        return Some(EndingDisplayProfile {
            origin_line: crate::engine::endings::origin_line(&game.state).to_string(),
            turn: game.state.turn,
            stats: game.state.stats,
        });
    }

    if preview_idx.is_some() {
        let p = preview_profile_for_ending(ending_id);
        let character_def = CharacterDef::for_character(p.character.clone());
        return Some(EndingDisplayProfile {
            origin_line: character_def.origin_line,
            turn: p.turn,
            stats: p.stats,
        });
    }

    None
}

fn render_ending(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    ending: &crate::engine::endings::Ending,
    preview_idx: Option<usize>,
) {
    let is_death = matches!(ending.id, EndingId::Broke | EndingId::BurnedOut);
    let accent = ending_style(ending.id);
    let bg = BG;
    let pad = "  ";
    let text_pad = "  ";
    let display_profile = ending_display_profile(app, ending.id, preview_idx);

    // ── Pre-compute content height so the modal fits snugly ──
    let modal_w = FOCUS_MODAL_W.min(area.width.saturating_sub(4)).max(1);
    let inner_w = modal_w.saturating_sub(4) as usize; // 2 pad each side

    let art_groups = ui::block_text_wrapped(&ending.title, inner_w - 4);
    let art_lines: usize = art_groups
        .iter()
        .map(|g| if g[0].is_empty() { 0 } else { 3 })
        .sum();
    let text_pad_w = text_pad.chars().count();
    let text_wrap_w = inner_w.saturating_sub(text_pad_w).max(1);
    let wrap_with_pad = |text: &str| -> Vec<String> {
        wrap_text_rows(text, text_wrap_w)
            .into_iter()
            .map(|row| format!("{}{}", text_pad, row))
            .collect()
    };
    let wrapped_body_lines: Vec<String> = ending.body.lines().flat_map(wrap_with_pad).collect();
    let body_lines = wrapped_body_lines.len();
    let has_stats = display_profile.is_some();
    let wrapped_origin_lines = display_profile
        .as_ref()
        .map(|display| wrap_with_pad(&display.origin_line))
        .unwrap_or_default();
    let wrapped_turn_lines = display_profile
        .as_ref()
        .map(|display| wrap_with_pad(&format!("You lasted {} turns.", display.turn)))
        .unwrap_or_default();
    let origin_lines = if has_stats {
        wrapped_origin_lines.len() + wrapped_turn_lines.len() + 1 // wrapped origin + wrapped turn + spacer
    } else {
        0
    };
    let stat_lines = if has_stats { 7 } else { 0 }; // blank + net worth + ai/sub + 4 bars

    // 1 blank + art + 1 blank + body + stats + 1 blank + 2 prompts + 1 bottom pad
    let content_h = 1 + art_lines + 1 + origin_lines + body_lines + stat_lines + 1 + 2 + 1;
    let modal_h = ((content_h as u16) + 2) // +2 for border
        .min(area.height.saturating_sub(2))
        .max(1);
    let modal = ui::centered(area, modal_w, modal_h);

    // ── Matrix rain background (skip behind modal) ──
    let vt = app.visual_tempo();
    render_matrix_rain_colored(
        frame,
        app.tick,
        area,
        modal,
        1,
        accent,
        vt.rain_speed,
        vt.rain_trail,
    );

    // ── Modal box ──
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent))
        .style(Style::default().bg(bg));
    frame.render_widget(block, modal);

    let content_pad_x = 1u16;
    let content_pad_top = 1u16;
    let content_pad_bottom = 1u16;
    let inner = Rect {
        x: modal.x + 2 + content_pad_x,
        y: modal.y + 1 + content_pad_top,
        width: modal.width.saturating_sub(4 + content_pad_x * 2),
        height: modal
            .height
            .saturating_sub(2 + content_pad_top + content_pad_bottom),
    };

    let mut lines: Vec<Line> = Vec::new();

    // Match title screen: empty line, then block art with "  " pad.
    lines.push(Line::from(""));

    // Ending title in block-letter ASCII art with glitch effect.
    let mut seed_idx = 0u64;
    for group in &art_groups {
        for row in group {
            lines.push(Line::from(Span::styled(
                ui::glitch_line_tempo(
                    &format!("{}{}", pad, row),
                    app.tick,
                    50 + seed_idx,
                    vt.glitch_every,
                    vt.glitch_rate,
                ),
                Style::default()
                    .fg(accent)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            )));
            seed_idx += 1;
        }
    }

    lines.push(Line::from(""));

    if display_profile.is_some() {
        for row in &wrapped_origin_lines {
            lines.push(Line::from(vec![Span::styled(
                row.clone(),
                Style::default().fg(accent).bg(bg),
            )]));
        }
        for row in &wrapped_turn_lines {
            lines.push(Line::from(vec![Span::styled(
                row.clone(),
                Style::default().fg(TEXT_DIM).bg(bg),
            )]));
        }
        lines.push(Line::from(""));
    }

    // Ending body (wrapped to modal width so preview text doesn't clip).
    for text_line in &wrapped_body_lines {
        lines.push(Line::from(vec![Span::styled(
            text_line.clone(),
            Style::default().fg(TEXT).bg(bg),
        )]));
    }

    // ── Stats with progress bars ──
    if let Some(display) = &display_profile {
        let s = &display.stats;
        lines.push(Line::from(""));

        let iw = inner.width as usize;
        let prefix_w = 17;
        let right_margin = 2;
        let bar_w = iw.saturating_sub(prefix_w + right_margin);

        let stat_bar = |label: &str, val: i32| -> Line<'static> {
            let prefix = format!("  {:<10} {:>3} ", label, val);
            let mut spans = vec![Span::styled(prefix, Style::default().fg(accent).bg(bg))];
            spans.extend(ui::bar_spans(val, bar_w, accent, TEXT_DIM, bg));
            Line::from(spans)
        };

        let money_str = format!("${}", ui::fmt_dollars(s.money));
        let label_part = format!("  {:<10} ", "NET WORTH:");
        let total_w = prefix_w + bar_w;
        let fill_w = total_w.saturating_sub(label_part.len() + money_str.len());
        lines.push(Line::from(vec![
            Span::styled(label_part, Style::default().fg(accent).bg(bg)),
            Span::styled(" ".repeat(fill_w), Style::default().bg(bg)),
            Span::styled(money_str, Style::default().fg(accent).bg(bg)),
        ]));
        lines.push(stat_bar("SANITY:", s.sanity));
        lines.push(stat_bar("AUDIENCE:", s.audience));
        lines.push(stat_bar("AI SKILL:", s.ai_skill));
        lines.push(stat_bar("FRIENDS:", s.friends));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(pad.to_string(), Style::default().bg(bg)),
        Span::styled(
            "\u{25b8} Enter",
            Style::default()
                .fg(if is_death { RED } else { GREEN })
                .bg(bg),
        ),
        Span::styled(": PLAY AGAIN", Style::default().fg(TEXT).bg(bg)),
    ]));
    lines.push(Line::from(vec![
        Span::styled(pad.to_string(), Style::default().bg(bg)),
        Span::styled("\u{25b8} Ctrl+C", Style::default().fg(RED).bg(bg)),
        Span::styled(": DISCONNECT", Style::default().fg(TEXT).bg(bg)),
    ]));

    let text = Text::from(lines);
    let para = Paragraph::new(text).style(Style::default().bg(bg));
    frame.render_widget(para, inner);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Subscription management
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_manage_sub(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    selected: usize,
    events: &TurnEvents,
    input: &crate::app::InputState,
) {
    let game = match &app.game {
        Some(g) => g,
        None => {
            return;
        }
    };

    // Render gameplay beneath the modal.
    render_turn(frame, app, area, events, input);

    let modal = fixed_modal_rect(area);
    let vt = app.visual_tempo();
    dim_outside_modal(frame, app.tick, area, modal, vt.rain_speed, vt.rain_trail);
    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(AMBER))
        .title(Span::styled(
            " AI SUBSCRIPTION ",
            Style::default()
                .fg(AMBER)
                .bg(BG_PANEL)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(BG_PANEL));
    frame.render_widget(block, modal);

    let inner = Rect {
        x: modal.x + 2,
        y: modal.y + 1,
        width: modal.width.saturating_sub(4),
        height: modal.height.saturating_sub(2),
    };

    let mut lines: Vec<Line> = Vec::new();
    let iw = inner.width as usize;
    let row_w = iw.saturating_sub(2); // explicit 1-char inset on both sides
    let display_w = |s: &str| s.chars().count();
    let dotted_row =
        |left: String, right: String, left_fg: Color, right_fg: Color| -> Line<'static> {
            let dots_w = row_w.saturating_sub(display_w(&left) + display_w(&right));
            Line::from(vec![
                Span::styled(" ", Style::default().bg(BG_PANEL)),
                Span::styled(left, Style::default().fg(left_fg).bg(BG_PANEL)),
                Span::styled("·".repeat(dots_w), Style::default().fg(BORDER).bg(BG_PANEL)),
                Span::styled(right, Style::default().fg(right_fg).bg(BG_PANEL)),
                Span::styled(" ", Style::default().bg(BG_PANEL)),
            ])
        };

    lines.push(Line::from(""));
    lines.push(dotted_row(
        "Current Subscription".to_string(),
        game.state.subscription.label().to_ascii_uppercase(),
        TEXT,
        AMBER,
    ));
    lines.push(dotted_row(
        "AI Index".to_string(),
        game.state.ai.percent_string(),
        TEXT,
        CYAN,
    ));
    lines.push(Line::from(""));

    let money_scale = game.state.money_scale() as i64;
    for (i, tier) in SubscriptionTier::ALL.iter().enumerate() {
        let unlocked = game.can_set_subscription(*tier);
        let is_current = *tier == game.state.subscription;
        let is_selected = i == selected;

        let marker = if is_selected { "\u{25b8} " } else { "  " };
        let tier_label = tier.label().to_ascii_uppercase();
        let status = if is_current {
            "ACTIVE".to_string()
        } else if unlocked {
            "OPEN".to_string()
        } else {
            format!("LOCK @ {:.0}%", (tier.unlock_ai_x10() as f32) / 10.0)
        };

        let cost_text = if tier.cost_per_turn() == 0 {
            "FREE".to_string()
        } else {
            format!("${}/t", tier.cost_per_turn() * money_scale)
        };
        let ai_bonus = tier.ai_fluency_bonus_per_turn();
        let bonus_text = if ai_bonus > 0 {
            format!("+{}AI/t", ai_bonus)
        } else {
            format!("{}AI/t", ai_bonus)
        };
        let row_left = format!(
            "{}{:<6} {:>8} {:>8}",
            marker, tier_label, cost_text, bonus_text
        );

        let label_color = if is_current {
            AMBER
        } else if !unlocked {
            TEXT_DIM
        } else if is_selected {
            TEXT_HI
        } else {
            TEXT
        };
        let status_color = if is_current {
            AMBER
        } else if unlocked {
            GREEN
        } else {
            RED
        };

        lines.push(dotted_row(row_left, status, label_color, status_color));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "Enter: select tier  |  Esc: back",
        Style::default().fg(TEXT_DIM).bg(BG_PANEL),
    )]));
    lines.push(Line::from(vec![Span::styled(
        "Ctrl+C: disconnect",
        Style::default().fg(TEXT_DIM).bg(BG_PANEL),
    )]));
    lines.push(Line::from(""));

    let text = Text::from(lines);
    let para = Paragraph::new(text);
    frame.render_widget(para, inner);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Audio settings
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_settings(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    selected: usize,
    events: &TurnEvents,
    input: &crate::app::InputState,
) {
    // Render the gameplay screen underneath.
    render_turn(frame, app, area, events, input);

    // Modal overlay with matrix rain.
    let modal = fixed_modal_rect(area);
    let vt = app.visual_tempo();
    dim_outside_modal(frame, app.tick, area, modal, vt.rain_speed, vt.rain_trail);
    frame.render_widget(Clear, modal);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(AMBER))
        .title(Span::styled(
            " SETTINGS ",
            Style::default()
                .fg(AMBER)
                .bg(BG_PANEL)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(BG_PANEL));
    frame.render_widget(block, modal);

    let inner = Rect {
        x: modal.x + 2,
        y: modal.y + 1,
        width: modal.width.saturating_sub(4),
        height: modal.height.saturating_sub(2),
    };

    let prefs = &app.audio_prefs;
    let narrator_label = app.narrator_label();
    let cheat_mode_label = app.cheat_mode_label();
    let tempo_label = app.tempo_label();
    let shepard_preset_label = app.shepard_preset_label();
    let items: [(&str, &str); 8] = [
        (
            "BACKGROUND SCORE",
            AudioPrefs::LEVEL_LABELS[prefs.score_level as usize],
        ),
        ("SCORE TRACK", prefs.score_track.label()),
        (
            "SHEPARD TONE",
            AudioPrefs::LEVEL_LABELS[prefs.shepard_level as usize],
        ),
        ("SHEPARD PRESET", &shepard_preset_label),
        ("SELECTION SFX", if prefs.sfx_on { "ON" } else { "OFF" }),
        ("LLM SHOWRUNNER", narrator_label),
        ("CHEAT MODE", cheat_mode_label),
        ("RUN LENGTH", &tempo_label),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        "Press Enter/Left/Right to cycle",
        Style::default().fg(TEXT_DIM).bg(BG_PANEL),
    )]));
    lines.push(Line::from(""));

    for (i, (label, value)) in items.iter().enumerate() {
        let marker = if i == selected { "\u{25b8} " } else { "  " };
        let fg = if i == selected { TEXT_HI } else { TEXT };

        let mut spans: Vec<Span> = Vec::new();
        spans.push(Span::styled(
            marker,
            Style::default().fg(AMBER).bg(BG_PANEL),
        ));
        spans.push(Span::styled(
            format!("{:<20}", label),
            Style::default().fg(fg).bg(BG_PANEL),
        ));

        // Visual bar for audio level items (row 0 = score volume, row 2 = shepard volume).
        if i == 0 || i == 2 {
            let level = if i == 0 {
                prefs.score_level
            } else {
                prefs.shepard_level
            };
            let bar: String = (0..AudioPrefs::NUM_LEVELS)
                .map(|j| if j < level { '\u{2588}' } else { '\u{2591}' })
                .collect();
            spans.push(Span::styled(
                format!("{} ", bar),
                Style::default().fg(AMBER).bg(BG_PANEL),
            ));
        }

        let val_color = if *value == "OFF" {
            TEXT_DIM
        } else if value.starts_with("AUTO") || *value == "CLAUDE" || *value == "CODEX" {
            CYAN
        } else {
            AMBER
        };
        spans.push(Span::styled(
            value.to_string(),
            Style::default()
                .fg(val_color)
                .bg(BG_PANEL)
                .add_modifier(Modifier::BOLD),
        ));

        lines.push(Line::from(spans));
    }

    // Description for selected settings row.
    if selected == 2 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  Classic audio illusion: a forever-rising tone.",
            Style::default().fg(TEXT_DIM).bg(BG_PANEL),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  It fakes acceleration so pressure feels constant.",
            Style::default().fg(TEXT_DIM).bg(BG_PANEL),
        )]));
    } else if selected == 5 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  Uses Claude/Codex for your freeform actions.",
            Style::default().fg(TEXT_DIM).bg(BG_PANEL),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  It resolves outcomes and adds flavor text.",
            Style::default().fg(TEXT_DIM).bg(BG_PANEL),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  AUTO prefers Claude if installed, else Codex.",
            Style::default().fg(TEXT_DIM).bg(BG_PANEL),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  Requires: claude auth / codex login",
            Style::default().fg(TEXT_DIM).bg(BG_PANEL),
        )]));
    } else if selected == 6 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  OFF: normal death rules. INVINCIBLE: zero stats won't kill you.",
            Style::default().fg(TEXT_DIM).bg(BG_PANEL),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  Final-turn endings still apply in both modes.",
            Style::default().fg(TEXT_DIM).bg(BG_PANEL),
        )]));
    } else if selected == 7 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  Simple = 12 turns. Standard = 32 turns.",
            Style::default().fg(TEXT_DIM).bg(BG_PANEL),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  This applies when you start the next run.",
            Style::default().fg(TEXT_DIM).bg(BG_PANEL),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        app.help_line(),
        Style::default().fg(TEXT_DIM).bg(BG_PANEL),
    )]));

    let text = Text::from(lines);
    let para = Paragraph::new(text);
    frame.render_widget(para, inner);
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  LLM overlay
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn render_overlay(frame: &mut Frame, area: Rect, msg: &str, tick: u64) {
    let r = fixed_modal_rect(area);
    dim_outside_modal(frame, tick, area, r, 1.0, 20);
    frame.render_widget(Clear, r);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(AMBER))
        .title(Span::styled(
            " SHOWRUNNER ",
            Style::default()
                .fg(AMBER)
                .bg(BG)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(BG));
    frame.render_widget(block, r);

    let inner = Rect {
        x: r.x + 2,
        y: r.y + 1,
        width: r.width.saturating_sub(4),
        height: r.height.saturating_sub(2),
    };

    if inner.height == 0 {
        return;
    }

    // Layout: center the main line + subtitle, hint at the bottom.
    let hint_y = inner.y + inner.height.saturating_sub(1);
    let body_h = inner.height.saturating_sub(1);
    // Center the pair (main + subtitle) in the body area.
    let pair_y = inner.y + body_h.saturating_sub(2) / 2;

    let spinner_frame = ui::spinner(tick);
    let main_line = Line::from(vec![
        Span::styled(
            format!(" {} ", spinner_frame),
            Style::default().fg(AMBER).bg(BG),
        ),
        Span::styled(msg, Style::default().fg(TEXT_HI).bg(BG)),
    ]);
    frame.render_widget(
        Paragraph::new(main_line)
            .alignment(Alignment::Center)
            .style(Style::default().bg(BG)),
        Rect {
            x: inner.x,
            y: pair_y,
            width: inner.width,
            height: 1,
        },
    );

    let subtitle = Line::from(vec![Span::styled(
        "this might take a moment",
        Style::default().fg(TEXT_DIM).bg(BG),
    )]);
    frame.render_widget(
        Paragraph::new(subtitle)
            .alignment(Alignment::Center)
            .style(Style::default().bg(BG)),
        Rect {
            x: inner.x,
            y: pair_y + 1,
            width: inner.width,
            height: 1,
        },
    );

    let hint_line = Line::from(vec![Span::styled(
        "▸ Esc: cancel",
        Style::default().fg(TEXT_DIM).bg(BG),
    )]);
    frame.render_widget(
        Paragraph::new(hint_line)
            .alignment(Alignment::Center)
            .style(Style::default().bg(BG)),
        Rect {
            x: inner.x,
            y: hint_y,
            width: inner.width,
            height: 1,
        },
    );
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  Helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn wrap_text_rows(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![];
    }

    let mut rows: Vec<String> = Vec::new();
    for raw in text.lines() {
        if raw.trim().is_empty() {
            rows.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in raw.split_whitespace() {
            let word_len = word.chars().count();
            if current.is_empty() {
                if word_len <= width {
                    current.push_str(word);
                } else {
                    let mut chunk = String::new();
                    for ch in word.chars() {
                        chunk.push(ch);
                        if chunk.chars().count() == width {
                            rows.push(std::mem::take(&mut chunk));
                        }
                    }
                    current = chunk;
                }
                continue;
            }

            let needed = current.chars().count() + 1 + word_len;
            if needed <= width {
                current.push(' ');
                current.push_str(word);
                continue;
            }

            rows.push(std::mem::take(&mut current));

            if word_len <= width {
                current.push_str(word);
            } else {
                let mut chunk = String::new();
                for ch in word.chars() {
                    chunk.push(ch);
                    if chunk.chars().count() == width {
                        rows.push(std::mem::take(&mut chunk));
                    }
                }
                current = chunk;
            }
        }

        if !current.is_empty() {
            rows.push(current);
        }
    }

    if rows.is_empty() {
        rows.push(String::new());
    }

    rows
}

/// Desaturate and dim all cells outside the modal, then overlay matrix rain.
fn dim_outside_modal(
    frame: &mut Frame,
    tick: u64,
    area: Rect,
    modal: Rect,
    rain_speed: f32,
    rain_trail: i64,
) {
    // First pass: dim everything outside the modal.
    let buf = frame.buffer_mut();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if x >= modal.x
                && x < modal.x + modal.width
                && y >= modal.y
                && y < modal.y + modal.height
            {
                continue;
            }
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.fg = dim_color(cell.fg);
                cell.bg = dim_color(cell.bg);
            }
        }
    }

    // Second pass: matrix rain (same logic as title screen).
    render_matrix_rain(frame, tick, area, modal, 4, rain_speed, rain_trail);
}

/// Desaturate and darken a color for the modal backdrop effect.
fn dim_color(color: Color) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            // Perceptual luminance
            let lum = (((r as u32) * 299 + (g as u32) * 587 + (b as u32) * 114) / 1000) as u8;
            let v = (lum / 3).max(0x06);
            // Slight navy tint to stay on-theme
            Color::Rgb(v, v, v.saturating_add(0x04))
        }
        _ => color,
    }
}

/// Shared fixed-size focus modal used by splash/title, LLM loading, and outcome.
fn fixed_modal_rect(area: Rect) -> Rect {
    let w = FOCUS_MODAL_W.min(area.width.saturating_sub(4)).max(1);
    let h = FOCUS_MODAL_H.min(area.height.saturating_sub(4)).max(1);
    ui::centered(area, w, h)
}

fn title_modal_rect(area: Rect) -> Rect {
    // Keep title card width consistent with other focus modals.
    let w = FOCUS_MODAL_W.min(area.width.saturating_sub(4)).max(1);
    // Title card is taller than other modals for the splash/menu layout.
    let h = (27)
        .min(area.height.saturating_sub(2))
        .max(16)
        .min(area.height);
    ui::centered(area, w, h)
}

/// Push a double-line separator into a lines vec.
fn push_double_sep(lines: &mut Vec<Line<'static>>, width: u16) {
    lines.push(Line::from(vec![Span::styled(
        ui::repeat_char('\u{2500}', width),
        Style::default().fg(BORDER_HI).bg(BG),
    )]));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::characters::CharacterId;
    use crate::engine::game::Game;

    #[test]
    fn ending_profile_prefers_real_run_when_game_exists() {
        let mut app = App::new(None, false, None, Tempo::Simple);
        app.game = Some(Game::new(
            Tempo::Simple,
            CharacterId::new("product_designer"),
            Some(1),
        ));

        let display = ending_display_profile(&app, EndingId::EnlightenedDropout, Some(0))
            .expect("ending profile should exist with game");

        assert!(!display.origin_line.trim().is_empty());
        assert!(!display.origin_line.contains("Preview profile:"));
    }

    #[test]
    fn ending_profile_preview_mode_uses_character_origin_line() {
        let app = App::new(None, false, None, Tempo::Simple);

        let character = CharacterDef::for_character(CharacterId::new("product_designer"));
        let display = ending_display_profile(&app, EndingId::EnlightenedDropout, Some(0))
            .expect("preview profile should exist");

        assert_eq!(display.origin_line, character.origin_line);
        assert!(!display.origin_line.contains("Preview profile:"));
    }
}
