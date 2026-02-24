use std::collections::HashMap;
use std::sync::OnceLock;

use crate::engine::game::GameState;
use crate::engine::subscription::SubscriptionTier;
use crate::engine::types::DeathReason;
use serde::Deserialize;

const ENDINGS_JSONL: &str = include_str!("../../data/endings.jsonl");
const CHARACTERS_JSONL: &str = include_str!("../../data/characters.jsonl");
static ENDINGS_CATALOG: OnceLock<EndingsCatalog> = OnceLock::new();
static ORIGIN_LINES: OnceLock<HashMap<String, String>> = OnceLock::new();

#[derive(Debug)]
struct EndingsCatalog {
    endings: HashMap<EndingId, Ending>,
    scoring: HashMap<EndingId, EndingScoringSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndingId {
    // Death screens
    Broke,
    BurnedOut,

    // Secret-ish
    Luddite,

    // Tiered endings
    Oligarch,
    LonelyOligarch,
    Irreplaceable,
    Builder,
    Connector,
    Comfortable,
    EnlightenedDropout,
    Artist,
    FamousForNoReason,
    PermanentUnderclass,
    Survivor,

    // V1.1 endings
    Sellout,
    Whistleblower,
    Phoenix,
    Collaborator,
    Ghost,
    DebtSpiral,
    Canceled,
    Expat,
    SlopMerchant,
    Unionized,
    Scraped,
}

impl EndingId {
    pub const ALL: [EndingId; 25] = [
        EndingId::Broke,
        EndingId::BurnedOut,
        EndingId::Luddite,
        EndingId::Oligarch,
        EndingId::LonelyOligarch,
        EndingId::Irreplaceable,
        EndingId::Builder,
        EndingId::Connector,
        EndingId::Comfortable,
        EndingId::EnlightenedDropout,
        EndingId::Artist,
        EndingId::FamousForNoReason,
        EndingId::PermanentUnderclass,
        EndingId::Survivor,
        EndingId::Sellout,
        EndingId::Whistleblower,
        EndingId::Phoenix,
        EndingId::Collaborator,
        EndingId::Ghost,
        EndingId::DebtSpiral,
        EndingId::Canceled,
        EndingId::Expat,
        EndingId::SlopMerchant,
        EndingId::Unionized,
        EndingId::Scraped,
    ];

