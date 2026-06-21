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

#### `IsDamageTxHealNeedyAlly` (damage converts to ally heal)

`BufKinds::IsDamageTxHealNeedyAlly` — fires immediately when the character deals HP damage, converting `value`% of the damage dealt into a HP heal for the **most needy alive ally** (the hero with the lowest current-HP/max-HP ratio).  The heal is capped at the target's HP max.  A log entry is appended to `ResultLaunchAttack.logs_atk` in the same turn so the heal is visible immediately.

**Elara la guerisseuse de la Lorien** carries this passive with `value = 25`: 25% of her damage is redistributed as healing to whichever ally is lowest on HP, within the same attack turn.

JSON definition (in `CharacterRoundsInfo.Buf-debuf`):

```json
{
  "stats-name": "",
  "is-percent": false,
  "passive-enabled": true,
  "passive": true,
  "kind": "IsDamageTxHealNeedyAlly",
  "value": 25
}
```

> This passive kind can also be enabled dynamically by an attack effect (any attack carrying an `IsDamageTxHealNeedyAlly` effect adds the passive buffer via `process_effect_type` in `rounds_information.rs`), allowing temporary versions on other heroes.

#### Passive stat bonus via `ChangeCurrentStatByPercentage`

A passive `ChangeCurrentStatByPercentage` buffer with a non-empty `stats-name` permanently raises that stat as a percentage of its base value at character load time.  The bonus is stored in `buf_effect_percent` inside `recompute_stat_max_and_current`, so it is automatically included whenever equipment is equipped or removed — no separate re-application is needed.

**Thraïn** carries a passive `ChangeCurrentStatByPercentage` on `Dodge` with `value = 10` (`is-percent: true`): his base Dodge of 5 gains +10 % → +0.5, or with full starting equipment Dodge ≈ 27 → +2.7 ≈ 29 effective Dodge, giving him an additional block chance through the softcap curve.

JSON definition (in `CharacterRoundsInfo.Buf-debuf`):

```json
{
  "stats-name": "Dodge",
  "is-percent": true,
  "passive-enabled": true,
  "passive": true,
  "kind": "ChangeCurrentStatByPercentage",
  "value": 10
}
```

#### Attack launch condition: `ConditionDamagePrevTurn`

When an attack contains an effect with `Buffer.kind = "ConditionDamagePrevTurn"`, the attack may only be launched if the character dealt HP damage on the **previous turn**.  The check mirrors the `process_one_effect` logic:

```
can_be_launched = current_turn_nb > 0 && tx_rx[DamageTx][current_turn_nb − 1] > 0
```

If the condition is not met, `can_be_launched` returns `false` and the attack is hidden from the launchable-attack list.  At processing time, `process_all_effects` also breaks early (no effects are applied) so the attack costs no mana.

**Elara la guerisseuse de la Lorien** — *Lumiere curative* uses this condition: the heal is only available after she dealt damage the previous turn, incentivising a mixed attack/heal play style.

#### Elara's attacks

