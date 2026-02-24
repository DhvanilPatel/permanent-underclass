# EVENT WRITING GUIDE

Principles, craft, and constraints for writing character event catalogs.

**Scope:** Applies to all 15 character catalogs in `data/events/*.jsonl`.

---

## 1. The Meta-Principle

Acceleration should be felt in three ways simultaneously:

1. **In the narrative** — events describe faster and faster change
2. **In the structure** — Early pool is compressed, events stack, choices become more urgent
3. **In the writing itself** — Early events are leisurely and descriptive; Endgame events are terse, breathless, clipped

The prose style should accelerate alongside the content. Early reads like a New Yorker essay. Endgame reads like an emergency dispatch.

---

## 2. Voice & Tone

### The narrator is the character's inner monologue, not an omniscient observer

Write in second person ("you") but from inside the character's head. The narrator knows what the character knows, uses the vocabulary the character would use, and shares the character's blind spots. A junior dev's inner monologue shouldn't sound like a CEO's.

### Gallows humor is the baseline

Every event should be at least a little funny, even the devastating ones. The humor comes from self-awareness — the character seeing the absurdity of their situation while being unable to escape it. Never punch down. The character is always the butt of their own joke.

**Good:** "Your body is a bag you're holding at a loss."
**Bad:** "You're pathetic and everyone knows it."

The difference: the first is the character's own metaphor framework applied to themselves. The second is external judgment.

### Maintain the comedic register even in serious moments

The Late and Endgame phases will tempt you toward earnestness. Resist. The funniest and most affecting events are the ones where the humor and the gravity coexist. "The philosopher nods. The quant smirks." is better than a paragraph of existential reflection.

### One killer line per event

Every event body should have one line that could be screenshotted and shared. This is the hook. It usually comes at the end of the body text or lands as an unexpected turn.

**Examples:**
- "The candle is red. The candle is always red."
- "Decentralized governance is just meetings with worse UX."
- "Your trading bot just asked you about ethics. The future is here and it has questions."

---

## 3. Event Structure

### Body text: setup → situation → twist

The body text should be 3-6 sentences. Structure:

1. **Setup** (1-2 sentences): Establish the situation and the character's emotional state.
2. **Situation** (1-2 sentences): The specific thing that's happening.
3. **Twist** (1-2 sentences): The uncomfortable truth, the irony, the punchline, or the knife.

Don't overwrite. The body should create a decision space, not resolve it. Leave room for the choices to do work.

### Choices: the three-way tension

Every event presents three choices structured as a genuine tension:

- **Choice A:** The pragmatic/sellout option. Best material outcome, worst for soul/relationships.
- **Choice B:** The principled/costly option. Best for integrity, worst for material position.
- **Choice C:** The middle ground/cope/lateral move. Avoids the binary but has its own costs.

**Critical: no obviously correct choice.** If one choice is clearly better than the others on all dimensions, the event fails as gameplay. The player should feel the trade-off physically.

### Choice body text: consequence, not narration

Each choice body should be 1-2 sentences that describe the *immediate consequence* of the action, not a continuation of the narrative. Use the same sardonic voice.

**Good:** "One tweet. Plausible deniability. The money hits your account before the guilt hits your conscience."
**Bad:** "You decide to take the money and post the tweet, feeling conflicted about it afterward."

The first is consequence with voice. The second is narration without perspective.

### Deltas must match the narrative

If a choice says "nobody will know," audience and friends shouldn't drop. If a choice says "you spend the weekend alone," friends shouldn't increase. The numbers are the story's truth — they must be consistent with what the text describes.

---

## 4. Phase-by-Phase Guide

Turn ranges vary by tempo:
- `standard` (32 turns): Early `1-8`, Mid `9-16`, Late `17-24`, Endgame `25-32`
- `simple` (12 turns): Early `1-3`, Mid `4-6`, Late `7-9`, Endgame `10-12`

### Early (`standard` turns 1–8; `simple` turns 1–3): "You still think you understand the game"

**Pool size:** 20–25 events. Minimum 20 to satisfy sim diversity constraints (≥2.5x turns).

**AI mention frequency:** ~20–30% of events mention AI directly; the rest have AI as ambient background detail.

