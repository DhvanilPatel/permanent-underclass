use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result, anyhow};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use serde::Deserialize;

use crate::engine::events::PersonalEvent;
use crate::engine::types::ResourceDelta;

const SPRITE_LIBRARY_META_JSON: &str = include_str!("../data/sprites/sprite_library_meta.json");
const SPRITE_LIBRARY_JSONL: &str = include_str!("../data/sprites/sprite_library.jsonl");
const SPRITE_SEMANTICS_DIR_ENV: &str = "PU_SPRITE_SEMANTICS_DIR";
const DISABLE_SPRITE_SEMANTICS_ENV: &str = "PU_DISABLE_SPRITE_SEMANTICS";
const BUILD_SPRITE_SEMANTICS_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/data/sprites/semantics");
const EVENT_SPRITE_SELECTION_FILE: &str = "event_sprite_selection.jsonl";
const CHOICE_SPRITE_SELECTION_FILE: &str = "choice_sprite_selection.jsonl";

const FALLBACK_EVENT_SPRITE_ID: &str = "abstract_concepts_game_mechanics__random_event";
const FALLBACK_CHOICE_SPRITE_ID: &str = "emotions_status_effects__curiosity";

static LIB: OnceLock<SpriteLibrary> = OnceLock::new();
static EVENT_ICON_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
static CHOICE_ICON_CACHE: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
static SEMANTIC_SPRITE_MAPS: OnceLock<SemanticSpriteMaps> = OnceLock::new();

#[derive(Debug)]
pub struct SpriteLibrary {
    palette: HashMap<String, Color>,
    sprites: Vec<SpriteEntry>,
    by_id: HashMap<String, usize>,
}

#[derive(Debug)]
struct SpriteEntry {
    id: String,
    // Default rendering lines (historically 2x for source-format sprites).
    lines: Vec<Vec<Segment>>,
    // Optional 1x lines for compact contexts (e.g., choice cards).
    lines_1x: Option<Vec<Vec<Segment>>>,
}

#[derive(Debug, Clone)]
struct Segment {
    text: String,
    color_key: String,
}

impl<'de> Deserialize<'de> for Segment {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (text, color_key) = <(String, String)>::deserialize(deserializer)?;
        Ok(Self { text, color_key })
    }
}

#[derive(Debug, Deserialize)]
struct Meta {
    palette: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct EntryEnrichedRaw {
    id: String,
    sprite: SpriteEnrichedRaw,
}

#[derive(Debug, Deserialize)]
struct SpriteEnrichedRaw {
    #[serde(rename = "rows")]
    _rows: u32,
    lines: Vec<Vec<Segment>>,
}

#[derive(Debug, Deserialize)]
struct EntrySourceRaw {
    name: String,
    category: String,
    #[serde(rename = "1x")]
    sprite_1x: Vec<Vec<Segment>>,
    #[serde(rename = "2x")]
    sprite_2x: Vec<Vec<Segment>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum EntryAnyRaw {
    Enriched(EntryEnrichedRaw),
    Source(EntrySourceRaw),
}

#[derive(Debug, Default)]
struct SemanticSpriteMaps {
    event_by_id: HashMap<String, String>,
    choice_by_key: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct EventSpriteSelectionRow {
    event_id: String,
    sprite_id: String,
}

#[derive(Debug, Deserialize)]
struct ChoiceSpriteSelectionRow {
    event_id: String,
    choice_index: usize,
    sprite_id: String,
}

pub fn library() -> &'static SpriteLibrary {
    LIB.get_or_init(|| match SpriteLibrary::from_embedded() {
        Ok(lib) => lib,
        Err(err) => {
            eprintln!("WARN: failed to load sprite library: {err}");
            SpriteLibrary::fallback()
        }
    })
}

impl SpriteLibrary {
    fn from_embedded() -> Result<Self> {
        let meta: Meta =
            serde_json::from_str(SPRITE_LIBRARY_META_JSON).context("parse sprite meta json")?;

        let mut palette: HashMap<String, Color> = HashMap::new();
        for (k, v) in meta.palette {
            let c = parse_hex_color(&v)
                .ok_or_else(|| anyhow!("invalid hex color '{}' for palette key '{}'", v, k))?;
            palette.insert(k, c);
        }

        let mut sprites: Vec<SpriteEntry> = Vec::new();
        let mut by_id: HashMap<String, usize> = HashMap::new();

        for (line_idx, raw) in SPRITE_LIBRARY_JSONL.lines().enumerate() {
            let line_no = line_idx + 1;
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let e: EntryAnyRaw = serde_json::from_str(line).with_context(|| {
                format!(
                    "parse sprite library jsonl: data/sprites/sprite_library.jsonl:{}",
                    line_no
                )
            })?;

            let entry = match e {
                EntryAnyRaw::Enriched(e) => SpriteEntry {
                    id: e.id,
                    lines: e.sprite.lines,
                    lines_1x: None,
                },
                EntryAnyRaw::Source(s) => {
                    // Source-of-truth format used by tooling/sprites/build_library.py.
                    let id = format!("{}__{}", slugify(&s.category), slugify(&s.name));

                    SpriteEntry {
                        id,
                        lines: s.sprite_2x,
                        lines_1x: Some(s.sprite_1x),
                    }
                }
            };
            by_id.insert(entry.id.clone(), sprites.len());
            sprites.push(entry);
        }

        if sprites.is_empty() {
            return Err(anyhow!("sprite library jsonl parsed 0 sprites"));
        }

        Ok(Self {
            palette,
            sprites,
            by_id,
        })
    }

