use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::engine::ai::Phase;
use crate::engine::characters::{CharacterCatalog, CharacterId};
use crate::engine::endings;
use crate::engine::events::TurnEvents;
use crate::engine::game::Game;
use crate::engine::llm::LlmNarrator;
use crate::engine::subscription::SubscriptionTier;
use crate::engine::tempo::Tempo;
use crate::sfx::{RunningTone, ScoreTrack, SelectionSfx, ShepardPreset, ShepardTone, VisualTempo};

const SOURCE_CODE_URL: &str = "https://github.com/DhvanilPatel/permanent-underclass";
const MAX_FREEFORM_INPUT_CHARS: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputMode {
    Cards,
    Freeform,
}

#[derive(Debug, Clone)]
pub(crate) struct InputState {
    pub(crate) mode: InputMode,
    pub(crate) buffer: String,
    pub(crate) selected: usize,
}

#[derive(Debug)]
enum LlmMsg {
    Freeform {
        turn: u32,
        result: Result<crate::engine::llm::FreeformResolution, String>,
        ms: u128,
    },
}

#[derive(Debug, Clone)]
enum LlmBusy {
    Freeform { turn: u32, action: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NarratorMode {
    Off,
    Auto,
    Claude,
    Codex,
}

impl NarratorMode {
    fn from_cli_provider(provider: Option<&str>, narrator: &Option<LlmNarrator>) -> Self {
        if let Some(raw) = provider {
            if raw.eq_ignore_ascii_case("auto") {
                return Self::Auto;
            }
            if raw.eq_ignore_ascii_case("claude") {
                return Self::Claude;
            }
            if raw.eq_ignore_ascii_case("codex") {
                return Self::Codex;
            }
        }

        match narrator {
            Some(LlmNarrator::Claude(_)) => Self::Claude,
            Some(LlmNarrator::Codex(_)) => Self::Codex,
            None => Self::Auto,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Off => Self::Auto,
            Self::Auto => Self::Claude,
            Self::Claude => Self::Codex,
            Self::Codex => Self::Off,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Off => Self::Codex,
            Self::Codex => Self::Claude,
            Self::Claude => Self::Auto,
            Self::Auto => Self::Off,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Auto => "AUTO",
            Self::Claude => "CLAUDE",
            Self::Codex => "CODEX",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AudioPrefs {
    pub(crate) score_level: u8, // 0=off, 1=low, 2=med, 3=high
    pub(crate) score_track: ScoreTrack,
    pub(crate) shepard_level: u8, // 0=off, 1=low, 2=med, 3=high
    pub(crate) sfx_on: bool,
}

impl Default for AudioPrefs {
    fn default() -> Self {
        Self {
            score_level: 3, // high by default (matches TITLE_TONE_VOLUME)
            score_track: ScoreTrack::Chiptune,
            shepard_level: 2, // med by default (matches SHEPARD_TONE_VOLUME)
            sfx_on: true,
        }
    }
}

impl AudioPrefs {
    pub(crate) const SCORE_VOLUMES: [f32; 5] = [0.0, 0.06, 0.12, 0.18, 0.30];
    pub(crate) const SHEPARD_VOLUMES: [f32; 5] = [0.0, 0.02, 0.045, 0.08, 0.14];
    pub(crate) const LEVEL_LABELS: [&'static str; 5] = ["OFF", "LOW", "MED", "HIGH", "V.HIGH"];
    pub(crate) const NUM_LEVELS: u8 = 5;
    pub(crate) const SETTINGS_ROWS: usize = 8;
}

#[derive(Debug)]
pub(crate) enum Screen {
    Title,
    TitleSettings {
        selected: usize,
    },
    Credits,
    PickClass {
        selected: usize,
    },
    Turn {
        events: TurnEvents,
        input: InputState,
    },
    ManageSub {
        selected: usize,
        events: TurnEvents,
        input: InputState,
    },
    Settings {
        selected: usize,
        events: TurnEvents,
        input: InputState,
    },
    Outcome {
        events: TurnEvents,
        outcome: crate::engine::events::TurnOutcome,
    },
    /// Brief pause after outcome so the player can see stat changes.
    StatPause {
        events: TurnEvents,
        delta: crate::engine::types::ResourceDelta,
        ticks_remaining: u8,
    },
    Ending {
        ending: endings::Ending,
        /// Index into EndingId::ALL for cycling preview (None = real ending).
        preview_idx: Option<usize>,
    },
}

pub struct App {
    pub running: bool,
    pub tick: u64,
    pub(crate) screen: Screen,
    pub(crate) game: Option<Game>,
    title_menu_selected: usize,

    narrator: Option<LlmNarrator>,
    narrator_mode: NarratorMode,

    llm_profile: bool,
    llm_tx: Sender<LlmMsg>,
    llm_rx: Receiver<LlmMsg>,
    llm_busy: Option<LlmBusy>,
    selection_sfx: Option<SelectionSfx>,
    running_tone: Option<RunningTone>,
    title_tone: Option<RunningTone>,
    shepard_tone: Option<ShepardTone>,
    pub(crate) audio_prefs: AudioPrefs,
    invincible: bool,
    default_tempo: Tempo,
    /// Konami code progress tracker.
    konami_pos: u8,
}

impl App {
    pub fn new(
        narrator: Option<LlmNarrator>,
        llm_profile: bool,
        llm_provider: Option<&str>,
        tempo: Tempo,
    ) -> Self {
        let (llm_tx, llm_rx) = mpsc::channel();
        let mut narrator = narrator;
        let narrator_mode = NarratorMode::from_cli_provider(llm_provider, &narrator);
        if narrator.is_none() {
            narrator = match narrator_mode {
                NarratorMode::Off => None,
                NarratorMode::Auto => Some(Self::resolve_auto_narrator()),
                NarratorMode::Claude => Some(LlmNarrator::claude()),
                NarratorMode::Codex => Some(LlmNarrator::codex()),
            };
        }
        let mut app = Self {
            running: true,
            tick: 0,
            screen: Screen::Title,
            game: None,
            title_menu_selected: 0,
            narrator,
            narrator_mode,
            llm_profile,
            llm_tx,
            llm_rx,
            llm_busy: None,
            selection_sfx: SelectionSfx::load_default(),
            running_tone: RunningTone::load_default(),
            title_tone: RunningTone::load_title_default(),
            shepard_tone: ShepardTone::load_default(),
            audio_prefs: AudioPrefs::default(),
            invincible: false,
            default_tempo: tempo,
            konami_pos: 0,
        };
        app.start_title_tone();
        app.start_shepard_tone();
        if let Some(tone) = &mut app.shepard_tone {
            tone.set_phase(Phase::Early);
        }
        app.apply_audio_prefs_to_active_tones();
        app
    }

    pub fn on_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if let Some(game) = &mut self.game {
            game.on_tick();
        }
        self.drain_llm_messages();

        // Auto-advance stat pause after countdown.
        if let Screen::StatPause {
            ticks_remaining, ..
        } = &mut self.screen
        {
            if *ticks_remaining <= 1 {
                // Take events out via swap.
                let mut tmp = Screen::Title;
                std::mem::swap(&mut tmp, &mut self.screen);
                if let Screen::StatPause { events, .. } = tmp {
                    self.screen = Screen::Turn {
                        events,
                        input: InputState {
                            mode: InputMode::Cards,
                            buffer: String::new(),
                            selected: 0,
                        },
                    };
                }
            } else {
                *ticks_remaining -= 1;
            }
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if matches!(key.kind, KeyEventKind::Release) {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return;
        }

        // ── Konami code: ↑↑↓↓←→←→BA → jump to ending screen ──
        {
            const KONAMI: [KeyCode; 10] = [
                KeyCode::Up,
                KeyCode::Up,
                KeyCode::Down,
                KeyCode::Down,
                KeyCode::Left,
                KeyCode::Right,
                KeyCode::Left,
                KeyCode::Right,
                KeyCode::Char('b'),
                KeyCode::Char('a'),
            ];
            if key.code == KONAMI[self.konami_pos as usize] {
                self.konami_pos += 1;
                if self.konami_pos as usize >= KONAMI.len() {
                    self.konami_pos = 0;
                    if let Some(game) = &self.game {
                        let ending = endings::evaluate(&game.state);
                        self.screen = Screen::Ending {
                            ending,
                            preview_idx: Some(0),
                        };
                    } else {
                        // No game yet — show first ending from catalog
                        self.screen = Screen::Ending {
                            ending: endings::ending_by_id(endings::EndingId::ALL[0]),
                            preview_idx: Some(0),
                        };
                    }
                    return;
                }
            } else {
                self.konami_pos = if key.code == KONAMI[0] { 1 } else { 0 };
            }
        }

        if self.llm_busy.is_some() {
            if key.code == KeyCode::Esc {
                self.llm_busy = None;
                if let Some(game) = &mut self.game {
                    game.state.push_log("LLM: canceled");
                }
            }
            return;
        }

        let mut play_selection_sfx = false;
        match (&mut self.screen, key.code) {
            (Screen::Title, KeyCode::Enter) => {
                match self.title_menu_selected {
                    0 => {
                        self.stop_title_tone();
                        self.screen = Screen::PickClass { selected: 0 };
                    }
                    1 => {
                        self.screen = Screen::TitleSettings { selected: 0 };
                    }
                    2 => {
                        self.screen = Screen::Credits;
                    }
                    3 => {
                        let _ = Self::open_url(SOURCE_CODE_URL);
                    }
                    4 => {
                        self.running = false;
                    }
                    _ => {}
                }
                play_selection_sfx = true;
            }
            (Screen::Title, KeyCode::Up) => {
                let next = self.title_menu_selected.saturating_sub(1);
                if next != self.title_menu_selected {
                    self.title_menu_selected = next;
                    play_selection_sfx = true;
                }
            }
            (Screen::Title, KeyCode::Down) => {
                let next = (self.title_menu_selected + 1).min(4);
                if next != self.title_menu_selected {
                    self.title_menu_selected = next;
                    play_selection_sfx = true;
                }
            }
            // Some terminals can emit a spurious Esc around startup/focus changes.
            // Ignore Esc on title to avoid accidental immediate app exit.
            (Screen::Title, KeyCode::Esc) => {}
            (Screen::TitleSettings { selected }, KeyCode::Up) => {
                let next = selected.saturating_sub(1);
                if next != *selected {
                    *selected = next;
                    play_selection_sfx = true;
                }
            }
            (Screen::TitleSettings { selected }, KeyCode::Down) => {
                let next = (*selected + 1).min(AudioPrefs::SETTINGS_ROWS - 1);
                if next != *selected {
                    *selected = next;
                    play_selection_sfx = true;
                }
            }
            (Screen::TitleSettings { selected }, KeyCode::Enter | KeyCode::Right) => {
                let row = *selected;
                self.adjust_settings_row(row, true);
                play_selection_sfx = true;
            }
            (Screen::TitleSettings { selected }, KeyCode::Left) => {
                let row = *selected;
                self.adjust_settings_row(row, false);
                play_selection_sfx = true;
            }
            (Screen::TitleSettings { .. }, KeyCode::Esc) => {
                self.screen = Screen::Title;
            }
            (Screen::Credits, KeyCode::Esc) => {
                self.screen = Screen::Title;
                play_selection_sfx = true;
            }

            (Screen::PickClass { selected }, KeyCode::Left) => {
                let Ok(catalog) = CharacterCatalog::embedded() else {
                    eprintln!("WARN: character catalog unavailable in pick screen");
                    self.title_menu_selected = 0;
                    self.screen = Screen::Title;
                    return;
                };
                let len = catalog.all().len().max(1);
                *selected = (*selected).min(len - 1);
                *selected = (*selected + len - 1) % len;
                play_selection_sfx = true;
            }
            (Screen::PickClass { selected }, KeyCode::Right) => {
                let Ok(catalog) = CharacterCatalog::embedded() else {
                    eprintln!("WARN: character catalog unavailable in pick screen");
                    self.title_menu_selected = 0;
                    self.screen = Screen::Title;
                    return;
                };
                let len = catalog.all().len().max(1);
                *selected = (*selected).min(len - 1);
                *selected = (*selected + 1) % len;
                play_selection_sfx = true;
            }
            (Screen::PickClass { selected }, KeyCode::Up) => {
                let Ok(catalog) = CharacterCatalog::embedded() else {
                    eprintln!("WARN: character catalog unavailable in pick screen");
                    self.title_menu_selected = 0;
                    self.screen = Screen::Title;
                    return;
                };
                let len = catalog.all().len().max(1);
                let cols = 5usize.min(len).max(1);
                *selected = (*selected).min(len - 1);
                let col = *selected % cols;
                *selected = if *selected >= cols {
                    *selected - cols
                } else {
                    let last_row_start = ((len - 1) / cols) * cols;
                    let mut candidate = last_row_start + col;
                    if candidate >= len {
                        candidate = len - 1;
                    }
                    candidate
                };
                play_selection_sfx = true;
            }
            (Screen::PickClass { selected }, KeyCode::Down) => {
                let Ok(catalog) = CharacterCatalog::embedded() else {
                    eprintln!("WARN: character catalog unavailable in pick screen");
                    self.title_menu_selected = 0;
                    self.screen = Screen::Title;
                    return;
                };
                let len = catalog.all().len().max(1);
                let cols = 5usize.min(len).max(1);
                *selected = (*selected).min(len - 1);
                let col = *selected % cols;
                *selected = if *selected + cols < len {
                    *selected + cols
                } else {
                    col.min(len - 1)
                };
                play_selection_sfx = true;
            }
            (Screen::PickClass { selected }, KeyCode::Enter) => {
                let Ok(catalog) = CharacterCatalog::embedded() else {
                    eprintln!("WARN: character catalog unavailable in pick screen");
                    self.title_menu_selected = 0;
                    self.screen = Screen::Title;
                    return;
                };
                let characters = catalog.all();
                if let Some(character) = characters.get(*selected) {
                    self.start_game(character.id.clone(), None);
                }
            }
            (Screen::PickClass { .. }, KeyCode::Esc) => {
                self.start_title_tone();
                self.start_shepard_tone();
                if let Some(tone) = &mut self.shepard_tone {
                    tone.set_phase(Phase::Early);
                }
                self.title_menu_selected = 0;
                self.screen = Screen::Title;
            }

            // Arrow navigation for choices (0..2 = cards, 3 = freeform).
            (Screen::Turn { events, input }, KeyCode::Left | KeyCode::Up)
                if input.mode == InputMode::Cards =>
            {
                let count = if events.personal.allow_freeform {
                    events.personal.choices.len().min(3) + 1
                } else {
                    events.personal.choices.len().min(3)
                };
                input.selected = (input.selected + count - 1) % count;
                play_selection_sfx = true;
            }
            (Screen::Turn { events, input }, KeyCode::Right | KeyCode::Down)
                if input.mode == InputMode::Cards =>
            {
                let count = if events.personal.allow_freeform {
                    events.personal.choices.len().min(3) + 1
                } else {
                    events.personal.choices.len().min(3)
                };
                input.selected = (input.selected + 1) % count;
                play_selection_sfx = true;
            }
            (Screen::Turn { events, input }, KeyCode::Enter) if input.mode == InputMode::Cards => {
                let num_choices = events.personal.choices.len().min(3);
                if input.selected < num_choices {
                    let key = (b'1' + input.selected as u8) as char;
                    let Some(game) = &mut self.game else {
                        return;
                    };
                    let outcome = game.resolve_choice(events, key);
                    if let Some(reason) = if self.invincible {
                        None
                    } else {
                        game.death_reason()
                    } {
                        self.screen = Screen::Ending {
                            ending: endings::death_ending(reason),
                            preview_idx: None,
                        };
                    } else if game.is_final_turn() {
                        self.screen = Screen::Ending {
                            ending: endings::evaluate(&game.state),
                            preview_idx: None,
                        };
                    } else {
                        self.screen = Screen::Outcome {
                            events: events.clone(),
                            outcome,
                        };
                    }
                } else if input.selected == num_choices && events.personal.allow_freeform {
                    input.mode = InputMode::Freeform;
                    input.buffer.clear();
                }
            }

            (Screen::Turn { events, input }, KeyCode::Char('4'))
                if input.mode == InputMode::Cards =>
            {
                input.selected = events.personal.choices.len().min(3);
                input.mode = InputMode::Freeform;
                input.buffer.clear();
            }

            (Screen::Turn { events, input }, KeyCode::Char('s'))
                if input.mode == InputMode::Cards =>
            {
                let Some(game) = &self.game else {
                    return;
                };
                let current = game.state.subscription;
                let mut selected = SubscriptionTier::ALL
                    .iter()
                    .position(|t| *t == current)
                    .unwrap_or(0);
                if !game.can_set_subscription(SubscriptionTier::ALL[selected])
                    && let Some(first_unlocked) = SubscriptionTier::ALL
                        .iter()
                        .position(|t| game.can_set_subscription(*t))
                {
                    selected = first_unlocked;
                }
                self.screen = Screen::ManageSub {
                    selected,
                    events: events.clone(),
                    input: input.clone(),
                };
            }

            (Screen::Turn { events, input }, KeyCode::Char('m'))
                if input.mode == InputMode::Cards =>
            {
                self.screen = Screen::Settings {
                    selected: 0,
                    events: events.clone(),
                    input: input.clone(),
                };
            }

            (Screen::ManageSub { selected, .. }, KeyCode::Up) => {
                let Some(game) = &self.game else {
                    return;
                };
                let len = SubscriptionTier::ALL.len();
                let mut next = *selected;
                let mut found = false;
                for _ in 0..len {
                    next = (next + len - 1) % len;
                    if game.can_set_subscription(SubscriptionTier::ALL[next]) {
                        found = true;
                        break;
                    }
                }
                if found && next != *selected {
                    *selected = next;
                    play_selection_sfx = true;
                }
            }
            (Screen::ManageSub { selected, .. }, KeyCode::Down) => {
                let Some(game) = &self.game else {
                    return;
                };
                let len = SubscriptionTier::ALL.len();
                let mut next = *selected;
                let mut found = false;
                for _ in 0..len {
                    next = (next + 1) % len;
                    if game.can_set_subscription(SubscriptionTier::ALL[next]) {
                        found = true;
                        break;
                    }
                }
                if found && next != *selected {
                    *selected = next;
                    play_selection_sfx = true;
                }
            }
            (
                Screen::ManageSub {
                    selected,
                    events,
                    input,
                },
                KeyCode::Enter,
            ) => {
                let Some(game) = &mut self.game else {
                    return;
                };
                let tier = SubscriptionTier::ALL[*selected];
                if !game.can_set_subscription(tier) {
                    return;
                }
                let _ = game.set_subscription(tier);
                if let Some(reason) = if self.invincible {
                    None
                } else {
                    game.death_reason()
                } {
                    self.screen = Screen::Ending {
                        ending: endings::death_ending(reason),
                        preview_idx: None,
                    };
                } else {
                    self.screen = Screen::Turn {
                        events: events.clone(),
                        input: input.clone(),
                    };
                }
            }
            (Screen::ManageSub { events, input, .. }, KeyCode::Esc) => {
                self.screen = Screen::Turn {
                    events: events.clone(),
                    input: input.clone(),
                };
            }

            (Screen::Settings { selected, .. }, KeyCode::Up) => {
                let next = selected.saturating_sub(1);
                if next != *selected {
                    *selected = next;
                    play_selection_sfx = true;
                }
            }
            (Screen::Settings { selected, .. }, KeyCode::Down) => {
                let next = (*selected + 1).min(AudioPrefs::SETTINGS_ROWS - 1);
                if next != *selected {
                    *selected = next;
                    play_selection_sfx = true;
                }
            }
            (Screen::Settings { selected, .. }, KeyCode::Enter | KeyCode::Right) => {
                let row = *selected;
                self.adjust_settings_row(row, true);
                play_selection_sfx = true;
            }
            (Screen::Settings { selected, .. }, KeyCode::Left) => {
                let row = *selected;
                self.adjust_settings_row(row, false);
                play_selection_sfx = true;
            }
            (Screen::Settings { events, input, .. }, KeyCode::Esc) => {
                self.screen = Screen::Turn {
                    events: events.clone(),
                    input: input.clone(),
                };
            }

            (Screen::Turn { input, .. }, KeyCode::Esc) if input.mode == InputMode::Freeform => {
                input.mode = InputMode::Cards;
                input.buffer.clear();
            }

            (Screen::Turn { input, .. }, KeyCode::Backspace)
                if input.mode == InputMode::Freeform =>
            {
                input.buffer.pop();
            }

            (Screen::Turn { events, input }, KeyCode::Enter)
                if input.mode == InputMode::Freeform =>
            {
                let Some(game) = &mut self.game else {
                    return;
                };
                if let Some(narrator) = self.narrator.clone() {
                    let action = input.buffer.clone();
                    let state = game.state.clone();
                    let events_snapshot = events.clone();
                    let turn = game.state.turn;

                    if self.llm_profile {
                        game.state.push_log("LLM: resolving freeform...");
                    }

                    self.llm_busy = Some(LlmBusy::Freeform {
                        turn,
                        action: action.clone(),
                    });

                    let tx = self.llm_tx.clone();
                    std::thread::spawn(move || {
                        let start = Instant::now();
                        let result = narrator
                            .resolve_freeform(&state, &events_snapshot, &action)
                            .map_err(|e| e.to_string());
                        let ms = start.elapsed().as_millis();
                        let _ = tx.send(LlmMsg::Freeform { turn, result, ms });
                    });
                } else {
                    let outcome = game.resolve_freeform(events, &input.buffer);
                    if let Some(reason) = if self.invincible {
                        None
                    } else {
                        game.death_reason()
                    } {
                        self.screen = Screen::Ending {
                            ending: endings::death_ending(reason),
                            preview_idx: None,
                        };
                    } else if game.is_final_turn() {
                        self.screen = Screen::Ending {
                            ending: endings::evaluate(&game.state),
                            preview_idx: None,
                        };
                    } else {
                        self.screen = Screen::Outcome {
                            events: events.clone(),
                            outcome,
                        };
                    }
                }
            }

            (Screen::Turn { events, input }, KeyCode::Char(c))
                if input.mode == InputMode::Freeform && !c.is_control() =>
            {
                if input.buffer.chars().count() < MAX_FREEFORM_INPUT_CHARS {
                    input.buffer.push(c);
                }
            }

            (Screen::Turn { events, input }, KeyCode::Char(c))
                if input.mode == InputMode::Cards && matches!(c, '1' | '2' | '3') =>
            {
                input.selected = (c as u8 - b'1') as usize;
                let Some(game) = &mut self.game else {
                    return;
                };
                let outcome = game.resolve_choice(events, c);
                if let Some(reason) = if self.invincible {
                    None
                } else {
                    game.death_reason()
                } {
                    self.screen = Screen::Ending {
                        ending: endings::death_ending(reason),
                        preview_idx: None,
                    };
                } else if game.is_final_turn() {
                    self.screen = Screen::Ending {
                        ending: endings::evaluate(&game.state),
                        preview_idx: None,
                    };
                } else {
                    self.screen = Screen::Outcome {
                        events: events.clone(),
                        outcome,
                    };
                }
            }

            (Screen::Outcome { outcome, .. }, KeyCode::Enter) => {
                let delta = outcome.delta;
                let Some(game) = &mut self.game else {
                    return;
                };
                if game.is_final_turn() {
                    self.screen = Screen::Ending {
                        ending: endings::evaluate(&game.state),
                        preview_idx: None,
                    };
                    return;
                }
                let events = game.begin_turn();
                let death_reason = if self.invincible {
                    None
                } else {
                    game.death_reason()
                };
                // Drop the mutable borrow on game before calling self methods.
                if let Some(reason) = death_reason {
                    self.screen = Screen::Ending {
                        ending: endings::death_ending(reason),
                        preview_idx: None,
                    };
                } else {
                    self.update_shepard_for_turn();
                    // Brief pause so the player can see stat changes before the next question.
                    self.screen = Screen::StatPause {
                        events,
                        delta,
                        ticks_remaining: 4, // ~720ms at 180ms tick rate
                    };
                }
            }
            // During stat pause: any key skips the pause.
            (Screen::StatPause { .. }, _) => {
                let mut tmp = Screen::Title;
                std::mem::swap(&mut tmp, &mut self.screen);
                if let Screen::StatPause { events, .. } = tmp {
                    self.screen = Screen::Turn {
                        events,
                        input: InputState {
                            mode: InputMode::Cards,
                            buffer: String::new(),
                            selected: 0,
                        },
                    };
                }
            }

            (Screen::Ending { .. }, KeyCode::Char('p'))
            | (Screen::Ending { .. }, KeyCode::Enter) => {
                // "Play again" -> back to title (fresh seed).
                self.game = None;
                self.stop_running_tone();
                self.stop_shepard_tone();
                self.start_title_tone();
                self.start_shepard_tone();
                if let Some(tone) = &mut self.shepard_tone {
                    tone.set_phase(Phase::Early);
                }
                self.title_menu_selected = 0;
                self.screen = Screen::Title;
            }
            (Screen::Ending { preview_idx, .. }, KeyCode::Char('c')) => {
                use crate::engine::endings::{EndingId, ending_by_id};
                let next = match preview_idx {
                    Some(i) => (*i + 1) % EndingId::ALL.len(),
                    None => 0,
                };
                self.screen = Screen::Ending {
                    ending: ending_by_id(EndingId::ALL[next]),
                    preview_idx: Some(next),
                };
            }
            _ => {}
        }

        if !self.running {
            self.stop_title_tone();
            self.stop_running_tone();
            self.stop_shepard_tone();
        }

        if play_selection_sfx {
            self.play_selection_sfx();
        }
    }

    pub fn llm_overlay(&self) -> Option<&'static str> {
        match self.llm_busy {
            Some(LlmBusy::Freeform { .. }) => match self.narrator {
                Some(LlmNarrator::Claude(_)) => Some("Claude is computing your consequences..."),
                Some(LlmNarrator::Codex(_)) => Some("Codex is computing your consequences..."),
                None => Some("computing your consequences..."),
            },
            None => None,
        }
    }

    pub fn llm_status(&self) -> Option<&'static str> {
        if self.llm_busy.is_some() {
            return Some("LLM Showrunner: thinking");
        }
        match self.narrator_mode {
            NarratorMode::Off => None,
            NarratorMode::Auto => match self.narrator {
                Some(LlmNarrator::Claude(_)) => Some("LLM Showrunner: auto (claude)"),
                Some(LlmNarrator::Codex(_)) => Some("LLM Showrunner: auto (codex)"),
                None => Some("LLM Showrunner: auto"),
            },
            NarratorMode::Codex => Some("LLM Showrunner: codex"),
            NarratorMode::Claude => Some("LLM Showrunner: claude"),
        }
    }

    pub fn narrator_label(&self) -> &'static str {
        self.narrator_mode.label()
    }

    pub fn cheat_mode_label(&self) -> &'static str {
        if self.invincible { "INVINCIBLE" } else { "OFF" }
    }

    pub fn tempo_label(&self) -> String {
        format!(
            "{} ({} TURNS)",
            self.default_tempo.display_name(),
            self.default_tempo.max_turns()
        )
    }

    pub(crate) fn title_menu_selected(&self) -> usize {
        self.title_menu_selected
    }

    pub(crate) fn visual_tempo(&self) -> VisualTempo {
        self.audio_prefs.score_track.visual_tempo()
    }

    pub fn shepard_preset_label(&self) -> String {
        match &self.shepard_tone {
            Some(tone) if tone.preset().is_auto() => {
                format!("AUTO ({})", tone.active_label())
            }
            Some(tone) => tone.preset().label().to_string(),
            None => "OFF".to_string(),
        }
    }

    fn drain_llm_messages(&mut self) {
        while let Ok(msg) = self.llm_rx.try_recv() {
            self.handle_llm_msg(msg);
        }
    }

    fn handle_llm_msg(&mut self, msg: LlmMsg) {
        match msg {
            LlmMsg::Freeform { turn, result, ms } => {
                let Some(LlmBusy::Freeform {
                    turn: busy_turn,
                    action,
                }) = self.llm_busy.clone()
                else {
                    return;
                };
                if busy_turn != turn {
                    return;
                }
                self.llm_busy = None;

                let Some(game) = &mut self.game else {
                    return;
                };
                match &result {
                    Ok(_) => {
                        if self.llm_profile {
                            game.state.push_log(format!("LLM: freeform ok ({}ms)", ms));
                        }
                    }
                    Err(e) => {
                        if self.llm_profile {
                            game.state.push_log(format!("LLM: freeform failed: {}", e));
                        } else {
                            game.state
                                .push_log("LLM: freeform failed; using offline fallback");
                        }
                    }
                }

                if game.state.turn != turn {
                    return;
                }
                let Screen::Turn { events, .. } = &mut self.screen else {
                    return;
                };

                let outcome = match result {
                    Ok(res) => game.apply_freeform(
                        res.delta,
                        res.effects,
                        format!("{}\n{}", res.title, res.narrative),
                        res.summary_line,
                    ),
                    Err(_) => game.resolve_freeform(events, &action),
                };

                if let Some(reason) = if self.invincible {
                    None
                } else {
                    game.death_reason()
                } {
                    self.screen = Screen::Ending {
                        ending: endings::death_ending(reason),
                        preview_idx: None,
                    };
                } else if game.is_final_turn() {
                    self.screen = Screen::Ending {
                        ending: endings::evaluate(&game.state),
                        preview_idx: None,
                    };
                } else {
                    self.screen = Screen::Outcome {
                        events: events.clone(),
                        outcome,
                    };
                }
            }
        }
    }

    pub fn help_line(&self) -> &'static str {
        match self.screen {
            Screen::Title => "Up/Down: menu | Enter: confirm | Ctrl+C: quit",
            Screen::TitleSettings { .. } => {
                "Up/Down: select | Left/Right/Enter: adjust | Esc: back | Ctrl+C: quit"
            }
            Screen::Credits => "Esc: back | Ctrl+C: quit",
            Screen::PickClass { .. } => {
                "Arrows: move grid | Enter: confirm | Esc: back | Ctrl+C: quit"
            }
            Screen::Turn { .. } => {
                "Arrows: select | Enter: confirm | s: sub | m: settings | Ctrl+C: quit"
            }
            Screen::ManageSub { .. } => {
                "Up/Down: select | Enter: confirm | Esc: back | Ctrl+C: quit"
            }
            Screen::Settings { .. } => {
                "Up/Down: select | Left/Right/Enter: toggle | Esc: back | Ctrl+C: quit"
            }
            Screen::Outcome { .. } => "Enter: continue | Ctrl+C: quit",
            Screen::StatPause { .. } => "",
            Screen::Ending { .. } => "Enter/p: play again | c: cycle endings | Ctrl+C: quit",
        }
    }
}

impl App {
    pub(crate) fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Returns the delta to withhold from displayed stats so the player sees
    /// "before" values while the outcome modal is open and during early ticks
    /// of the stat pause, then the numbers visually jump to new values.
    pub(crate) fn stat_pause_pending_delta(&self) -> Option<crate::engine::types::ResourceDelta> {
        match &self.screen {
            // While the outcome modal is showing, keep stats at pre-choice values.
            Screen::Outcome { outcome, .. } => Some(outcome.delta),
            // First half of the pause: still withhold. Second half: reveal.
            Screen::StatPause {
                delta,
                ticks_remaining,
                ..
            } if *ticks_remaining > 2 => Some(*delta),
            _ => None,
        }
    }

