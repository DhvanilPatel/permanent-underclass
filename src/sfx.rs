use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(target_os = "macos")]
use std::process::Command;

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

const SELECT_SFX_ENV: &str = "PU_SELECTION_SFX";
const DEFAULT_SELECT_SFX_PATH: &str = "data/sfx/glitch_004.mp3";
const TITLE_TONE_ENV: &str = "PU_TITLE_TONE";
const DEFAULT_TITLE_TONE_PATH: &str = "data/sfx/gamevideo_seamless_looping_tra_elevenlabs.mp3";
const RUNNING_TONE_ENV: &str = "PU_RUNNING_TONE";
const DEFAULT_RUNNING_TONE_PATH: &str = DEFAULT_TITLE_TONE_PATH;
const CHIPTUNE_SCORE_PATH: &str = "data/sfx/cutilatk_ringtone_4_387502.mp3";
const TECHNO_SCORE_PATH: &str =
    "data/sfx/lesiakower_8_bit_energetic_drums_chiptune_beat_384593.mp3";
const SHEPARD_TONE_ENV: &str = "PU_SHEPARD_TONE";
const TITLE_TONE_VOLUME: f32 = 0.18;
const RUNNING_TONE_VOLUME: f32 = 0.0;
const SHEPARD_TONE_VOLUME: f32 = 0.045;

/// Selectable background score tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScoreTrack {
    Synth,
    Chiptune,
    Techno,
}

/// Visual tempo parameters driven by the current score track.
/// Passed to rendering functions so animations sync with the music's energy.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VisualTempo {
    /// Multiplier for matrix rain fall speed (1.0 = default).
    pub rain_speed: f32,
    /// Matrix rain trail length.
    pub rain_trail: i64,
    /// Glitch fires every N ticks (lower = more frequent).
    pub glitch_every: u64,
    /// Glitch character replacement rate denominator (lower = more glitchy).
    pub glitch_rate: u64,
    /// Ticker scroll chars per tick.
    pub ticker_speed: usize,
}