**Key principles:**
- AI is already here. It's just not the main event yet. It's ambient pressure — background noise the player notices but the character doesn't.
- Cut generic "life is hard" events that have no AI presence. The game is about acceleration.
- Every Early event should contain at least one AI signal, even if it's a throwaway detail:
  - A founder mentions building with Claude *in a parenthetical*
  - Someone at dinner references an autonomous fund *and nobody reacts*
  - A portfolio company *casually* replaced a role with AI
  - A solo founder built everything with AI *and is pitch #27 that nobody remembers*
- These seeds create **dramatic irony** — the player can see what's coming even when the character can't.

**Prose style:** Leisurely. Full sentences. Metaphors. Time for description. The world is still moving at a pace you can narrate.

**Delta patterns:**
- Money: mostly small or zero; occasional big check (-$25K to -$50K) or lucky exit (+$75K)
- Sanity: slightly negative average (-1.1) — Early should feel mildly stressful, not crushing
- Audience/AI Skill/Friends: small movements (+6 to +8 range), averaging slightly positive
- Avoid giving ai_skill in Early except for explicit AI-engagement choices

### Mid (`standard` turns 9–16; `simple` turns 4–6): "The ground is shifting"

**Pool size:** 25–40 events. Minimum 20 to satisfy sim diversity.

**AI mention frequency:** ~50–60% of events.

**Key principles:**
- **Stack disruptions.** Don't sequence them one-at-a-time. Multiple things should happen in the same event. "Before 9am, five things broke."
- **Mid-event escalation.** The situation should change *while the player is reading*. A founder pivots mid-pitch. A product gets a competitor during a demo. The world moves faster than your decision-making.
- **Time-compression language.** Reference how recently the last disruption happened and how much faster the next one will be. "Three months ago, building took a weekend. Now it takes an afternoon. Next month, an afternoon will be geological."
- **Events where deliberation becomes irrelevant.** You spend a week on diligence; by Friday the company has pivoted twice. Your diligence document is a historical artifact.

**Prose style:** Standard. Full sentences but tighter. Less room for metaphors. The pace is picking up.

**Delta patterns:**
- Money: more negative average (-$1,350) — the financial pressure is real
- Sanity: most negative phase (-1.9 avg) — this is where the existential grind peaks
- AI Skill: starts appearing more frequently as characters engage/resist AI
- Key flags set here: `invested_in_ai`, `embraced_ai`, `rejected_ai`, `started_newsletter`, `went_viral`

### Late (`standard` turns 17–24; `simple` turns 7–9): "The acceleration is visible"

**Pool size:** 20–35 events. Minimum 20 to satisfy sim diversity.

**AI mention frequency:** ~80–90% of events.

**Key principles:**
- **Time compression events.** Two weeks off = a decade of change. Monthly advice = obsolete by the 15th. Your cadence is geological in a world running at processor speed.
- **Structural obsolescence.** It's not just hard anymore — it's structurally over. The role itself is disappearing.
- Choices shift from "how to play the game" to "whether the game still exists."

**Prose style:** Tighter. More urgent. Shorter sentences. Less time for metaphor — the metaphors are being disrupted too.

**Delta patterns:**
- Money: slightly positive average (+$1,002) — some recovery events, book deals, exits
- Sanity: slightly positive average (+0.7) — acceptance starts arriving
- Friends: positive average (+1.8) — relationships become the currency
- AI Skill: only positive values (6–12) — by Late, there's no "reject AI" option that teaches you anything
- Key flags: `hit_rock_bottom`, `shipped_creative_work`, `became_mentor`, `took_sabbatical`

### Endgame (`standard` turns 25–32; `simple` turns 10–12): "Emergency dispatches"

**Pool size:** 15–30 events. Minimum 15 to satisfy sim diversity.

**AI mention frequency:** ~100% of events.

**Key principles:**
- **Terse. Breathless. Clipped.** Fragment sentences. No time for full paragraphs. The writing itself embodies the speed.
- **Extreme time compression.** Companies live and die in minutes. A full lifecycle — launch to acquisition to obsolescence — in a week.
- **Identity, not strategy.** The choices aren't about the character's job anymore. They're about who you are when the thing that defined you is gone.
- The player should feel like they have *less time to react*. The events should feel faster to read.

