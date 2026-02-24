# Sprite Library Expansion

Generate new ASCII sprites and add them to the Permanent Underclass sprite library.

## Arguments
- $ARGUMENTS: Optional theme/category to focus on (e.g. "food", "tech", "emotions"). If empty, pick a mix of relevant themes.

## Instructions

You are expanding the sprite library at `docs/sprite_library.html` for the game "Permanent Underclass" (Bloomberg Terminal × Oregon Trail × Cogmind).

### Step 1: Read existing sprite names
Read `docs/sprite_library.html` and extract all existing sprite names to avoid duplicates. Use grep or python to get a list of all `<div class="name">...</div>` values.

### Step 2: Generate new sprites
Generate 50-100 new unique sprites. Each sprite must follow this exact format:

**Sprite spec:** 5 lines × 10 characters, using:
- Box-drawing: `╔╗╚╝║═╠╣╦╩┌┐└┘│─├┤┬┴`
- Fills: `░▒▓█`
- Diagonals: `╱╲`
- Text characters
- Spaces for padding (NEVER dots)

**12 color classes** (single-letter CSS classes):
| Class | Color | Hex | Use |
|-------|-------|-----|-----|
| `g` | green | #00ff41 | success, money, nature |
| `a` | amber | #ffb000 | warnings, gold, warm |
| `r` | red | #ff4444 | danger, errors, urgent |
| `b` | blue | #4488ff | info, tech, links |
| `p` | purple | #bb88ff | AI, creative, premium |
| `c` | cyan | #00ddff | data, network, cool |
| `y` | yellow | #ffdd00 | highlights, stars |
| `k` | pink | #ff66aa | social, love, fun |
| `o` | orange | #ff8844 | energy, activity |
| `d` | dim | #334455 | borders, backgrounds |
| `m` | medium | #556677 | secondary elements |
| `w` | white | #ccddee | text, highlights |

**HTML format per sprite:**
```html
<div class="card"><pre class="sprite"><span class="d">╔════════╗</span>
<span class="d">║</span><span class="r">▓▓▓▓▓▓▓▓</span><span class="d">║</span>
<span class="d">║</span><span class="r">▓</span><span class="w">LABEL</span><span class="r">▓▓</span><span class="d">║</span>
<span class="d">║</span><span class="r">▓▓▓▓▓▓▓▓</span><span class="d">║</span>
<span class="d">╚════════╝</span></pre>
<div class="name">Sprite Name</div></div>
```

Mix multiple color spans per line for visual interest. Use `d`/`m` for borders and backgrounds. Use bright colors for content.

If `$ARGUMENTS` specifies a theme, focus sprites on that theme. Otherwise pick from: tech, work, people, lifestyle, emotions, economy, social media, politics, health, nature, places, gaming, abstract concepts.

### Step 3: Add to HTML file
Determine which category section the new sprites belong to. If the category already exists in the file, append within that section's `<div class="grid">`. If it's a new category, add a new section before the footer:

```html
<!-- ════════════════════════════════════════════════ -->
<div class="category"><div class="category-title">Category Name</div>
<div class="grid">

...cards here...

</div></div>
```

### Step 4: Update the footer count
Update the sprite count in the footer line: `PERMANENT UNDERCLASS SPRITE LIBRARY v3 — 5x10 STANDARD — XXX SPRITES`

### Step 5: Verify
Count the total sprites in the final file to confirm the count is correct. Report what was added.

### Quality guidelines
- Each sprite should be visually distinct and recognizable
- Use dense, colorful fills rather than sparse outlines
- Names should be concise (1-3 words)
- Relevant to the Permanent Underclass theme: surviving as a knowledge worker 2025-2027
- Avoid emoji characters (they break monospace alignment)
- Make sure every line is exactly 10 characters wide (count carefully)
