use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::engine::ai::Phase;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker {
    stock_line: String,
    headline_1: String,
    headline_2: String,
    text: String,
    offset: usize,
}

impl Default for Ticker {
    fn default() -> Self {
        Self::new()
    }
}

impl Ticker {
    pub fn new() -> Self {
        Self {
            stock_line: String::new(),
            headline_1: String::new(),
            headline_2: String::new(),
            text: String::new(),
            offset: 0,
        }
    }

    pub fn stock_line(&self) -> &str {
        &self.stock_line
    }

    pub fn headlines(&self) -> (&str, &str) {
        (&self.headline_1, &self.headline_2)
    }

    pub fn update_for_turn<R: Rng>(
        &mut self,
        rng: &mut R,
        phase: Phase,
        ai_x10: u32,
        headlines: &[&str],
    ) {
        let ai_pct = ai_x10 as f32 / 10.0;

        // Very fake, but visually plausible: prices rise with AI capability.
        let nvda = (500.0 + ai_pct * 7.5) as i64;
        let msft = (350.0 + ai_pct * 1.6) as i64;
        let tsmc = (120.0 + ai_pct * 0.9) as i64;

        // Small random "daily" moves. More volatility later.
        let vol: f32 = match phase {
            Phase::Early => 3.0,
            Phase::Mid => 5.0,
            Phase::Late => 7.0,
            Phase::Endgame => 9.0,
        };
        let p = |rng: &mut R| (rng.gen_range(-vol..=vol) * 10.0_f32).round() / 10.0_f32;

        let stock_line = format!(
            "NVDA ${} ({:+.1}%) | MSFT ${} ({:+.1}%) | TSMC ${} ({:+.1}%)",
            nvda,
            p(rng),
            msft,
            p(rng),
            tsmc,
            p(rng)
        );

        let clean: Vec<String> = headlines
            .iter()
            .map(|h| sanitize_headline(h))
            .filter(|h| !h.is_empty())
            .collect();

        self.stock_line = stock_line.clone();
        self.headline_1 = clean.first().cloned().unwrap_or_default();
        self.headline_2 = clean.get(1).cloned().unwrap_or_default();

        let headline_part = clean.join(" | ");
        self.text = if headline_part.is_empty() {
            format!("{} | ", stock_line)
        } else {
            format!("{} | {} | ", stock_line, headline_part)
        };
        self.offset = self.offset.min(self.text.len());
    }

    pub fn tick(&mut self) {
        if self.text.is_empty() {
            return;
        }
        self.offset = (self.offset + 1) % self.text.len();
    }

    pub fn view(&self, width: usize) -> String {
        if width == 0 {
            return String::new();
        }
        if self.text.is_empty() {
            return " ".repeat(width);
        }

        // ASCII-only ticker text => byte indexing is safe.
        let mut out = String::with_capacity(width);
        for i in 0..width {
            let idx = (self.offset + i) % self.text.len();
            out.push(self.text.as_bytes()[idx] as char);
        }
        out
    }
}

fn sanitize_headline(raw: &str) -> String {
    raw.trim().trim_end_matches('.').trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn ticker_view_is_fixed_width() {
        let mut rng = StdRng::seed_from_u64(123);
        let mut t = Ticker::new();
        t.update_for_turn(&mut rng, Phase::Early, 100, &["Test headline goes here."]);
        let v = t.view(40);
        assert_eq!(v.len(), 40);
    }

    #[test]
    fn ticker_wrap_has_separator_and_no_trailing_full_stop() {
        let mut rng = StdRng::seed_from_u64(123);
        let mut t = Ticker::new();
        t.update_for_turn(&mut rng, Phase::Early, 100, &["News cycle shifts."]);
        assert_eq!(t.headline_1, "News cycle shifts");
        assert!(t.text.contains(" | News cycle shifts | "));
        assert!(t.text.ends_with(" | "));
    }
}