**Prose style:**
- BAD: "A teenager with Claude builds a $50M-revenue company in six weeks. Total cost: $200 in API credits and a laptop her parents bought for college."
- GOOD: "Teenager. Claude. Six weeks. $50M revenue. Cost: $200."

**Delta patterns:**
- Sanity: most positive phase (+1.9 avg) — acceptance and peace arrive
- Friends: highest phase (+3.9 avg) — human connection is the endgame return
- AI Skill: still only positive — no going back
- Money: modest positive average — small income from pivoted careers
- `found_balance` is the dominant flag (10 instances) — the resolution stat
- Total non-money delta per choice is highest here (+9.8 avg) — late-game choices should matter more

### Any-phase events

**Pool size:** 8–20 events. Universal (`Any`) events supplement all phases.

These are wildcards and palette cleansers — random encounters that could happen in any phase. They should still contain AI signals appropriate to the game's premise.

---

## 5. Acceleration Design

This is the most important section. The game is about AI acceleration. Every event file should *feel* like acceleration, not just describe it.

### Named Event Patterns

These patterns should exist in every character catalog:

**1. The Dramatic Irony Seed (Early)**
A small detail the character ignores but the player notices. Someone mentions something that will become the entire Late-game crisis — in a parenthetical, in a footnote, at pitch #27 that nobody remembers.

**2. The Stacked Disruption (Mid)**
Multiple bad things happen in a single event. Not sequential — simultaneous. The inbox before 10am. The email stack. The Monday morning implosion.

**3. The Mid-Event Escalation (Mid)**
The situation changes between the setup and the choice. A founder pivots mid-sentence. A market disappears during a demo. The world moves while you're watching.

**4. The Deliberation That Became Irrelevant (Mid/Late)**
You spend time deciding. By the time you decide, the decision doesn't matter anymore. Your diligence took longer than the company's lifecycle.

**5. The Time Skip (Late)**
You step away briefly. When you return, the world has moved a decade. Two weeks off = total chaos. Monthly advice = already obsolete.

**6. The Speed Event (Endgame)**
Pure velocity. A company lives and dies in 4 minutes. An entire industry cycle in one week. The prose should be as fast as the event.

### Acceleration Principles

**Compress the "impossible → boring" gap.** Real acceleration isn't "things get more intense." It's "the interval between intensity spikes shrinks until the spikes merge into a wall." Write event pairs where the first presents something as mind-blowing and a later event in the *same phase* treats it as old news:
- Turn N: "An AI agent completes its first autonomous trade. You screenshot it. History."
- Turn N+3: "AI agents completed 4 million trades today. Nobody tweeted about it."

**Phase transitions should feel like gear shifts.** Add 1-2 events per phase boundary that feel qualitatively different — shorter body text, starker language, fewer cope options:
- "You wake up and three of the top accounts in your field are AI-generated. They didn't exist yesterday."

**Make earlier choices feel quaint.** The most powerful acceleration trick: the player makes a careful decision in Early, then a Late event makes *both options* irrelevant:
- "Remember when you agonized about whether to learn Python or use AI to code? The new framework doesn't use any human programming language. The entire debate had 18 months of relevance."

**Accelerate AI event frequency, not just intensity.** The player should feel AI *crowding out* other concerns. Not because those concerns disappear, but because AI absorbs them.

**Callback compression.** Write Late/Endgame events that directly reference Early events but with the timescale collapsed:
- Early: "You spend three hours analyzing a market trend..."
- Late: "Your AI analyzed 4,000 trends in the time it took you to read this event."

**Bend voice, not mechanics (use sparingly).** Use 2-3 events per character to *feel* structurally unusual while staying inside runtime constraints:
- Keep exactly 3 authored choices (plus optional freeform card). Do not author fake fourth-choice copy.
- If an option became obsolete, write it as a painful but selectable choice. Do not write fake disabled/greyed-out UI states in choice text.
- You can write "AI recommends X" in title/body, but the player must still make a real choice.
- You can shorten body text aggressively (e.g., "AI summarized this"), or switch narration voice for effect.

One per phase maximum. More than that and the trick stops working.

---

## 6. Character-Specific Design

### Every character needs a signature obsession

The crypto bro checks charts. The junior dev checks GitHub stars. The product guy checks Product Hunt. Each character should have a *specific repeated behavior* that appears across phases and gets progressively more absurd or irrelevant as AI advances.