    fn render_sprite(&self, id: &str, bg: Color) -> Option<Vec<Line<'static>>> {
        let idx = *self.by_id.get(id)?;
        let e = &self.sprites[idx];
        Some(render_lines(&self.palette, &e.lines, bg))
    }

    fn render_sprite_1x(&self, id: &str, bg: Color) -> Option<Vec<Line<'static>>> {
        let idx = *self.by_id.get(id)?;
        let e = &self.sprites[idx];
        let lines = e.lines_1x.as_ref().unwrap_or(&e.lines);
        Some(render_lines(&self.palette, lines, bg))
    }

    fn fallback() -> Self {
        let mut palette = HashMap::new();
        palette.insert("w".to_string(), Color::Rgb(0xdd, 0xdd, 0xdd));
        palette.insert("d".to_string(), Color::Rgb(0x66, 0x66, 0x66));

        let fallback_lines = vec![
            vec![Segment {
                text: "┌────────┐".to_string(),
                color_key: "d".to_string(),
            }],
            vec![Segment {
                text: "│ SPRITE │".to_string(),
                color_key: "w".to_string(),
            }],
            vec![Segment {
                text: "└────────┘".to_string(),
                color_key: "d".to_string(),
            }],
        ];

        let mut sprites = Vec::new();
        let mut by_id = HashMap::new();

        for id in [FALLBACK_EVENT_SPRITE_ID, FALLBACK_CHOICE_SPRITE_ID] {
            let entry = SpriteEntry {
                id: id.to_string(),
                lines: fallback_lines.clone(),
                lines_1x: Some(fallback_lines.clone()),
            };
            by_id.insert(entry.id.clone(), sprites.len());
            sprites.push(entry);
        }

        Self {
            palette,
            sprites,
            by_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChoiceDeltaAxis {
    Money,
    Sanity,
    Audience,
    AiSkill,
    Friends,
    Mixed,
}

fn dominant_choice_axis(delta: ResourceDelta) -> ChoiceDeltaAxis {
    let mut best_axis = ChoiceDeltaAxis::Mixed;
    let mut best_magnitude = 0_i64;
    let dimensions = [
        (ChoiceDeltaAxis::Money, delta.money.saturating_abs()),
        (
            ChoiceDeltaAxis::Sanity,
            i64::from(delta.sanity.saturating_abs()),
        ),
        (
            ChoiceDeltaAxis::Audience,
            i64::from(delta.audience.saturating_abs()),
        ),
        (
            ChoiceDeltaAxis::AiSkill,
            i64::from(delta.ai_skill.saturating_abs()),
        ),
        (
            ChoiceDeltaAxis::Friends,
            i64::from(delta.friends.saturating_abs()),
        ),
    ];

    for (axis, magnitude) in dimensions {
        if magnitude > best_magnitude {
            best_axis = axis;
            best_magnitude = magnitude;
        }
    }

    if best_magnitude == 0 {
        ChoiceDeltaAxis::Mixed
    } else {
        best_axis
    }
}

fn choice_axis_label(axis: ChoiceDeltaAxis) -> &'static str {
    match axis {
        ChoiceDeltaAxis::Money => "MONEY",
        ChoiceDeltaAxis::Sanity => "SANITY",
        ChoiceDeltaAxis::Audience => "AUDIENCE",
        ChoiceDeltaAxis::AiSkill => "AI",
        ChoiceDeltaAxis::Friends => "FRIENDS",
        ChoiceDeltaAxis::Mixed => "MIXED",
    }
}

fn default_choice_sprite_id(delta: ResourceDelta) -> &'static str {
    let _ = delta;
    FALLBACK_CHOICE_SPRITE_ID
}

fn choice_delta_sprite(delta: ResourceDelta, bg: Color) -> Vec<Line<'static>> {
    let lib = library();
    let id = default_choice_sprite_id(delta);
    lib.render_sprite_1x(id, bg)
        .unwrap_or_else(|| render_fallback_choice_delta(delta, bg))
}