impl ScoreTrack {
    pub(crate) const ALL: [ScoreTrack; 3] =
        [ScoreTrack::Synth, ScoreTrack::Chiptune, ScoreTrack::Techno];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            ScoreTrack::Synth => "SYNTH",
            ScoreTrack::Chiptune => "CHIPTUNE",
            ScoreTrack::Techno => "TECHNO",
        }
    }

    pub(crate) fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub(crate) fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    /// Visual animation parameters derived from audio analysis (ffmpeg BPM + RMS):
    ///   Synth:    104 BPM, -11.2 dB RMS (bass-heavy, moderate tempo, compressed)
    ///   Chiptune: 144 BPM, -20.9 dB RMS (balanced spectrum, fast, dynamic/quiet)
    ///   Techno:   144 BPM, -13.5 dB RMS (loud, punchy, lots of high-end)
    ///
    /// Values are exaggerated from raw ratios so the visual difference is obvious.
    pub(crate) const fn visual_tempo(self) -> VisualTempo {
        match self {
            // Slow, heavy, brooding — bass-dominated synth drone
            ScoreTrack::Synth => VisualTempo {
                rain_speed: 0.7, // slow drip, 104 BPM baseline
                rain_trail: 24,  // long ghostly trails (bass = heavy sustain)
                glitch_every: 5, // infrequent glitch pulses
                glitch_rate: 30, // sparse corruption
                ticker_speed: 1, // slow crawl
            },
            // Upbeat, airy, rhythmic — balanced melodic chiptune line
            ScoreTrack::Chiptune => VisualTempo {
                rain_speed: 1.5, // brisk, 144 BPM
                rain_trail: 14,  // medium trails (clean mix)
                glitch_every: 3, // regular pulse
                glitch_rate: 20, // moderate glitch
                ticker_speed: 3, // lively scroll
            },
            // Fast, aggressive, relentless — loud 8-bit drums
            ScoreTrack::Techno => VisualTempo {
                rain_speed: 2.5, // rapid cascade, 144 BPM + loud energy
                rain_trail: 8,   // short staccato trails (punchy transients)
                glitch_every: 2, // constant glitch
                glitch_rate: 10, // heavy corruption
                ticker_speed: 5, // frantic scroll
            },
        }
    }

    fn resolve_path(self) -> Option<PathBuf> {
        match self {
            ScoreTrack::Synth => resolve_audio_path(TITLE_TONE_ENV, &[DEFAULT_TITLE_TONE_PATH]),
            ScoreTrack::Chiptune => resolve_audio_path("PU_CHIPTUNE_SCORE", &[CHIPTUNE_SCORE_PATH]),
            ScoreTrack::Techno => resolve_audio_path("PU_TECHNO_SCORE", &[TECHNO_SCORE_PATH]),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SelectionSfx {
    path: PathBuf,
    muted: bool,
}

impl SelectionSfx {
    pub(crate) fn load_default() -> Option<Self> {
        resolve_audio_path(SELECT_SFX_ENV, &[DEFAULT_SELECT_SFX_PATH])
            .map(|path| Self { path, muted: false })
    }

    pub(crate) fn play_move(&self) {
        if !self.muted {
            play_mp3(&self.path);
        }
    }

    pub(crate) fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }
}

pub(crate) struct RunningTone {
    path: PathBuf,
    volume: f32,
    stream: Option<OutputStream>,
    sink: Option<Sink>,
}

impl RunningTone {
    pub(crate) fn load_default() -> Option<Self> {
        let path = resolve_audio_path(RUNNING_TONE_ENV, &[DEFAULT_RUNNING_TONE_PATH])
            .or_else(|| resolve_audio_path(TITLE_TONE_ENV, &[]))?;
        Some(Self::new(path, RUNNING_TONE_VOLUME))
    }

    pub(crate) fn load_title_default() -> Option<Self> {
        resolve_audio_path(TITLE_TONE_ENV, &[DEFAULT_TITLE_TONE_PATH])
            .map(|path| Self::new(path, TITLE_TONE_VOLUME))
    }

    pub(crate) fn start(&mut self) {
        if self.sink.is_some() {
            return;
        }

        let Ok(file) = File::open(&self.path) else {
            return;
        };
        let Ok(looped) = Decoder::new_looped(file) else {
            return;
        };
        let Ok(mut stream) = OutputStreamBuilder::open_default_stream() else {
            return;
        };
        // Expected lifecycle here: we start/stop streams as screens change.
        stream.log_on_drop(false);

        let sink = Sink::connect_new(stream.mixer());
        sink.set_volume(self.volume);
        sink.append(looped);
        sink.play();

        self.stream = Some(stream);
        self.sink = Some(sink);
    }

    pub(crate) fn set_volume(&mut self, vol: f32) {
        self.volume = vol;
        if let Some(sink) = &self.sink {
            sink.set_volume(vol);
        }
    }

    /// Switch to a different track. Restarts playback if currently playing.
    pub(crate) fn set_track(&mut self, track: ScoreTrack) {
        let Some(new_path) = track.resolve_path() else {
            return;
        };
        if self.path == new_path {
            return;
        }
        self.path = new_path;
        if self.sink.is_some() {
            self.stop();
            self.start();
        }
    }

    pub(crate) fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.stream = None;
    }

    fn new(path: PathBuf, volume: f32) -> Self {
        Self {
            path,
            volume,
            stream: None,
            sink: None,
        }
    }
}

impl Drop for RunningTone {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(crate) struct ShepardTone {
    stream: Option<OutputStream>,
    sink: Option<Sink>,
    preset: ShepardPreset, // user-chosen mode (Auto or a fixed preset)
    active: ShepardPreset, // resolved preset actually driving audio
    volume: f32,
}

/// Shepard tone presets ordered by escalating tension.
/// Auto mode advances through: Drift → Ambient → Default → Dread based on game phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShepardPreset {
    Auto,
    Drift,
    Ambient,
    Default,
    Dread,
}