    fn start_game(&mut self, character: CharacterId, seed: Option<u64>) {
        self.stop_title_tone();
        let mut game = Game::new(self.default_tempo, character, seed);
        let events = game.begin_turn();

        if let Some(reason) = if self.invincible {
            None
        } else {
            game.death_reason()
        } {
            self.game = Some(game);
            self.screen = Screen::Ending {
                ending: endings::death_ending(reason),
                preview_idx: None,
            };
            return;
        }

        self.game = Some(game);
        self.start_running_tone();
        self.start_shepard_tone();
        self.update_shepard_for_turn();
        // Apply current audio prefs after starting (sinks must exist first).
        self.apply_audio_prefs_to_active_tones();
        self.screen = Screen::Turn {
            events,
            input: InputState {
                mode: InputMode::Cards,
                buffer: String::new(),
                selected: 0,
            },
        };
    }

    fn play_selection_sfx(&self) {
        if let Some(sfx) = &self.selection_sfx {
            sfx.play_move();
        }
    }

    fn set_narrator_mode(&mut self, mode: NarratorMode) {
        self.narrator_mode = mode;
        self.narrator = match mode {
            NarratorMode::Off => None,
            NarratorMode::Auto => Some(Self::resolve_auto_narrator()),
            NarratorMode::Claude => Some(LlmNarrator::claude()),
            NarratorMode::Codex => Some(LlmNarrator::codex()),
        };
    }