pub fn personal_event_sprite(ev: &PersonalEvent, bg: Color) -> Vec<Line<'static>> {
    let id = personal_event_sprite_id(ev);
    library().render_sprite(&id, bg).unwrap_or_else(|| {
        library()
            .render_sprite(FALLBACK_EVENT_SPRITE_ID, bg)
            .unwrap_or_default()
    })
}

fn personal_event_sprite_id(ev: &PersonalEvent) -> String {
    let cache = EVENT_ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock()
        && let Some(id) = guard.get(&ev.id)
    {
        return id.clone();
    }

    if let Some(mapped) = semantic_event_sprite_id(&ev.id) {
        if let Ok(mut guard) = cache.lock() {
            guard.insert(ev.id.clone(), mapped.clone());
        }
        return mapped;
    }

    // Semantic map is authoritative. Missing/invalid rows fall back to a neutral misc sprite.
    let picked = FALLBACK_EVENT_SPRITE_ID.to_string();

    if let Ok(mut guard) = cache.lock() {
        guard.insert(ev.id.clone(), picked.clone());
    }
    picked
}

pub fn choice_contextual_sprite(
    ev: &PersonalEvent,
    choice_idx: usize,
    bg: Color,
) -> Vec<Line<'static>> {
    let ids = choice_sprite_ids_for_event(ev);
    if let Some(id) = ids.get(choice_idx)
        && let Some(rendered) = library().render_sprite_1x(id, bg)
    {
        return rendered;
    }
    let delta = ev
        .choices
        .get(choice_idx)
        .map(|c| c.delta)
        .unwrap_or(ResourceDelta::ZERO);
    choice_delta_sprite(delta, bg)
}

fn choice_sprite_ids_for_event(ev: &PersonalEvent) -> Vec<String> {
    let key = choice_sprite_cache_key(ev);
    let cache = CHOICE_ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock()
        && let Some(ids) = guard.get(&key)
    {
        return ids.clone();
    }

    let picked = pick_choice_sprite_ids_for_event(ev);
    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, picked.clone());
    }
    picked
}

fn choice_sprite_cache_key(ev: &PersonalEvent) -> String {
    let mut key = String::new();
    key.push_str(&ev.id);
    key.push('|');
    key.push_str(&ev.title);
    key.push('|');
    for c in ev.choices.iter().take(3) {
        key.push(c.key);
        key.push(':');
        key.push_str(&c.title);
        key.push('|');
        key.push_str(&c.body);
        key.push(';');
    }
    key
}

fn pick_choice_sprite_ids_for_event(ev: &PersonalEvent) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();

    for (choice_idx, choice) in ev.choices.iter().take(3).enumerate() {
        if let Some(mapped) = semantic_choice_sprite_id(&ev.id, choice_idx) {
            out.push(mapped);
            continue;
        }

        out.push(fallback_choice_sprite_id(choice.delta));
    }

    out
}

fn fallback_choice_sprite_id(choice_delta: ResourceDelta) -> String {
    let _ = choice_delta;
    FALLBACK_CHOICE_SPRITE_ID.to_string()
}

fn render_fallback_choice_delta(delta: ResourceDelta, bg: Color) -> Vec<Line<'static>> {
    let fg = Color::Rgb(0xcc, 0xcc, 0xcc);
    let label = choice_axis_label(dominant_choice_axis(delta));
    let s = |t: String| Span::styled(t, Style::default().fg(fg).bg(bg));
    vec![
        Line::from(s("╔══════════════════╗".to_string())),
        Line::from(s("║▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒║".to_string())),
        Line::from(s("║▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒║".to_string())),
        Line::from(s(format!("║{:^18}║", label))),
        Line::from(s(format!("║{:^18}║", label))),
        Line::from(s("║▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒║".to_string())),
        Line::from(s("║▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒║".to_string())),
        Line::from(s("║▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒║".to_string())),
        Line::from(s("║▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒║".to_string())),
        Line::from(s("╚══════════════════╝".to_string())),
    ]
}