### Every character needs a throughline ritual

The crypto bro's "gm" posts thread through the entire arc, from ritual to relic. Each character needs an equivalent — a recurring motif that starts meaningful, becomes contested, and eventually either transforms or dies.

### Map the character's AI journey

Before writing events, plot the character's relationship to AI across phases:

1. **Denial/Ignorance** (Early): AI is someone else's problem
2. **Encounter** (Early→Mid): First real confrontation with AI capability
3. **Negotiation** (Mid): Trying to integrate AI while preserving identity
4. **Crisis** (Mid→Late): AI threatens core identity/livelihood
5. **Reckoning** (Late): What am I if not this thing AI just replaced?
6. **Resolution** (Endgame): New relationship with AI, for better or worse

Not every character follows this exactly, but this arc should be the skeleton.

### The character's jargon should evolve

Early events should use the character's native vocabulary heavily. By Late, AI vocabulary should be infiltrating their speech. By Endgame, the two vocabularies should be merged or the character should be noticeably code-switching.

---

## 7. Avoiding Overlap

### The overlap test

Before writing an event, ask: "What emotional beat does this hit?" If you can name another event in the same phase that hits the same beat, one of them needs to go or be substantially differentiated.

Common overlap traps:
- Multiple "I'm down bad and coping" events in Early
- Multiple "AI is replacing my specific skill" events in Mid
- Multiple "explaining my career to normies" events across phases
- Multiple "ethical dilemma about monetizing audience" events in Mid
- Multiple "content creation burnout" events in Mid/Late

### The differentiation rule

Two events can share a *theme* if they present *different decision spaces*. "AI outperforms me at trading" and "AI outperforms me at writing" are the same theme but could be different events IF the choices lead to genuinely different strategic paths. If the choices are essentially "adapt / resist / cope" in both cases, merge them.

---

## 8. Delta Design

### Stat semantics

- **money:** The literal financial impact. Keep proportional to what the character would actually gain/lose.
- **sanity:** The psychological cost. Choices that align with the character's values should gain sanity. Choices that betray them should lose it. Avoiding decisions also costs sanity.
- **audience:** Public-facing social capital. Goes up with visibility, goes down with privacy or withdrawal.
- **ai_skill:** Competence with AI tools and thinking. Goes up with learning and engagement, goes down with avoidance or shallow usage.
- **friends:** Real human relationships. Almost always trades against audience. Building community costs clout and vice versa.

### Phase scaling

Authored deltas are multiplied before application:
- Early: `1.00x`
- Mid: `1.10x`
- Late: `1.25x`
- Endgame: `1.40x`

Late/Endgame authored values can be slightly smaller to achieve similar felt impact.

### Delta magnitude guide

| Stat | Small | Medium | Large |
|------|-------|--------|-------|
| Money | ±$150–$900 | ±$2K–$10K | ±$15K–$50K |
| Sanity | ±6 | ±8 | ±12–14 |
| Audience | ±6 | ±8 | ±12–14 |
| AI Skill | ±6 | ±8 | ±12 |
| Friends | ±6 | ±8 | ±12–14 |

### Balancing checklist

- [ ] **Each event has 3 choices** with meaningfully different delta profiles
- [ ] **No choice is positive on all five stats.** If it is, it's not a real choice.
- [ ] **Sanity is the pressure stat:** slightly negative in Early/Mid, neutral to positive in Late/Endgame
- [ ] **Friends increases in later phases:** human connection becomes the endgame currency
- [ ] **AI Skill is only positive in Late/Endgame:** by then, rejecting AI teaches nothing
- [ ] **Money choices in Early use `min_money` gates** to create real tension for cash-poor players
- [ ] **Flag effects are phase-appropriate:** `invested_in_ai` in Early/Mid, `hit_rock_bottom` in Late, `found_balance` in Endgame

---

## 9. Flag & Gating Design

### Flags should create narrative consequences, not just unlock content

A flag like `embraced_ai` shouldn't just gate events — it should change the *emotional texture* of events that fire after it's set. An event about content creation burnout hits differently if the player previously embraced AI-generated content vs. if they rejected it.

### Gate mid-to-late events on meaningful early choices

The best gated events are the ones where the player sees the *consequence* of a choice they made 20 turns ago. "Your open-source project got forked" only fires if you chose to open-source — and that choice was made in a completely different context.