    fn adjust_settings_row(&mut self, row: usize, forward: bool) {
        match row {
            0 => self.cycle_score_level(forward),
            1 => self.cycle_score_track(forward),
            2 => self.cycle_shepard_level(forward),
            3 => self.cycle_shepard_preset(forward),
            4 => self.toggle_selection_sfx(),
            5 => {
                let next = if forward {
                    self.narrator_mode.next()
                } else {
                    self.narrator_mode.prev()
                };
                self.set_narrator_mode(next);
            }
            6 => self.toggle_invincible(),
            7 => self.cycle_default_tempo(forward),
            _ => {}
        }
    }

    fn cycle_score_level(&mut self, forward: bool) {
        let current = self.audio_prefs.score_level;
        let next = if forward {
            (current + 1) % AudioPrefs::NUM_LEVELS
        } else {
            (current + AudioPrefs::NUM_LEVELS - 1) % AudioPrefs::NUM_LEVELS
        };
        self.audio_prefs.score_level = next;
        self.apply_audio_prefs_to_active_tones();
    }

    fn cycle_shepard_level(&mut self, forward: bool) {
        let current = self.audio_prefs.shepard_level;
        let next = if forward {
            (current + 1) % AudioPrefs::NUM_LEVELS
        } else {
            (current + AudioPrefs::NUM_LEVELS - 1) % AudioPrefs::NUM_LEVELS
        };
        self.audio_prefs.shepard_level = next;
        self.apply_audio_prefs_to_active_tones();
    }