fn render_lines(
    palette: &HashMap<String, Color>,
    lines: &[Vec<Segment>],
    bg: Color,
) -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = Vec::new();
    for line in lines {
        let mut spans: Vec<Span<'static>> = Vec::new();
        for seg in line {
            let fg = palette
                .get(&seg.color_key)
                .copied()
                .unwrap_or(Color::Rgb(0xcc, 0xcc, 0xcc));
            spans.push(Span::styled(
                seg.text.clone(),
                Style::default().fg(fg).bg(bg),
            ));
        }
        out.push(Line::from(spans));
    }
    out
}

fn parse_hex_color(s: &str) -> Option<Color> {
    let hex = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn slugify(s: &str) -> String {
    // Match tooling/sprites/build_library.py slugify (lowercase, non [a-z0-9] -> _, collapse).
    let mut out = String::new();
    let mut last_was_us = false;

    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_us = false;
        } else if !out.is_empty() && !last_was_us {
            out.push('_');
            last_was_us = true;
        }
    }

    while out.ends_with('_') {
        out.pop();
    }

    if out.is_empty() { "x".to_string() } else { out }
}

fn semantic_sprite_maps_enabled() -> bool {
    if cfg!(test) {
        return true;
    }

    match std::env::var(DISABLE_SPRITE_SEMANTICS_ENV) {
        Ok(raw) => {
            let v = raw.trim().to_ascii_lowercase();
            !(v == "1" || v == "true" || v == "yes" || v == "on")
        }
        Err(_) => true,
    }
}

fn semantic_sprite_maps() -> &'static SemanticSpriteMaps {
    SEMANTIC_SPRITE_MAPS.get_or_init(load_semantic_sprite_maps)
}

fn load_semantic_sprite_maps() -> SemanticSpriteMaps {
    if !semantic_sprite_maps_enabled() {
        return SemanticSpriteMaps::default();
    }

    let event_by_id = match load_event_sprite_selections() {
        Ok(m) => m,
        Err(err) => {
            eprintln!("WARN: failed loading event sprite selections: {err}");
            HashMap::new()
        }
    };
    let choice_by_key = match load_choice_sprite_selections() {
        Ok(m) => m,
        Err(err) => {
            eprintln!("WARN: failed loading choice sprite selections: {err}");
            HashMap::new()
        }
    };

    if event_by_id.is_empty() && choice_by_key.is_empty() {
        eprintln!("WARN: sprite semantic mappings unavailable; using fallback sprites");
    }

    SemanticSpriteMaps {
        event_by_id,
        choice_by_key,
    }
}

fn semantic_event_sprite_id(event_id: &str) -> Option<String> {
    let maps = semantic_sprite_maps();
    let id = maps.event_by_id.get(event_id)?;
    if library().by_id.contains_key(id) {
        Some(id.clone())
    } else {
        None
    }
}

fn semantic_choice_sprite_id(event_id: &str, choice_idx: usize) -> Option<String> {
    let maps = semantic_sprite_maps();
    let key = semantic_choice_key(event_id, choice_idx);
    let id = maps.choice_by_key.get(&key)?;
    if library().by_id.contains_key(id) {
        Some(id.clone())
    } else {
        None
    }
}

fn semantic_choice_key(event_id: &str, choice_idx: usize) -> String {
    format!("{}#{}", event_id, choice_idx)
}

fn load_event_sprite_selections() -> Result<HashMap<String, String>> {
    let Some((path, jsonl)) = read_sprite_semantics_file(EVENT_SPRITE_SELECTION_FILE)? else {
        return Ok(HashMap::new());
    };

    let mut out: HashMap<String, String> = HashMap::new();
    for (line_idx, raw) in jsonl.lines().enumerate() {
        let line_no = line_idx + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let row: EventSpriteSelectionRow = serde_json::from_str(line).with_context(|| {
            format!(
                "parse {}:{} as event sprite selection row",
                path.display(),
                line_no
            )
        })?;
        out.insert(row.event_id, row.sprite_id);
    }
    Ok(out)
}