| Attack | Target | Key mechanics |
|---|---|---|
| **Frappe élémentaire** | 1 Enemy | −70 magic HP; may repeat with 50 % chance if Elara healed last turn (`RepeatIfHeal`) |
| **Don de vie** | 1 Ally | `DecreasingRateOnTurn` (1–3 × decreasing rate); ally +30 % HP, self −15 % HP, ally +25 % max mag/phy power |
| **Lumiere curative** | 1 Ally | Requires `ConditionDamagePrevTurn`; ally +(130 + Elara's magical power) HP |
| **Non sans raison** | All Allies | All allies +100 % HP; `AddAsMuchAsHp` power boost 3 t; `BlockHealAtk` on Elara 3 t; free (0 mana) |

#### Thraïn's attacks

| Attack | Cost | Target | Key mechanics |
|---|---|---|---|
| **Enchaînement Furieux** | 20 Berserk | 1 Enemy | `RepeatAsManyAsPossible`: fires `floor(berserk / actual_cost).max(1)` times where `actual_cost = 20 × max / 100`; each hit deals 50 physical HP damage bypassing armor; all repeats drain rage |
| **Provocation Féroce** | Free | Self + Allies | Self +12 Berserk; self +10 Aggro; `ReinitBuf` Aggro on all allies; self +40 max Critical strike for 3 t; 5-turn cooldown |
| **Tourbillon Destructeur** | 15 Berserk | All Enemies | All enemies −60 physical HP (armor formula applies); self +5 Aggro; self +100 % max Berserk rate for 4 t |

##### `RepeatAsManyAsPossible`

When an attack has a `RepeatAsManyAsPossible` effect the ability fires as many times as the launcher's energy allows, draining that energy on every repeat:

1. `process_atk_cost` deducts the first apply's cost (`raw_cost × stat_max / 100`) from the launcher.
2. `process_atk` reads the remaining energy and the stat's max to compute:
   - `actual_cost = raw_cost × stat_max / 100`
   - `nb_applies = floor((remaining + actual_cost) / actual_cost).max(1)` — recovers the initial energy then divides by the actual per-apply cost.
   - Extra cost for applies 2..N is deducted immediately: `apply_cost_on_stats((nb_applies − 1) × raw_cost, energy_stat)`.
3. `process_one_effect` reads `nb_applies` from `ApplyEffectInit` and sets `number_of_applies` on the `ProcessedEffectParam`.
4. `apply_processed_effect_param` computes `full_amount = nb_applies × buffer.value` (no armor formula — goes through the `else` branch).

Every apply costs energy. The launcher fires until it can no longer afford another repeat.

**Example** — Enchaînement Furieux with 60 Berserk, raw_cost=20, max=110:
- `actual_cost = 20 × 110 / 100 = 22`
- `nb_applies = floor(60 / 22) = 2`
- Berserk spent: 2 × 22 = 44 → remaining = 16
- Damage = 2 × 50 = 100 HP (no armor, bypassed by the `else` branch)

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

`GameAtkEffect::log_text()` is the single source of truth for per-effect log text. It is used by
both the gameboard (displayed inline after each attack) and the log sheet (via `build_logs_atk`).
All messages use `←` to mean "target received".

| Effect type | Format |
|---|---|
| HP damage, no mitigation | `{target} ← {real} HP` |
| HP damage, armor / cap | `{target} ← {real} HP (full: {pre}, real: {real})` |
| HP heal, uncapped | `{target} ← {real} HP ({kind})` |
| HP heal, capped at max | `{target} ← {real} HP (full: {full}, real: {real})` |
| Cooldown | `{target} ← Cooldown for {buf_value} turns` |
| Debuff removed | `{target} ← debuff removed` |
| Debuff remove no-op | *(hidden — `log_text()` returns `None`)* |
| HOT boost | `{target} ← HOTs +{pct}% (+{hp}/turn)` |
| Stat max change | `{target} ← {stat} max +{pct}%` |

**Passive logs** (e.g. `IsDamageTxHealNeedyAlly`) produce `LogData` entries stored in
`ResultLaunchAttack.passive_logs`. They are also included in `logs_atk` so the full log sheet
is consistent. The gameboard renders `passive_logs` separately below the per-effect block.

---

## Hero Balance (LOTR roster)

The four LOTR heroes are balanced around the following roles:

| Hero | Role | Primary resource |
|------|------|-----------------|
| Azrak Ombresang | Physical/Dark damage dealer + debuffer | Mana + Vigor |
| Thalia | Druid — HOT healer + nature buffs | Mana |
| Elara la guerisseuse | Pure healer — burst heal + sacrifice | Mana |
| Thraïn | Berserker tank — taunt + armor + provoke | Berserk |

### Azrak Ombresang
- **Base stats**: 145 HP · 10 Physical armor · Vigor regen 5/turn
- **Passive** (`OverHealBoostStat`): overheal received → bonus Physical power (value 15)
- **Key attack changes**: Furie du Mordor reduced to +20%/+10% power (self/allies); Récupération Mordorienne fixed (values were 0, now +15%/+20%/+25% HP/Mana/Vigor on 20-kill threshold); Fracas des Abysses now restores 20 flat Vigor instead of +200% regen on a zero base; Flèche de la Montagne du Destin DmgRx reduced 100→60% and bonus changed to +20 Physical power; Lame de Morgul DoT reduced to -75/t×4 and DmgRx +30→+20%; Éclipse du Mordor damage 400→280, party DmgRx debuff 25→15%; Chaînes de la Rage 3rd effect target fixed.

### Thalia
- **Passive** (`ChangeMaxStatByPercentage`): +5% max Magic power permanently (nature affinity)
- **Key attack changes**: Rameau Guérisseur mana 20→13; Fleur de l'Espoir -5 Dodge ally penalty removed; Sève Régénératrice mana 10→18 (was underpriced for 120 instant heal + ReinitBuf); Arbre de Vie mana 9→18, power buffs +20%/+30%→+15%/+20%

### Elara la guerisseuse
- **Base stats**: Speed 8→9 · Mana regen 5→8 · Vigor removed from energies (was 0/0 placeholder)
- **Key attack changes**: Eclat d'espoir HP heal 30→20%, mana 18→12; Offrande vitale cooldown 2→3; Rayon astral +2-turn cooldown added; Benediction de la Lorien mana regen bonus 500→15 flat (was game-breaking); Nova etherée damage 150→110, phantom 3rd effect removed; Prière du desespoir armor 100→75%, power 100→60%; Non sans raison mana cost 0→24% (ultimate now has a real cost)

### Thraïn
- **Base stats**: Berserk rate 0→5/turn (passive buildup enables Tourbillon Destructeur's rate-boost to be meaningful)
- **Passive** (`ChangeCurrentStatByPercentage`): kind field corrected from `ChangeCurrentStatByValue` to match README documentation; +10% Dodge permanently
- **Key attack changes**: Fracas Marteau damage -25→-35; Cor d'Erebor HP boost +25→+15%; Coup Puissant berserk cost 20→15; Folie des profondeurs self-HP penalty -30→-20%; Fracassage de crâne DamageRxPercent -20→+20 (was accidentally reducing enemy's incoming damage instead of increasing it)

---

## Boss Balance (LOTR roster)

Bosses for the 10 LOTR scenarios, scaled by difficulty tier:

| Boss | Stage(s) | HP | Tier |
|------|----------|----|------|
| Gobelin Eclaireur | 1, 2, 4, 6, 8 | 300 | Common |
| Angmar10PV | 2 | 10 | Common (tutorial) |
| Orc Pillard | 3, 4 | 1 500 | Common |
| Champion Orc | 5, 6 | 5 000 | Intermediate |
| Necromancien du Mordor | 7, 8 | 10 000 | Intermediate |
| Nazgul | 9 | 25 000 | Advanced |
| Sauron l'Oeil Flamboyant | 10 | 50 000 + 100 regen/turn | Advanced |

### Changes applied

**Gobelin Eclaireur / Griffure**: `Tours actifs` 3→1 — was a 3-turn DoT for 105 total damage at stage 1 (too high for intro). Now instant -35 physical. Description updated to match.

**Champion Orc / Charge**: Description corrected from "physical damage" to "magic damage" (`IsMagicEffect: true` was always set).

**Necromancien du Mordor / Malédiction des Morts**: DoT -80/turn → -50/turn (3 turns, AoE). Was -720 total party damage per cast, now -450 — survivable with a healer.

**Nazgul / Lame du Spectre**: -300 physical single target → -220. Most heroes have 450–600 HP; -300 was near-instant kill even for Thraïn after armor. -220 is still a heavy threat.

**Sauron l'Oeil Flamboyant**:
- Physical armor: 800→80, Magical armor: 800→60. At 800 armor the boss absorbed ~89% of all incoming damage (final_dmg = raw × 100/900 ≈ 11%) making him unkillable. At 80/60, physical hits do ~55% effective and magic ~62% — very tanky but beatable. HP regen 100/turn is kept as a signature final-boss mechanic.
- Frappe Corrompue: -450 physical → -350. Still lethal but doesn't instant-kill every hero.
- Malédiction Ancienne: -120/turn → -80/turn (4-turn AoE DoT). Was -1920 total party damage per cast, now -1280 — devastating but survives with healer focus.

---

## Equipment & Loot

### Equipment tiers

Two tiers of body equipment exist (`starting_*` and `medium_*`). Stats roughly double between tiers.

| Slot | Starting bonus | Medium bonus |
|------|---------------|-------------|
| Belt | Physical power +10 | Physical power +20 |
| LeftWeapon | Physical power +10 | Physical power +20 |
| RightWeapon | Physical power +10 | Physical power +20 |
| LeftRing | Berserk +10 | Berserk +20 |
| RightRing | HP regeneration +5 | Vigor +20 |
| Gloves | Magical power +10 | Magical power +20 |
| Amulet | Dodge +4 · Mana +10 | Dodge +4 · Mana +20 |
| Chest | Magical armor +5 · Physical armor +5 | Magical armor +10 · Physical armor +5 |
| Pants | Physical armor +10 | Physical armor +20 |
| Head | Physical armor +10 | — (no medium tier) |
| Shoes | Dodge +10 | Dodge +20 |
| Cape | Dodge +10 | — (no medium tier) |
| Tattoes | Class-specific (all zero in starter slot) | — |

### Loot progression across 10 stages

| Stage | Warrior/Berserker | Healer/Mage | Gold |
|-------|------------------|-------------|------|
| 1 | — | — | 30 |
| 2 | medium belt | — | 50 |
| 3 | — | medium amulet | 70 |
| 4 | medium pants | — | 90 |
| 5 | — | medium pants + medium belt | 150 |
| 6 | medium shoes | — | 200 |
| 7 | — | medium shoes | 250 |
| 8 | medium left ring | medium right ring | 300 |
| 9 | medium right ring | medium left ring | 500 |
| 10 | medium gloves | medium gloves | 1 000 |

Slots never awarded as loot (store-only): Chest, Head, Cape, LeftWeapon, RightWeapon.

### Bug fixes applied

- **`starting_right_ring.json` created** — all heroes referenced "starting right ring" but the file did not exist; slot was silently empty. Now gives HP regeneration +5.
- **`starting_right_weapon.json` created** — same issue for RightWeapon. Now gives Physical power +10.
- **`starting_tattoo.json` created** — Elara's equipment file referenced "starting tattoo" which didn't exist.
- **`medium_gloves.json` Nom fixed** — file had `Nom/Nom unique: "starting gloves"`, causing stage 10 loot drops to silently fail (no item matched "medium gloves"). Fixed to "medium gloves".
- **`meidum_right_weapon.json` renamed** to `medium_right_weapon.json` (filename typo).
- **Stage 5 duplicate amulet removed** — stages 3 and 5 both dropped "medium amulet" for Healer/Mage. Stage 5 second slot changed to "medium belt" (Physical power +20), giving mage/healer classes an offensive upgrade they never otherwise received.

---

## Building & Testing

```bash
cargo fmt
cargo clippy --all-targets
cargo test
```

All 297 tests should pass with no warnings.

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

### `load_next_scenario` — "Failed to initialize game" when universe is set

**Root cause:** `load_next_scenario` searched for the next scenario with `level == current_level + 1 && universe == current_universe`. On a fresh `GameManager`, `current_scenario` is the zero-value default, which has `universe == ""`. But `DataManager` injects the subdirectory name as the universe (e.g. `"lotr"`) into every loaded scenario, so no scenario ever matched `universe == ""`.

**Fix:** When `current_universe` is empty (first load of a new game), the universe filter is skipped and the search finds the first `level == 1` scenario regardless of universe. Subsequent calls use the real universe from the now-loaded scenario.

### dx-rpg — Shop "Buy" button had no effect

**Root cause (in dx-rpg `event_store.rs`):** `buy_item_handler` / `sell_item_handler` modified the hero directly inside `pm.active_heroes`, then called `pm.modify_active_character(&id_name)` which copies `pm.current_player` (the active combat character, unchanged) back over that hero, erasing the purchase.

**Fix:** Removed the `pm.modify_active_character` call. The direct `active_heroes.iter_mut()` mutation is sufficient; `modify_active_character` is only for post-combat state propagation.

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