impl ShepardPreset {
    pub(crate) const ALL: [ShepardPreset; 5] = [
        ShepardPreset::Auto,
        ShepardPreset::Drift,
        ShepardPreset::Ambient,
        ShepardPreset::Default,
        ShepardPreset::Dread,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            ShepardPreset::Auto => "AUTO",
            ShepardPreset::Drift => "DRIFT",
            ShepardPreset::Ambient => "AMBIENT",
            ShepardPreset::Default => "DEFAULT",
            ShepardPreset::Dread => "DREAD",
        }
    }

    pub(crate) fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub(crate) fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|p| *p == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    /// Map a game phase to the corresponding fixed preset.
    pub(crate) fn for_phase(phase: crate::engine::ai::Phase) -> Self {
        match phase {
            crate::engine::ai::Phase::Early => ShepardPreset::Drift,
            crate::engine::ai::Phase::Mid => ShepardPreset::Ambient,
            crate::engine::ai::Phase::Late => ShepardPreset::Default,
            crate::engine::ai::Phase::Endgame => ShepardPreset::Dread,
        }
    }

    pub(crate) fn is_auto(self) -> bool {
        matches!(self, ShepardPreset::Auto)
    }

    fn params(self) -> ShepardParams {
        match self {
            ShepardPreset::Auto | ShepardPreset::Drift => ShepardParams {
                base_freq_hz: 180.0,
                octaves_per_second: 0.015,
                sigma: 2.8,
                output_gain: 0.25,
                sine_blend: 1.0,
                pulse_blend: 0.0,
                bitcrush_levels: 512.0,
            },
            ShepardPreset::Ambient => ShepardParams {
                base_freq_hz: 140.0,
                octaves_per_second: 0.025,
                sigma: 2.4,
                output_gain: 0.35,
                sine_blend: 0.97,
                pulse_blend: 0.03,
                bitcrush_levels: 256.0,
            },
            ShepardPreset::Default => ShepardParams {
                base_freq_hz: 110.0,
                octaves_per_second: 0.05,
                sigma: 1.8,
                output_gain: 0.45,
                sine_blend: 0.82,
                pulse_blend: 0.18,
                bitcrush_levels: 48.0,
            },
            ShepardPreset::Dread => ShepardParams {
                base_freq_hz: 65.0,
                octaves_per_second: 0.12,
                sigma: 1.4,
                output_gain: 0.50,
                sine_blend: 0.60,
                pulse_blend: 0.40,
                bitcrush_levels: 24.0,
            },
        }
    }
}

impl ShepardTone {
    pub(crate) fn load_default() -> Option<Self> {
        if !env_enabled(SHEPARD_TONE_ENV, true) {
            return None;
        }
        Some(Self {
            stream: None,
            sink: None,
            preset: ShepardPreset::Auto,
            active: ShepardPreset::Drift,
            volume: SHEPARD_TONE_VOLUME,
        })
    }

    pub(crate) fn start(&mut self) {
        if self.sink.is_some() {
            return;
        }

        let Ok(mut stream) = OutputStreamBuilder::open_default_stream() else {
            return;
        };
        stream.log_on_drop(false);

        let sink = Sink::connect_new(stream.mixer());
        sink.set_volume(self.volume);
        sink.append(ShepardSource::new(44_100, self.active.params()));
        sink.play();

        self.stream = Some(stream);
        self.sink = Some(sink);
    }

    pub(crate) fn set_volume(&mut self, vol: f32) {
        self.volume = vol;
        if let Some(sink) = &self.sink {
            sink.set_volume(vol);
        }
    }

    /// Set the user-chosen preset. If a fixed preset, immediately apply it.
    pub(crate) fn set_preset(&mut self, preset: ShepardPreset) {
        if self.preset == preset {
            return;
        }
        self.preset = preset;
        if !preset.is_auto() {
            self.apply_active(preset);
        }
    }

