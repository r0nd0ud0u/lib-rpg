# lib-rpg

A Rust game-engine library for turn-based RPG combat, used by [dx-rpg](https://github.com/r0nd0ud0u/dx-rpg).

---

## Overview

`lib-rpg` provides all game logic independently of any UI framework:

- **Character system** — stats, effects, energy (HP / Mana / Vigor / Berserk), inventory
- **Combat engine** — turn resolution, damage / heal / buff / debuff, critical hits, dodge, block
- **Cooldown system** — per-attack cooldown tracked by `buffer.value` (turns remaining)
- **Boss AI** — attack pattern sequencing, multiple bosses per scenario
- **Scenario progression** — sequential scenarios, accumulated kill counter across scenarios
- **Save / load** — full game state serialised to JSON for persistence
- **Data manager** — loads characters, attacks, scenarios and equipment from offline JSON files

---

## Key Concepts

### Cooldown

Cooldowns are tracked via `BufKinds::CooldownTurnsNumber` effects on the character's active effects list.

`buffer.value` is the **single source of truth** for the cooldown duration (number of turns an attack is unavailable).  `nb_turns` on the same `EffectParam` must equal `buffer.value` for consistency.

The check in `can_be_launched` uses `buffer.value - counter_turn > 0`.  All log messages also use `buffer.value` so the displayed number matches the actual lock duration.

### Kill Counter

`GameState.accumulated_kills` tracks the total number of bosses killed across all scenarios in a session.  It is **never reset** when calling `clear_scenario()`.  Before each scenario transition (`load_next_scenario`), the count of dead bosses from `active_bosses` is added to `accumulated_kills`.  This means consumers can always compute the true kill total as:

```rust
game_state.accumulated_kills + pm.active_bosses.iter().filter(|b| b.stats.is_dead().unwrap_or(false)).count()
```

### `DecreasingRateOnTurn` HOT

An effect with `buffer.kind = DecreasingRateOnTurn` on HP applies a **probabilistic healing-over-time** (HOT):

- **Launch turn** — `process_decrease_on_turn` rolls 1–`value` *applies* (first roll is always 100 %; each subsequent roll decreases linearly).  `full_amount = applies × (buffer.value + magical_power / nb_turns)`.
- **Subsequent ticks** — for each turn `counter_turn ∈ [1, value]`, the tick fires with probability `(value − counter_turn + 1) / value`:
  - counter 1 → 100 %
  - counter 2 → 67 % (for value = 3)
  - counter 3 → 33 % (for value = 3)
- Ticks with `counter_turn > value` never fire; the effect still expires normally at `counter_turn == nb_turns`.

This means the HOT fires **at most** `value` ticks after launch, not always `value` times.

### `loaded_from_save`

`CoreGameData.loaded_from_save` is `false` for fresh games and `true` when a game is restored from a save file.  UI layers use this flag to lock the universe selector once a save has been loaded.

### Passive Powers

A passive power is a `Buffer` entry in a character's `Buf-debuf` list (`CharacterRoundsInfo.all_buffers`) with `"passive": true` and `"passive-enabled": true`.  Unlike attack-triggered effects, passives are defined statically in the character JSON and fire automatically at the start of each turn inside `Character::new_round`.

#### `OverHealBoostStat` (overheal → stat boost)

`BufKinds::OverHealBoostStat` — at the start of each turn, reads the overheal amount recorded for the **previous turn** in `tx_rx[AmountType::OverHealRx]` and adds it to the stat named in `buffer.stats_name`.  The boost bypasses the stat's max cap (physical power can exceed its base max).

`tx_rx[AmountType::OverHealRx]` is populated by two paths:
- **HOT ticks** — `apply_hot_or_dot` writes any HP excess when HOTs push HP past max.
- **Regular heal attacks** — `apply_processed_effect_param` accumulates any HP excess when a direct heal overflows max HP.

This same buffer kind is also enabled dynamically by the `AddAsMuchAsHp` attack effect, so it serves both as a static character passive and as an attack-triggered passive.

**Azrak Ombresang** carries this passive with `stats_name = "Physical power"`: each point of overheal he received on the previous turn is converted into a bonus to his current Physical power, rewarding sustained healing beyond his HP cap.

JSON definition (in `CharacterRoundsInfo.Buf-debuf`):

```json
{
  "stats-name": "Physical power",
  "is-percent": false,
  "passive-enabled": true,
  "passive": true,
  "kind": "OverHealBoostStat",
  "value": 0
}
```

---

## Data Files

All game data lives under `offlines/` as JSON:

| Path | Contents |
|------|----------|
| `offlines/characters/<universe>/` | Hero and boss character definitions |
| `offlines/attack/<character-name>/` | Attack / skill JSON files per character |
| `offlines/equipment/` | Equipment items |
| `offlines/scenarios/<universe>/` | Scenario stage definitions |

Scenarios are filtered by universe at game initialisation and when the universe is changed before a game starts.

---

## Damage Formula

### `AttackType::damage_by_atk`

Returns `(raw_damage, effective_damage)`:

```
raw_damage     = atk_value − (launcher_power / nb_turns)
effective      = round(raw_damage × ARMOR_FACTOR / (ARMOR_FACTOR + target_armor))
```

- `ARMOR_FACTOR = 100.0` — armor equal to this value halves incoming damage
- Both values are **negative** for damage, **positive** for healing
- `raw_damage` is logged as "full" damage (before armor); `effective_damage` is applied to HP

**Armor scaling:** at ARMOR_FACTOR = 100 and hero armor in the 0–90 range, a 50 % armor buff gives ~9–10 % less damage taken (vs ~2 % with the former constant of 1000).

**Boss armor** is scaled to preserve hero–boss balance at the new constant (e.g. Angmar 800 → 80 still absorbs ~44 % of hero attacks).

### Combat log

HP damage effects are logged as:
- `"{target} ← {real} HP"` when no mitigation occurred
- `"{target} ← {real} HP (full: {pre_armor}, real: {real})"` when armor, blocking, or HP cap reduced the raw hit

---

## Building & Testing

```bash
cargo fmt
cargo clippy --all-targets
cargo test
```

All 251 tests should pass with no warnings.

---

## Bug Fixes

### RemoveOneDebuf — debuff not removed from target (e.g. Éveil de l'Espérance)

**Root cause:** `process_effect_type` for `RemoveOneDebuf` operated on the **launcher's** `all_effects`, not the target's. Additionally, `apply_processed_effect_param` returned early (empty `stats_name` guard) before ever touching the target's effect list, so no debuff was ever removed from the character receiving the heal.

**Fix:** `process_effect_type` is now a no-op for `RemoveOneDebuf` on the launcher side. The actual removal runs in `apply_processed_effect_param` on the target character, which removes the oldest debuff from the target's `all_effects`. A new `is_debuf_effect` helper in `effect.rs` classifies effects properly — covering DOTs, stat reductions, `BlockHealAtk`, and percent modifiers like `DamageRxPercent`.

### Bouclier Défensif — aggro overcounting (+42 instead of +40)

**Root cause:** An `else` branch in `apply_processed_effect_param` generated implicit aggro from any `ChangeCurrentStatByValue` effect on a non-HP, non-Aggro stat. Bouclier Défensif applies Berserk +30, which rounded to 2 extra aggro, giving 42 total.

**Fix:** Removed the `else` branch. Only HP changes (heals/damage) and explicit Aggro stat effects now generate aggro.

### BoostHotsByPercentage — HOT boost only applied to the caster, not to other allies

**Root cause:** `process_effect_type` for `BoostHotsByPercentage` mutated `self.all_effects` on the **launcher's** `CharacterRoundsInfo`. For zone / "All allies" attacks (e.g. Thalia's *Éveil de la forêt*), `is_receiving_atk` / `apply_processed_effect_param` returned early (empty `stats_name` guard) without ever touching the other allies' HOTs.

**Fix:** `process_effect_type` is now a no-op for `BoostHotsByPercentage` on the launcher side (only sets the log message). The actual boost iterates over `self.character_rounds_info.all_effects` in `apply_processed_effect_param`, which runs for every receiving target including the caster.

### Offrande vitale — apparent lack of armor buff impact

The +50% magic/physical armor buff on the target is applied correctly: `set_stats_on_effect` updates `buf_effect_percent` and `recompute_stat_max_and_current` raises the max from 50 → 75. The limited visible damage reduction (~2%) is by design — the armor formula `1000 / (1000 + armor)` yields diminishing returns at low armor values.

---

## Integration with dx-rpg

Add a path override in `dx-rpg/Cargo.toml` to use a local development copy:

```toml
[patch."https://github.com/r0nd0ud0u/lib-rpg.git"]
lib-rpg = { path = "../lib-rpg" }
```

Remove (or comment out) this section before publishing or deploying.