### Use `once_per_run` for irreversible life events

Breakups, health scares, major financial windfalls, job offers — things that would change the character's baseline state. Don't fire these more than once.

### Flag names should be human-readable states, not actions

**Good:** `found_balance`, `burned_bridge`, `embraced_ai`
**Bad:** `chose_option_2`, `completed_event_45`

Flags are narrative states the character inhabits, not event tracking variables.

---

## 10. Diversity Constraints

The sim test (`cargo test --test sim`) enforces:
- **Max 46% per-event run share** (40% + 6% tolerance): no single event can appear in more than 46% of 100 random runs
- **Pool coverage minimum**: enough events must exist per phase to cover a meaningful portion across runs

**Rule of thumb:** pool size should be at least 2.5x the number of turns in that phase.

For `standard` (32 turns):
- Early (8 turns): need >=20 events
- Mid (8 turns): need >=20 events
- Late (8 turns): need >=20 events
- Endgame (8 turns): need >=20 events

For `simple` (12 turns):
- Early (3 turns): need >=8 events
- Mid (3 turns): need >=8 events
- Late (3 turns): need >=8 events
- Endgame (3 turns): need >=8 events

Universal (`Any`) events supplement all phases.

---

## 11. Quality Checklist

Before submitting an event, verify:

- [ ] **Voice consistent?** Does this sound like this character's inner monologue?
- [ ] **One killer line?** Is there a line worth screenshotting?
- [ ] **Genuine three-way tension?** Would a player actually agonize over this?
- [ ] **No dominant choice?** Is any option clearly best on all dimensions?
- [ ] **Not overlapping?** Does another event in this phase hit the same emotional beat?
- [ ] **Phase-appropriate AI intensity?** Does the AI mention frequency match the phase?
- [ ] **Deltas balanced?** No choice is positive on all five stats?
- [ ] **Deltas match narrative?** If nobody knows, reputation doesn't drop.
- [ ] **Body text tight?** 3-6 sentences, setup → situation → twist?
- [ ] **Choice consequences, not narration?** 1-2 sentence consequences with voice?
- [ ] **Acceleration present?** Does this event contribute to the feeling that things are speeding up?

---

## 12. Anti-Patterns

### Don't write therapy sessions

Events should present situations, not process emotions. The character shouldn't spend the body text reflecting on their feelings — that's what the choices are for. "Your therapist says the word 'addiction'" is a situation. Three paragraphs about what addiction means to the character is a therapy session.

### Don't lecture the player about AI

The game's thesis should emerge from lived experience, not exposition. "AI is changing everything" is a lecture. "The leaderboard is all bots and the last human just quit" is an experience.

### Don't make AI the villain or the savior

AI should be a force, like weather. It doesn't have intentions toward the character. It just is, and the character has to navigate it. The most unsettling AI events are the ones where AI is simply *better* and doesn't care.

### Don't resolve tension in the body text

The body text creates the tension. The choices navigate it. If the body text already tells the player how to feel about the situation, the choices become mechanical rather than emotional.

### Don't use "you realize" or "you understand"

These phrases short-circuit the player's own realization. Show the situation; let the player realize things on their own. "An AI wrote a better thread than you in 30 seconds" is better than "You realize AI can now write better than you."

---

## Appendix: Case Study — `angel_investor.jsonl`

Applied Feb 2026 as the first systematic rewrite.

### Structural changes
- **Early:** 30 → 20 events (cut 17 generic, added 7 with AI seeds)
- **Mid:** 35 → 38 events (added 3: stacked disruptions, mid-event escalation)
- **Late:** 30 → 32 events (added 2: time compression)
- **Endgame:** 25 → 27 events (added 2: speed/breathless events)
- **Total:** 140 → 137 events

### Narrative changes
- All 20 Early events now have AI as ambient background
- 3 Mid events demonstrate stacked disruptions and mid-event escalation
- 2 Late events demonstrate time compression
- ~11 Endgame bodies rewritten to be terse/clipped (fragment sentences, no metaphors)

### Still needed
- Systematic delta rebalancing of new events against phase averages
- Verification that flag dependencies still chain correctly after cuts
- Tonal pass on Late events (still reads similar to Mid in many cases)
- Application to remaining 14 character catalogs