    /// Called by auto-progression: update active preset based on game phase.
    /// Only takes effect when user is in Auto mode.
    pub(crate) fn set_phase(&mut self, phase: crate::engine::ai::Phase) {
        if !self.preset.is_auto() {
            return;
        }
        let target = ShepardPreset::for_phase(phase);
        self.apply_active(target);
    }

    pub(crate) fn preset(&self) -> ShepardPreset {
        self.preset
    }

    pub(crate) fn active_label(&self) -> &'static str {
        if self.preset.is_auto() {
            self.active.label()
        } else {
            self.preset.label()
        }
    }

    fn apply_active(&mut self, target: ShepardPreset) {
        if self.active == target {
            return;
        }
        self.active = target;
        if self.sink.is_some() {
            self.stop();
            self.start();
        }
    }

    pub(crate) fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.stream = None;
    }
}

impl Drop for ShepardTone {
    fn drop(&mut self) {
        self.stop();
    }
}

struct ShepardParams {
    base_freq_hz: f32,
    octaves_per_second: f32,
    sigma: f32,
    output_gain: f32,
    sine_blend: f32,
    pulse_blend: f32,
    bitcrush_levels: f32,
}

struct ShepardSource {
    sample_rate: u32,
    sample_index: u64,
    phases: [f32; 7],
    params: ShepardParams,
}

impl ShepardSource {
    fn new(sample_rate: u32, params: ShepardParams) -> Self {
        // Spread initial phases to prevent phase-alignment throb.
        let mut phases = [0.0_f32; 7];
        for (i, p) in phases.iter_mut().enumerate() {
            *p = i as f32 / 7.0;
        }
        Self {
            sample_rate,
            sample_index: 0,
            phases,
            params,
        }
    }
}

impl Iterator for ShepardSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        const PARTIAL_CENTER: i32 = 3;

        let p = &self.params;
        let t = self.sample_index as f32 / self.sample_rate as f32;
        let progress = (t * p.octaves_per_second).fract();

        let mut sum = 0.0;
        let mut amp_sum = 0.0;
        for (i, phase) in self.phases.iter_mut().enumerate() {
            let octave = i as i32 - PARTIAL_CENTER;
            let pos = octave as f32 + progress;
            let freq = p.base_freq_hz * 2.0_f32.powf(pos);
            let amp = (-0.5 * (pos / p.sigma).powi(2)).exp();

            *phase = (*phase + freq / self.sample_rate as f32).fract();
            let sine = (std::f32::consts::TAU * *phase).sin();
            let pulse = if *phase < 0.5 { 1.0 } else { -1.0 };
            let wave = sine * p.sine_blend + pulse * p.pulse_blend;
            sum += wave * amp;
            amp_sum += amp;
        }

        self.sample_index = self.sample_index.wrapping_add(1);

        if amp_sum <= f32::EPSILON {
            return Some(0.0);
        }
        let raw = (sum / amp_sum * p.output_gain).clamp(-1.0, 1.0);
        Some((raw * p.bitcrush_levels).round() / p.bitcrush_levels)
    }
}

impl Source for ShepardSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> rodio::ChannelCount {
        1
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

fn env_enabled(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(v) => {
            let val = v.trim().to_ascii_lowercase();
            !matches!(val.as_str(), "0" | "false" | "off" | "no")
        }
        Err(_) => default,
    }
}

fn resolve_audio_path(env_key: &str, defaults: &[&str]) -> Option<PathBuf> {
    if let Ok(raw) = std::env::var(env_key) {
        let path = normalize_path(&raw);
        if path.is_file() {
            return Some(path);
        }
    }

    for candidate in defaults {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(path);
        }
    }

    None
}

fn normalize_path(raw: &str) -> PathBuf {
    if let Some(stripped) = raw.strip_prefix("file://") {
        PathBuf::from(stripped)
    } else {
        PathBuf::from(raw)
    }
}

#[cfg(target_os = "macos")]
fn play_mp3(path: &Path) {
    // Detached process so UI input remains responsive.
    let _ = Command::new("afplay").arg(path).spawn();
}

#[cfg(not(target_os = "macos"))]
fn play_mp3(_path: &Path) {}