fn load_choice_sprite_selections() -> Result<HashMap<String, String>> {
    let Some((path, jsonl)) = read_sprite_semantics_file(CHOICE_SPRITE_SELECTION_FILE)? else {
        return Ok(HashMap::new());
    };

    let mut out: HashMap<String, String> = HashMap::new();
    for (line_idx, raw) in jsonl.lines().enumerate() {
        let line_no = line_idx + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let row: ChoiceSpriteSelectionRow = serde_json::from_str(line).with_context(|| {
            format!(
                "parse {}:{} as choice sprite selection row",
                path.display(),
                line_no
            )
        })?;
        let key = semantic_choice_key(&row.event_id, row.choice_index);
        out.insert(key, row.sprite_id);
    }
    Ok(out)
}

fn read_sprite_semantics_file(filename: &str) -> Result<Option<(PathBuf, String)>> {
    for candidate in sprite_semantics_file_candidates(filename) {
        match fs::read_to_string(&candidate) {
            Ok(contents) => return Ok(Some((candidate, contents))),
            Err(err) if err.kind() == ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(err).with_context(|| {
                    format!("read sprite semantics file {}", candidate.display())
                });
            }
        }
    }
    Ok(None)
}

fn sprite_semantics_file_candidates(filename: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();

    if let Ok(raw) = std::env::var(SPRITE_SEMANTICS_DIR_ENV) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            out.push(PathBuf::from(trimmed).join(filename));
        }
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(root) = exe.parent().and_then(|p| p.parent())
    {
        out.push(root.join("data/sprites/semantics").join(filename));
    }

    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join("data/sprites/semantics").join(filename));
    }

    out.push(PathBuf::from(BUILD_SPRITE_SEMANTICS_DIR).join(filename));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::content::Content;
    use crate::engine::events::Choice;
    use crate::engine::types::ResourceDelta;
    use std::collections::HashSet;

    #[test]
    fn sprite_library_loads() {
        let lib = library();
        assert!(lib.sprites.len() >= 1000);
        assert!(lib.by_id.contains_key(FALLBACK_EVENT_SPRITE_ID));
    }

    #[test]
    fn default_choice_sprite_exists() {
        let lib = library();
        assert!(
            lib.by_id.contains_key(FALLBACK_CHOICE_SPRITE_ID),
            "missing fallback choice sprite id '{}'",
            FALLBACK_CHOICE_SPRITE_ID
        );
    }

    #[test]
    fn choice_sprites_fallback_to_curiosity_when_unmapped_and_deterministic() {
        let ev = PersonalEvent {
            id: "test_choice_context".to_string(),
            title: "Context test".to_string(),
            body: "Resolve icons from choice text".to_string(),
            choices: vec![
                Choice {
                    key: '1',
                    title: "Do a code review sprint".to_string(),
                    body: "Push PR feedback and fix blockers.".to_string(),
                    delta: ResourceDelta {
                        money: 2_500,
                        sanity: -6,
                        ai_skill: 6,
                        ..ResourceDelta::ZERO
                    },
                    min_money: None,
                    effects: Vec::new(),
                },
                Choice {
                    key: '2',
                    title: "Go to networking event".to_string(),
                    body: "Meet peers and exchange contacts.".to_string(),
                    delta: ResourceDelta {
                        friends: 6,
                        audience: 6,
                        ..ResourceDelta::ZERO
                    },
                    min_money: None,
                    effects: Vec::new(),
                },
                Choice {
                    key: '3',
                    title: "Enable rest mode this weekend".to_string(),
                    body: "Log off and recover.".to_string(),
                    delta: ResourceDelta {
                        sanity: 6,
                        friends: 2,
                        ..ResourceDelta::ZERO
                    },
                    min_money: None,
                    effects: Vec::new(),
                },
            ],
            allow_freeform: true,
        };

        let ids_a = choice_sprite_ids_for_event(&ev);
        let ids_b = choice_sprite_ids_for_event(&ev);

        assert_eq!(ids_a, ids_b);
        assert_eq!(ids_a.len(), 3);
        assert_eq!(ids_a[0], FALLBACK_CHOICE_SPRITE_ID);
        assert_eq!(ids_a[1], FALLBACK_CHOICE_SPRITE_ID);
        assert_eq!(ids_a[2], FALLBACK_CHOICE_SPRITE_ID);
    }

    #[test]
    fn recovery_choice_avoids_pressure_icons_for_linkedin_walk_case() {
        let ev = PersonalEvent {
            id: "ex_faang_engineer_03_linkedin_open_to_work".to_string(),
            title: "LinkedIn: Open To Work".to_string(),
            body: "You open LinkedIn for the first time since your layoff. Your Open to Work banner has been on for six weeks. A recruiter messages you about a role that is clearly your old job, reposted at 60% of the salary. The hiring manager is someone you onboarded in 2021. You stare at this for a long time. The message preview says Hi [First Name].".to_string(),
            choices: vec![
                Choice {
                    key: '1',
                    title: "Apply anyway".to_string(),
                    body: "Apply anyway — you need the structure".to_string(),
                    delta: ResourceDelta {
                        money: 2_500,
                        sanity: -6,
                        ..ResourceDelta::ZERO
                    },
                    min_money: None,
                    effects: Vec::new(),
                },
                Choice {
                    key: '2',
                    title: "Close the laptop, go for a walk".to_string(),
                    body: "Close the laptop, go for a walk".to_string(),
                    delta: ResourceDelta {
                        sanity: 6,
                        friends: 2,
                        ..ResourceDelta::ZERO
                    },
                    min_money: None,
                    effects: Vec::new(),
                },
                Choice {
                    key: '3',
                    title: "Forward it to your old team Slack".to_string(),
                    body: "Forward it to your old team Slack".to_string(),
                    delta: ResourceDelta {
                        friends: 6,
                        audience: 6,
                        ..ResourceDelta::ZERO
                    },
                    min_money: None,
                    effects: Vec::new(),
                },
            ],
            allow_freeform: true,
        };

        let ids = choice_sprite_ids_for_event(&ev);
        assert_eq!(ids.len(), 3);
        assert_ne!(
            ids[1], "emotions_status_effects__hustle_mode",
            "recovery choice should never pick hustle-mode icon"
        );
        assert_ne!(
            ids[1], "work_career_corporate__the_grind",
            "recovery choice should never pick pressure icon"
        );
    }

    #[test]
    fn exact_event_31_choice_sprites_are_semantic_and_not_collapsed() {
        let content = Content::embedded().expect("embedded content should parse");
        let ev = content
            .per_character
            .values()
            .flatten()
            .find(|e| e.id == "ex_faang_engineer_09_recruiter_asks_level")
            .expect("recruiter event should exist");

        let personal = PersonalEvent {
            id: ev.id.clone(),
            title: ev.title.clone(),
            body: ev.body.clone(),
            choices: ev
                .choices
                .iter()
                .enumerate()
                .map(|(idx, c)| Choice {
                    key: char::from_digit((idx + 1) as u32, 10).unwrap_or('?'),
                    title: c.title.clone(),
                    body: c.body.clone(),
                    delta: c.delta,
                    min_money: c.min_money,
                    effects: Vec::new(),
                })
                .collect(),
            allow_freeform: ev.allow_freeform,
        };

        let ids = choice_sprite_ids_for_event(&personal);
        assert_eq!(ids.len(), 3);
        let unique: HashSet<String> = ids.iter().cloned().collect();
        assert_eq!(
            unique.len(),
            3,
            "all three choice icons should be different"
        );

        // Specific guard against the previously observed bad behavior.
        assert_ne!(
            ids[2], "emotions_status_effects__hustle_mode",
            "recovery choice must not resolve to hustle_mode"
        );
        assert_ne!(
            ids[2], "work_career_corporate__the_grind",
            "recovery choice must not resolve to pressure icon"
        );
        assert_ne!(
            ids[0], FALLBACK_CHOICE_SPRITE_ID,
            "mapped money choice should not resolve to fallback curiosity"
        );
    }

    #[test]
    fn semantic_mappings_cover_embedded_events_and_choices() {
        let content = Content::embedded().expect("embedded content should parse");

        let mut missing_event_ids: Vec<String> = Vec::new();
        let mut missing_choice_keys: Vec<String> = Vec::new();

        for ev in content
            .universal
            .iter()
            .chain(content.per_character.values().flatten())
        {
            if semantic_event_sprite_id(&ev.id).is_none() {
                missing_event_ids.push(ev.id.clone());
            }
            for choice_idx in 0..ev.choices.len() {
                if semantic_choice_sprite_id(&ev.id, choice_idx).is_none() {
                    missing_choice_keys.push(format!("{}#{}", ev.id, choice_idx));
                }
            }
        }

        assert!(
            missing_event_ids.is_empty(),
            "missing semantic event mappings (sample): {:?}",
            &missing_event_ids[..missing_event_ids.len().min(10)]
        );
        assert!(
            missing_choice_keys.is_empty(),
            "missing semantic choice mappings (sample): {:?}",
            &missing_choice_keys[..missing_choice_keys.len().min(10)]
        );
    }
}
