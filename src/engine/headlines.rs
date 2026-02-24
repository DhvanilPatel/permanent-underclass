use std::sync::OnceLock;

use anyhow::{Context, Result};
use rand::Rng;
use serde::Deserialize;

use crate::engine::ai::Phase;
use crate::engine::content::PhaseGate;
use crate::engine::types::ResourceDelta;

const HEADLINES_JSONL: &str = include_str!("../../data/headlines.jsonl");

#[derive(Debug, Clone, Deserialize)]
pub struct HeadlineDef {
    pub phase: PhaseGate,
    pub text: String,
    #[serde(default)]
    pub delta: ResourceDelta,
}

#[derive(Debug)]
pub struct Headlines {
    pub list: Vec<HeadlineDef>,
}

static EMBEDDED: OnceLock<Headlines> = OnceLock::new();

impl Headlines {
    pub fn embedded() -> Result<&'static Headlines> {
        if let Some(h) = EMBEDDED.get() {
            return Ok(h);
        }
        let parsed = Self::from_embedded()?;
        let _ = EMBEDDED.set(parsed);
        Ok(EMBEDDED.get().expect("embedded headlines initialized"))
    }

    /// Pick `n` distinct random headline texts for the given phase.
    pub fn pick_random<R: Rng>(phase: Phase, n: usize, rng: &mut R) -> Vec<String> {
        let Ok(headlines) = Self::embedded() else {
            return Vec::new();
        };
        let pool: Vec<&str> = headlines
            .list
            .iter()
            .filter(|h| h.phase.matches(phase))
            .map(|h| h.text.as_str())
            .collect();
        if pool.is_empty() {
            return Vec::new();
        }
        // Shuffle indices and take up to n distinct headlines.
        let mut indices: Vec<usize> = (0..pool.len()).collect();
        for i in (1..indices.len()).rev() {
            let j = rng.gen_range(0..=i);
            indices.swap(i, j);
        }
        indices
            .into_iter()
            .take(n)
            .map(|i| pool[i].to_string())
            .collect()
    }

    fn from_embedded() -> Result<Headlines> {
        let mut list = Vec::new();
        for (idx, raw) in HEADLINES_JSONL.lines().enumerate() {
            let line_no = idx + 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let h: HeadlineDef = serde_json::from_str(line).with_context(|| {
                format!(
                    "Failed to parse embedded JSONL: data/headlines.jsonl:{}",
                    line_no
                )
            })?;
            list.push(h);
        }
        Ok(Headlines { list })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::content::PhaseGate;

    #[test]
    fn embedded_headlines_parse() {
        let h = Headlines::embedded().unwrap();
        assert!(!h.list.is_empty());
    }

    #[test]
    fn headlines_cover_all_phases() {
        let h = Headlines::embedded().unwrap();
        for phase in [
            PhaseGate::Early,
            PhaseGate::Mid,
            PhaseGate::Late,
            PhaseGate::Endgame,
        ] {
            assert!(
                h.list.iter().any(|x| x.phase == phase),
                "missing headlines for phase {:?}",
                phase
            );
        }
    }
}