    fn key(self) -> &'static str {
        match self {
            EndingId::Broke => "broke",
            EndingId::BurnedOut => "burned_out",
            EndingId::Luddite => "luddite",
            EndingId::Oligarch => "oligarch",
            EndingId::LonelyOligarch => "lonely_oligarch",
            EndingId::Irreplaceable => "irreplaceable",
            EndingId::Builder => "builder",
            EndingId::Connector => "connector",
            EndingId::Comfortable => "comfortable",
            EndingId::EnlightenedDropout => "enlightened_dropout",
            EndingId::Artist => "artist",
            EndingId::FamousForNoReason => "famous_for_no_reason",
            EndingId::PermanentUnderclass => "permanent_underclass",
            EndingId::Survivor => "survivor",
            EndingId::Sellout => "sellout",
            EndingId::Whistleblower => "whistleblower",
            EndingId::Phoenix => "phoenix",
            EndingId::Collaborator => "collaborator",
            EndingId::Ghost => "ghost",
            EndingId::DebtSpiral => "debt_spiral",
            EndingId::Canceled => "canceled",
            EndingId::Expat => "expat",
            EndingId::SlopMerchant => "slop_merchant",
            EndingId::Unionized => "unionized",
            EndingId::Scraped => "scraped",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        let normalized: String = key
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();

        match normalized.as_str() {
            "broke" => Some(EndingId::Broke),
            "burnedout" => Some(EndingId::BurnedOut),
            "luddite" => Some(EndingId::Luddite),
            "oligarch" => Some(EndingId::Oligarch),
            "lonelyoligarch" => Some(EndingId::LonelyOligarch),
            "irreplaceable" => Some(EndingId::Irreplaceable),
            "builder" => Some(EndingId::Builder),
            "connector" => Some(EndingId::Connector),
            "comfortable" => Some(EndingId::Comfortable),
            "enlighteneddropout" => Some(EndingId::EnlightenedDropout),
            "artist" => Some(EndingId::Artist),
            "famousfornoreason" => Some(EndingId::FamousForNoReason),
            "permanentunderclass" => Some(EndingId::PermanentUnderclass),
            "survivor" => Some(EndingId::Survivor),
            "sellout" => Some(EndingId::Sellout),
            "whistleblower" => Some(EndingId::Whistleblower),
            "phoenix" => Some(EndingId::Phoenix),
            "collaborator" => Some(EndingId::Collaborator),
            "ghost" => Some(EndingId::Ghost),
            "debtspiral" => Some(EndingId::DebtSpiral),
            "canceled" => Some(EndingId::Canceled),
            "expat" => Some(EndingId::Expat),
            "slopmerchant" => Some(EndingId::SlopMerchant),
            "unionized" => Some(EndingId::Unionized),
            "scraped" => Some(EndingId::Scraped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EndingRow {
    id: String,
    title: String,
    body: String,
    #[serde(rename = "type", default)]
    ending_type: Option<String>,
    #[serde(default)]
    scoring: Option<EndingScoringRow>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EndingScoringRow {
    conditions: Vec<EndingConditionRow>,
    min_threshold: i32,
    specificity: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EndingConditionRow {
    check: String,
    points: i32,
}

#[derive(Debug, Clone)]
struct EndingScoringSpec {
    conditions: Vec<EndingConditionSpec>,
    min_threshold: i32,
    specificity: i32,
}

#[derive(Debug, Clone)]
struct EndingConditionSpec {
    check: EndingCheck,
    points: i32,
}

#[derive(Debug, Clone)]
enum EndingCheck {
    HasFlag(String),
    NotFlag(String),
    CharacterIs(String),
    SubscriptionIs(SubscriptionTier),
    Compare {
        field: NumericField,
        op: CompareOp,
        rhs: i64,
    },
    Between {
        field: NumericField,
        min: i64,
        max: i64,
    },
    All(Vec<EndingCheck>),
    Any(Vec<EndingCheck>),
}

#[derive(Debug, Clone, Copy)]
enum NumericField {
    Money,
    Sanity,
    Audience,
    AiSkill,
    Friends,
    TurnsRecorded,
    CashFocusTurns,
    AudiencePushTurns,
    AiPushTurns,
    RecoveryTurns,
    RelationshipTurns,
    HighPressureTurns,
    CashFocusPct,
    HighPressurePct,
}

#[derive(Debug, Clone, Copy)]
enum CompareOp {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
    Ne,
}

/// Get an ending by its ID from embedded JSON.
pub fn ending_by_id(id: EndingId) -> Ending {
    endings_catalog()
        .endings
        .get(&id)
        .cloned()
        .unwrap_or_else(|| fallback_ending_for_id(id))
}

fn fallback_ending_for_id(id: EndingId) -> Ending {
    let title = id
        .key()
        .replace('_', " ")
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}",
                    first.to_ascii_uppercase(),
                    chars.as_str().to_ascii_lowercase()
                ),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    Ending {
        id,
        title,
        body: "Ending data is unavailable for this build. Please verify data/endings.jsonl."
            .to_string(),
    }
}

fn fallback_endings_catalog() -> EndingsCatalog {
    let endings = EndingId::ALL
        .into_iter()
        .map(|id| (id, fallback_ending_for_id(id)))
        .collect();
    EndingsCatalog {
        endings,
        scoring: HashMap::new(),
    }
}

fn load_or_fallback_catalog() -> EndingsCatalog {
    match load_endings_catalog() {
        Ok(catalog) => catalog,
        Err(err) => {
            eprintln!("WARN: failed to load endings catalog: {err}");
            fallback_endings_catalog()
        }
    }
}

fn parse_origin_lines() -> HashMap<String, String> {
    #[derive(Deserialize)]
    struct ClassEntry {
        id: String,
        origin_line: String,
    }

    let mut out = HashMap::new();
    for (idx, raw_line) in CHARACTERS_JSONL.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match serde_json::from_str::<ClassEntry>(line) {
            Ok(entry) => {
                out.insert(entry.id, entry.origin_line);
            }
            Err(err) => {
                eprintln!(
                    "WARN: data/characters.jsonl:{} parse error while loading origin lines: {}",
                    line_no, err
                );
            }
        }
    }
    out
}

fn load_origin_lines() -> &'static HashMap<String, String> {
    ORIGIN_LINES.get_or_init(parse_origin_lines)
}

fn get_scoring_spec(id: EndingId, catalog: &EndingsCatalog) -> Option<&EndingScoringSpec> {
    catalog.scoring.get(&id)
}

fn endings_catalog() -> &'static EndingsCatalog {
    ENDINGS_CATALOG.get_or_init(load_or_fallback_catalog)
}

fn load_endings_catalog() -> Result<EndingsCatalog, String> {
    let mut rows: Vec<EndingRow> = Vec::new();
    for (idx, raw_line) in ENDINGS_JSONL.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let row: EndingRow = serde_json::from_str(line)
            .map_err(|e| format!("data/endings.jsonl:{} parse error: {}", line_no, e))?;
        rows.push(row);
    }