    fn cycle_shepard_preset(&mut self, forward: bool) {
        let current = self
            .shepard_tone
            .as_ref()
            .map(|t| t.preset())
            .unwrap_or(ShepardPreset::Auto);
        let next = if forward {
            current.next()
        } else {
            current.prev()
        };
        if let Some(tone) = &mut self.shepard_tone {
            tone.set_preset(next);
        }
    }

    fn cycle_score_track(&mut self, forward: bool) {
        self.audio_prefs.score_track = if forward {
            self.audio_prefs.score_track.next()
        } else {
            self.audio_prefs.score_track.prev()
        };
        if let Some(tone) = &mut self.title_tone {
            tone.set_track(self.audio_prefs.score_track);
        }
        if let Some(tone) = &mut self.running_tone {
            tone.set_track(self.audio_prefs.score_track);
        }
    }

    fn toggle_selection_sfx(&mut self) {
        self.audio_prefs.sfx_on = !self.audio_prefs.sfx_on;
        if let Some(sfx) = &mut self.selection_sfx {
            sfx.set_muted(!self.audio_prefs.sfx_on);
        }
    }

    fn toggle_invincible(&mut self) {
        self.invincible = !self.invincible;
    }

    fn cycle_default_tempo(&mut self, forward: bool) {
        self.default_tempo = match (self.default_tempo, forward) {
            (Tempo::Simple, true) => Tempo::Standard,
            (Tempo::Standard, true) => Tempo::Simple,
            (Tempo::Simple, false) => Tempo::Standard,
            (Tempo::Standard, false) => Tempo::Simple,
        };
    }

