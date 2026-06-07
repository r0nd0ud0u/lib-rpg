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

### `loaded_from_save`

`CoreGameData.loaded_from_save` is `false` for fresh games and `true` when a game is restored from a save file.  UI layers use this flag to lock the universe selector once a save has been loaded.

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

## Building & Testing

```bash
cargo fmt
cargo clippy --all-targets
cargo test
```

All 161 tests should pass with no warnings.

---

## Integration with dx-rpg

Add a path override in `dx-rpg/Cargo.toml` to use a local development copy:

```toml
[patch."https://github.com/r0nd0ud0u/lib-rpg.git"]
lib-rpg = { path = "../lib-rpg" }
```

Remove (or comment out) this section before publishing or deploying.