    let mut endings = HashMap::new();
    let mut scoring = HashMap::new();
    for row in rows {
        let Some(id) = EndingId::from_key(row.id.trim()) else {
            return Err(format!("data/endings.jsonl unknown ending id '{}'", row.id));
        };
        if endings.contains_key(&id) {
            return Err(format!(
                "data/endings.jsonl duplicate ending id '{}'",
                row.id
            ));
        }
        let _ = &row.ending_type;
        endings.insert(
            id,
            Ending {
                id,
                title: row.title.trim().to_string(),
                body: row.body.trim().to_string(),
            },
        );

        if let Some(scoring_row) = row.scoring {
            let mut conditions = Vec::new();
            for cond in scoring_row.conditions {
                let parsed = parse_ending_check(cond.check.trim())
                    .map_err(|e| format!("data/endings.jsonl '{}': {}", row.id, e))?;
                conditions.push(EndingConditionSpec {
                    check: parsed,
                    points: cond.points,
                });
            }
            scoring.insert(
                id,
                EndingScoringSpec {
                    conditions,
                    min_threshold: scoring_row.min_threshold,
                    specificity: scoring_row.specificity,
                },
            );
        }
    }

    for id in EndingId::ALL {
        if !endings.contains_key(&id) {
            return Err(format!(
                "data/endings.jsonl missing required ending '{}'",
                id.key()
            ));
        }

        if !matches!(id, EndingId::Broke | EndingId::BurnedOut) && !scoring.contains_key(&id) {
            return Err(format!(
                "data/endings.jsonl missing scoring for ending '{}'",
                id.key()
            ));
        }
    }
    Ok(EndingsCatalog { endings, scoring })
}

fn parse_ending_check(raw: &str) -> Result<EndingCheck, String> {
    if let Some(inner) = raw.strip_prefix("all(").and_then(|s| s.strip_suffix(')')) {
        let checks = parse_check_list(inner)?;
        return Ok(EndingCheck::All(checks));
    }
    if let Some(inner) = raw.strip_prefix("any(").and_then(|s| s.strip_suffix(')')) {
        let checks = parse_check_list(inner)?;
        return Ok(EndingCheck::Any(checks));
    }

    if let Some(flag) = raw.strip_prefix("has_flag:") {
        let flag = flag.trim();
        if flag.is_empty() {
            return Err("has_flag check cannot be empty".to_string());
        }
        return Ok(EndingCheck::HasFlag(flag.to_string()));
    }
    if let Some(flag) = raw.strip_prefix("not_flag:") {
        let flag = flag.trim();
        if flag.is_empty() {
            return Err("not_flag check cannot be empty".to_string());
        }
        return Ok(EndingCheck::NotFlag(flag.to_string()));
    }
    if let Some(character) = raw.strip_prefix("character_is:") {
        let character = character.trim();
        if character.is_empty() {
            return Err("character_is check cannot be empty".to_string());
        }
        return Ok(EndingCheck::CharacterIs(character.to_string()));
    }
    if let Some(tier) = raw.strip_prefix("subscription_is:") {
        let tier = tier.trim();
        let tier = match tier {
            "free" => SubscriptionTier::Free,
            "pro" => SubscriptionTier::Pro,
            "ultra" => SubscriptionTier::Ultra,
            "giga" => SubscriptionTier::Giga,
            _ => {
                return Err(format!(
                    "unknown subscription tier '{}' (expected free|pro|ultra|giga)",
                    tier
                ));
            }
        };
        return Ok(EndingCheck::SubscriptionIs(tier));
    }

    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() == 4 && parts[1] == "between" {
        let field = NumericField::from_token(parts[0])
            .ok_or_else(|| format!("unknown score field '{}'", parts[0]))?;
        let min = parts[2]
            .parse::<i64>()
            .map_err(|_| format!("invalid numeric literal '{}'", parts[2]))?;
        let max = parts[3]
            .parse::<i64>()
            .map_err(|_| format!("invalid numeric literal '{}'", parts[3]))?;
        if min > max {
            return Err(format!(
                "invalid between range: min {} is greater than max {}",
                min, max
            ));
        }
        return Ok(EndingCheck::Between { field, min, max });
    }
    if parts.len() != 3 {
        return Err(format!("unsupported check expression '{}'", raw));
    }

    let field = NumericField::from_token(parts[0])
        .ok_or_else(|| format!("unknown score field '{}'", parts[0]))?;
    let op = CompareOp::from_token(parts[1])
        .ok_or_else(|| format!("unknown comparison op '{}'", parts[1]))?;
    let rhs = parts[2]
        .parse::<i64>()
        .map_err(|_| format!("invalid numeric literal '{}'", parts[2]))?;

    Ok(EndingCheck::Compare { field, op, rhs })
}

fn parse_check_list(raw: &str) -> Result<Vec<EndingCheck>, String> {
    let mut checks = Vec::new();
    for part in split_top_level_commas(raw)? {
        let parsed = parse_ending_check(part.trim())?;
        checks.push(parsed);
    }
    if checks.is_empty() {
        return Err("composite check list cannot be empty".to_string());
    }
    Ok(checks)
}

fn split_top_level_commas(raw: &str) -> Result<Vec<&str>, String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (idx, ch) in raw.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err(format!("unbalanced ')' in '{}'", raw));
                }
            }
            ',' if depth == 0 => {
                out.push(&raw[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(format!("unbalanced '(' in '{}'", raw));
    }
    let tail = &raw[start..];
    if !tail.trim().is_empty() {
        out.push(tail);
    }
    Ok(out)
}

impl NumericField {
    fn from_token(token: &str) -> Option<Self> {
        match token {
            "money" => Some(Self::Money),
            "sanity" => Some(Self::Sanity),
            "audience" => Some(Self::Audience),
            "ai_skill" => Some(Self::AiSkill),
            "friends" => Some(Self::Friends),
            "turns_recorded" => Some(Self::TurnsRecorded),
            "cash_focus_turns" => Some(Self::CashFocusTurns),
            "audience_push_turns" => Some(Self::AudiencePushTurns),
            "ai_push_turns" => Some(Self::AiPushTurns),
            "recovery_turns" => Some(Self::RecoveryTurns),
            "relationship_turns" => Some(Self::RelationshipTurns),
            "high_pressure_turns" => Some(Self::HighPressureTurns),
            "cash_focus_pct" => Some(Self::CashFocusPct),
            "high_pressure_pct" => Some(Self::HighPressurePct),
            _ => None,
        }
    }
}

impl CompareOp {
    fn from_token(token: &str) -> Option<Self> {
        match token {
            ">" => Some(Self::Gt),
            ">=" => Some(Self::Gte),
            "<" => Some(Self::Lt),
            "<=" => Some(Self::Lte),
            "==" => Some(Self::Eq),
            "!=" => Some(Self::Ne),
            _ => None,
        }
    }

    fn eval(self, lhs: i64, rhs: i64) -> bool {
        match self {
            Self::Gt => lhs > rhs,
            Self::Gte => lhs >= rhs,
            Self::Lt => lhs < rhs,
            Self::Lte => lhs <= rhs,
            Self::Eq => lhs == rhs,
            Self::Ne => lhs != rhs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ending {
    pub id: EndingId,
    pub title: String,
    pub body: String,
}

pub fn death_ending(reason: DeathReason) -> Ending {
    match reason {
        DeathReason::Broke => ending_by_id(EndingId::Broke),
        DeathReason::BurnedOut => ending_by_id(EndingId::BurnedOut),
    }
}

pub fn evaluate(state: &GameState) -> Ending {
    let catalog = endings_catalog();
    let mut best = (EndingId::Survivor, i32::MIN, i32::MIN);

    for id in EndingId::ALL {
        if matches!(id, EndingId::Broke | EndingId::BurnedOut) {
            continue;
        }
        let Some(score) = score_ending(id, state, catalog) else {
            continue;
        };
        let specificity = ending_specificity(id, catalog);
        if score > best.1 || (score == best.1 && specificity > best.2) {
            best = (id, score, specificity);
        }
    }

    ending_by_id(best.0)
}

pub fn origin_line(state: &GameState) -> &'static str {
    let lines = load_origin_lines();
    let class_id = state.character.id();
    lines
        .get(class_id)
        .map(|s| s.as_str())
        .unwrap_or("You started with nothing but a dream.")
}

fn score_ending(id: EndingId, state: &GameState, catalog: &EndingsCatalog) -> Option<i32> {
    let spec = get_scoring_spec(id, catalog)?;
    score_from_spec(spec, state)
}

fn score_from_spec(spec: &EndingScoringSpec, state: &GameState) -> Option<i32> {
    let score: i32 = spec
        .conditions
        .iter()
        .map(|cond| {
            if check_matches(&cond.check, state) {
                cond.points
            } else {
                0
            }
        })
        .sum();

    (score >= spec.min_threshold).then_some(score)
}

fn check_matches(check: &EndingCheck, state: &GameState) -> bool {
    match check {
        EndingCheck::HasFlag(flag) => ending_flag_is_set(flag, state),
        EndingCheck::NotFlag(flag) => !ending_flag_is_set(flag, state),
        EndingCheck::CharacterIs(character) => state.character.id() == character,
        EndingCheck::SubscriptionIs(tier) => state.subscription == *tier,
        EndingCheck::Compare { field, op, rhs } => {
            op.eval(numeric_field_value(*field, state), *rhs)
        }
        EndingCheck::Between { field, min, max } => {
            let value = numeric_field_value(*field, state);
            (*min..=*max).contains(&value)
        }
        EndingCheck::All(checks) => checks.iter().all(|c| check_matches(c, state)),
        EndingCheck::Any(checks) => checks.iter().any(|c| check_matches(c, state)),
    }
}

fn ending_flag_is_set(flag: &str, state: &GameState) -> bool {
    if !state.run_memory.flags.contains(flag) {
        return false;
    }
    if !requires_late_game_for_ending(flag) {
        return true;
    }
    state.run_memory.late_game_flags.contains(flag)
}

fn requires_late_game_for_ending(flag: &str) -> bool {
    matches!(flag, "blew_the_whistle")
}

fn numeric_field_value(field: NumericField, state: &GameState) -> i64 {
    let turns = state.run_patterns.turns_recorded.max(1) as i64;
    match field {
        NumericField::Money => state.stats.money,
        NumericField::Sanity => state.stats.sanity as i64,
        NumericField::Audience => state.stats.audience as i64,
        NumericField::AiSkill => state.stats.ai_skill as i64,
        NumericField::Friends => state.stats.friends as i64,
        NumericField::TurnsRecorded => state.run_patterns.turns_recorded as i64,
        NumericField::CashFocusTurns => state.run_patterns.cash_focus_turns as i64,
        NumericField::AudiencePushTurns => state.run_patterns.audience_push_turns as i64,
        NumericField::AiPushTurns => state.run_patterns.ai_push_turns as i64,
        NumericField::RecoveryTurns => state.run_patterns.recovery_turns as i64,
        NumericField::RelationshipTurns => state.run_patterns.relationship_turns as i64,
        NumericField::HighPressureTurns => state.run_patterns.high_pressure_turns as i64,
        NumericField::CashFocusPct => ((state.run_patterns.cash_focus_turns as i64) * 100) / turns,
        NumericField::HighPressurePct => {
            ((state.run_patterns.high_pressure_turns as i64) * 100) / turns
        }
    }
}

fn ending_specificity(id: EndingId, catalog: &EndingsCatalog) -> i32 {
    catalog
        .scoring
        .get(&id)
        .map(|spec| spec.specificity)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::characters::CharacterId;
    use crate::engine::conditions::Condition;
    use crate::engine::game::Game;
    use crate::engine::subscription::SubscriptionTier;
    use crate::engine::tempo::Tempo;
    use crate::engine::types::Stats;

    fn base_state() -> GameState {
        let mut g = Game::new(Tempo::default(), CharacterId::new("junior_dev"), Some(0));
        g.state.turn = Tempo::default().max_turns();
        g.state.max_turns = Tempo::default().max_turns();
        g.state.subscription = SubscriptionTier::Free;
        g.state.income_per_turn = 0;
        g.state.rent_per_turn = 0;
        g.state.stats = Stats {
            money: 10_000,
            sanity: 50,
            audience: 50,
            ai_skill: 50,
            friends: 50,
        };
        g.state.conditions = vec![Condition::ImposterSyndrome];
        g.state.run_patterns = crate::engine::game::RunPatterns::default();
        g.state
    }

    #[test]
    fn oligarch_vs_lonely() {
        let mut s = base_state();
        s.stats.money = 600_000;
        s.stats.ai_skill = 80;
        s.stats.friends = 30;
        assert_eq!(evaluate(&s).id, EndingId::Oligarch);
        s.stats.friends = 0;
        assert_eq!(evaluate(&s).id, EndingId::LonelyOligarch);
    }

    #[test]
    fn permanent_underclass_triggers() {
        let mut s = base_state();
        s.stats.money = 4_999;
        s.stats.ai_skill = 10;
        assert_eq!(evaluate(&s).id, EndingId::PermanentUnderclass);
    }

    #[test]
    fn plumber_ascendant_wins_early() {
        let mut s = base_state();
        s.character = CharacterId::new("plumber");
        s.stats.money = 1;
        s.stats.ai_skill = 0;
        s.subscription = SubscriptionTier::Free;
        // Secret endings are checked first, but plumber isn't forced into luddite unless ai=0.
        // Here ai is 0, so luddite wins first.
        assert_eq!(evaluate(&s).id, EndingId::Luddite);
        s.stats.ai_skill = 1;
        assert_eq!(evaluate(&s).id, EndingId::Irreplaceable);
    }

    #[test]
    fn sellout_scoring_from_json_triggers() {
        let mut s = base_state();
        s.stats.money = 250_000;
        s.stats.sanity = 45;
        s.stats.friends = 20;
        s.run_memory.flags.insert("sold_out".to_string());
        s.run_memory.flags.insert("embraced_ai".to_string());
        assert_eq!(evaluate(&s).id, EndingId::Sellout);
    }

    #[test]
    fn ending_catalog_contains_all_ids() {
        for id in EndingId::ALL {
            let ending = ending_by_id(id);
            assert_eq!(ending.id, id);
            assert!(!ending.title.trim().is_empty());
            assert!(!ending.body.trim().is_empty());
        }
    }

    #[test]
    fn defining_flags_only_count_if_set_in_late_game_for_endings() {
        let mut s = base_state();
        s.run_memory.flags.insert("blew_the_whistle".to_string());
        assert!(!ending_flag_is_set("blew_the_whistle", &s));

        s.run_memory
            .late_game_flags
            .insert("blew_the_whistle".to_string());
        assert!(ending_flag_is_set("blew_the_whistle", &s));

        s.run_memory.flags.insert("found_balance".to_string());
        assert!(ending_flag_is_set("found_balance", &s));
    }
}