    fn apply_audio_prefs_to_active_tones(&mut self) {
        let score_vol = AudioPrefs::SCORE_VOLUMES[self.audio_prefs.score_level as usize];
        let shepard_vol = AudioPrefs::SHEPARD_VOLUMES[self.audio_prefs.shepard_level as usize];
        if let Some(tone) = &mut self.title_tone {
            tone.set_volume(score_vol);
        }
        if let Some(tone) = &mut self.running_tone {
            tone.set_volume(score_vol);
        }
        if let Some(tone) = &mut self.shepard_tone {
            tone.set_volume(shepard_vol);
        }
    }

    fn resolve_auto_narrator() -> LlmNarrator {
        if Self::is_cli_available("claude") {
            LlmNarrator::claude()
        } else {
            LlmNarrator::codex()
        }
    }

    fn is_cli_available(cmd: &str) -> bool {
        use std::process::{Command, Stdio};

        Command::new(cmd)
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
    }

    fn open_url(url: &str) -> bool {
        use std::process::Command;

        #[cfg(target_os = "macos")]
        {
            Command::new("open").arg(url).status().is_ok()
        }
        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open").arg(url).status().is_ok()
        }
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd")
                .args(["/C", "start", "", url])
                .status()
                .is_ok()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = url;
            false
        }
    }

    fn start_running_tone(&mut self) {
        if let Some(tone) = &mut self.running_tone {
            tone.set_track(self.audio_prefs.score_track);
            tone.start();
        }
    }

    fn stop_running_tone(&mut self) {
        if let Some(tone) = &mut self.running_tone {
            tone.stop();
        }
    }

    fn start_shepard_tone(&mut self) {
        if let Some(tone) = &mut self.shepard_tone {
            tone.start();
        }
    }

    fn stop_shepard_tone(&mut self) {
        if let Some(tone) = &mut self.shepard_tone {
            tone.stop();
        }
    }

    fn update_shepard_for_turn(&mut self) {
        if let Some(game) = &self.game {
            let phase = game.state.phase();
            if let Some(tone) = &mut self.shepard_tone {
                tone.set_phase(phase);
            }
        }
    }

    fn start_title_tone(&mut self) {
        if let Some(tone) = &mut self.title_tone {
            tone.set_track(self.audio_prefs.score_track);
            tone.start();
        }
    }

    fn stop_title_tone(&mut self) {
        if let Some(tone) = &mut self.title_tone {
            tone.stop();
        }
    }
}
