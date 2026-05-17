# lib-rpg — Architecture & Game Mechanics

## Table of Contents

1. [High-level architecture](#high-level-architecture)
2. [Module structure](#module-structure)
3. [Offline data layout](#offline-data-layout)
4. [Game flow](#game-flow)
5. [Character anatomy](#character-anatomy)
6. [Stats system](#stats-system)
7. [Attack flow & damage calculation](#attack-flow--damage-calculation)
8. [Critical strike system](#critical-strike-system)
9. [Dodge & block system](#dodge--block-system)
10. [Effects, HOTs & DOTs](#effects-hots--dots)
11. [Buffers & debuffers](#buffers--debuffers)
12. [Aggro system](#aggro-system)
13. [Experience & level-up](#experience--level-up)
14. [Equipment](#equipment)

---

## High-level architecture

```mermaid
flowchart TD
    DM["DataManager\nLoads JSON files on disk"] --> GM["GameManager\nMain entry point for the library"]
    GM --> GS["GameState\nTurn, round, status, statistics"]
    GM --> PM["PlayerManager\nActive heroes / bosses"]
    PM --> CP["current_player: Character\nAttacks the target this turn"]
    PM --> Heroes["active_heroes: Vec&lt;Character&gt;"]
    PM --> Bosses["active_bosses: Vec&lt;Character&gt;"]
    PM --> ET[equipment_table]
```

The client calls `GameManager` exclusively. `DataManager` is only called during initialisation.

---

## Module structure

```mermaid
flowchart LR
    subgraph src
        lib["lib.rs\n(public API)"]
        utils["utils.rs\n(helpers: random, IO)"]
        subgraph character_mod
            character["character.rs\n(Character struct + methods)"]
            stats_rs["stats.rs\n(Attribute, Stats)"]
            attack_type["attack_type.rs\n(AttackType)"]
            effect_rs["effect.rs\n(EffectParam, EffectOutcome)"]
            buffers_rs["buffers.rs\n(BufKinds, Buffer)"]
            rounds_info["rounds_information.rs\n(CharacterRoundsInfo)"]
            class_rs["class.rs\n(Class enum)"]
            rank_rs["rank.rs\n(Rank enum)"]
            energy_rs["energy.rs\n(Energy)"]
            equipment_rs["equipment.rs\n(Equipment)"]
            inventory_rs["inventory.rs\n(Inventory, Consumable)"]
            loot_rs["loot.rs\n(Loot, LootType)"]
            experience_rs["experience.rs\n(exp formula)"]
            target_rs["target.rs\n(TargetData)"]
        end
        subgraph server
            game_manager_rs["game_manager.rs\n(GameManager)"]
            players_manager_rs["players_manager.rs\n(PlayerManager)"]
            game_state_rs["game_state.rs\n(GameState)"]
            data_manager_rs["data_manager.rs\n(DataManager)"]
            scenario_rs["scenario.rs\n(Scenario)"]
            end_scenario["end_of_scenario.rs\n(EndOfScenario)"]
            game_paths["game_paths.rs"]
        end
        subgraph common
            constants_rs["constants.rs\n(all constants)"]
            log_data["log_data.rs\n(LogData)"]
        end
    end
```

---

## Offline data layout

All game data lives under `offlines/` and is loaded from JSON by `DataManager`.

| Folder | Content |
|---|---|
| `offlines/characters/` | One `.json` per character (heroes and bosses) |
| `offlines/attack/<CharacterName>/` | One `.json` per attack for that character |
| `offlines/equipment/body/` | Equipment items |
| `offlines/equipment/characters/` | Equipment pre-assigned to characters |
| `offlines/scenarios/` | Scenario files (`stage_1.json`, …) |

---

## Game flow

### Initialisation

```mermaid
sequenceDiagram
    participant Client
    participant DataManager
    participant GameManager
    participant PlayerManager

    Client->>DataManager: try_new(path)
    DataManager->>DataManager: load characters, attacks, equipment, scenarios
    Client->>GameManager: new(path, equipment_table, scenarios)
    GameManager->>PlayerManager: set_active_heroes / set_active_bosses
    PlayerManager->>PlayerManager: init stats, tx_rx table, exp_to_next_level
    Client->>GameManager: load_next_scenario()
    GameManager->>GameManager: set current_scenario, active bosses for stage
```

### Turn loop

```mermaid
sequenceDiagram
    participant Client
    participant GameManager
    participant PlayerManager
    participant Character

    Client->>GameManager: launch_attack(atk_name)
    GameManager->>Character: process_atk_cost(atk_name)
    GameManager->>PlayerManager: process_all_dodging(atk_level)
    Note over PlayerManager: all potential targets roll dodge/block
    GameManager->>Character: process_critical_strike(atk_name)
    Note over Character: softcap roll + streak-breaker check
    GameManager->>Character: process_atk(game_state, is_crit, atk)
    loop For each effect × each target
        GameManager->>Character: is_receiving_atk(effect, is_crit, launcher_info)
        Character->>Character: apply_processed_effect_param(...)
        Note over Character: armor, power, buf/debuf, aggro
    end
    GameManager->>GameManager: update tx_rx (CriticalStrike slot)
    GameManager->>PlayerManager: process_died_players()
    GameManager->>GameManager: eval_end_of_round()
    GameManager-->>Client: ResultLaunchAttack
```

---

## Character anatomy

```mermaid
flowchart TD
    C[Character]
    C --> stats_n["Stats\n18 Attributes"]
    C --> cri["CharacterRoundsInfo\nFight state"]
    C --> atks["attacks_list\nIndexMap of AttackType"]
    C --> inv["Inventory\nEquipment + Consumables"]
    C --> energies["Vec&lt;Energy&gt;\nMana / Vigor / Berserk"]
    C --> cls["Class\nStandard / Berserker\nHealer / Mage / Warrior"]
    C --> rnk["Rank\nCommon / Intermediate / Advanced"]
    C --> lvl["level: u64"]

    cri --> txrx["tx_rx: Vec&lt;HashMap&lt;turn,amount&gt;&gt;\nDmgRx/Tx · HealRx/Tx\nOverHealRx · Aggro · CriticalStrike"]
    cri --> buffers_n["all_buffers: Vec&lt;Buffer&gt;"]
    cri --> effects_n["all_effects: Vec&lt;GameAtkEffect&gt;\nActive HOTs / DOTs / buffs"]
    cri --> dodge_info["dodge_info: DodgeInfo"]
    cri --> drought["crit_drought_counter\ndodge_drought_counter"]
```

---

## Stats system

Each character has 18 stats stored in an `IndexMap<String, Attribute>`:

| Stat name | Role |
|---|---|
| `HP` | Hit points |
| `Mana` | Mana resource |
| `Vigor` | Vigor resource |
| `Berserk` | Berserk resource |
| `Physical armor` | Reduces incoming physical damage |
| `Magic armor` | Reduces incoming magical damage |
| `Physical power` | Boosts outgoing physical damage |
| `Magic power` | Boosts outgoing magical damage |
| `Aggro` | Threat accumulation (last 5 turns, see NB_TURN_SUM_AGGRO) |
| `Speed` | Determines turn order; consumes SPEED_THRESHOLD (100) per turn |
| `Critical strike` | Raw stat fed into the softcap formula |
| `Dodge` | Raw stat fed into the softcap formula |
| `HP regeneration` | Passive HP restored per round |
| `Mana regeneration` | Passive Mana restored per round |
| `Vigor regeneration` | Passive Vigor restored per round |
| `Berserk rate` | Rate at which Berserk fills |
| `Aggro rate` | Multiplier on generated aggro |
| `Speed regeneration` | Speed restored per round |

Each `Attribute` tracks:

```
current = max_raw + equip_value + equip_percent×max_raw/100
                  + buf_effect_value + buf_effect_percent×max_raw/100
```

Stats that scale on level-up (10 % of `max_raw`): HP, Mana, Vigor, Physical/Magic power, Physical/Magic armor, Speed.

---

## Attack flow & damage calculation

```mermaid
flowchart TD
    A["apply_processed_effect_param\ncalled on target"] --> B{HP effect?}
    B -- No --> C["Apply stat change directly"]
    B -- Yes --> D["Compute full_amount\nbase value + launcher power scaling"]
    D --> E["apply_buf_debuf\nbuffers + debuffers + crit multiplier"]
    E --> F{is_crit?}
    F -- Yes --> G["full_amount × COEFF_CRIT_DMG 2.0\n+ DamageCritCapped bonus"]
    F -- No --> H["full_amount unchanged"]
    G --> I["armor reduction\nphysical or magical"]
    H --> I
    I --> J["update_hp_process_real_amount\ncap at 0 / max"]
    J --> K["Update tx_rx\nDmgRx/Tx or HealRx/Tx or OverHealRx"]
    K --> L["Compute aggro\nreal_amount × aggro_rate"]
    L --> M["Update tx_rx Aggro slot"]
```

### Armor reduction formula

$$\text{real damage} = \text{full damage} \times \frac{1000}{1000 + \text{armor}}$$

This gives a soft reduction: armor=1000 → 50 %, armor=3000 → 25 %.

### Power scaling (HP effects only)

$$\text{full amount} = \text{base value} + \frac{\text{launcher power}}{\text{nb turns}}$$

where `launcher_power` is `Physical power` for physical attacks and `Magic power` for magical attacks.

---

## Critical strike system

### Probability — hyperbolic softcap

Raw `Critical strike` stat is converted to an effective percentage using a softcap:

$$P_{\text{crit}} = \frac{\text{stat}}{100 + \text{stat}} \times 100$$

| Raw stat | Effective chance |
|---|---|
| 10 | ≈ 9 % |
| 30 | ≈ 23 % |
| 60 | ≈ 38 % |
| 100 | 50 % |
| 200 | ≈ 67 % |

This prevents hard-capping at a specific value while allowing high investment to still matter.

### Excess stat → DamageCritCapped bonus

If `raw_stat > 60`, the excess converts into an additive bonus on top of the base crit multiplier:

```
delta = raw_stat − 60
effective_crit_multiplier = COEFF_CRIT_DMG (2.0) + delta
```

So a character with stat=80 crits at `×2.0 + 20 = ×22.0` (the multiplier, not the bonus — the delta value is small by design).

### Priority order for a crit to fire

1. **Passive `NextHealAtkIsCrit`** — fires unconditionally on the next heal-only attack when `is_passive_enabled = true`. Disables the passive once consumed.
2. **Streak-breaker** — if the drought counter ≥ threshold, the crit is guaranteed (see below).
3. **Dice roll** — `rand(1..=100) ≤ P_crit`.

If none fire, `crit_drought_counter` increments by 1.

### Streak-breaker

Prevents frustrating long dry spells. After `N` consecutive turns without a crit, the next one is guaranteed.

| Activation condition | Default threshold N |
|---|---|
| `StreakBreakerCrit` buffer active (set by any effect) | buffer's `value` field |
| `Class::Berserker` | 3 turns |
| `Rank::Advanced` AND `level ≥ 5` | 5 turns |
| `Rank::Intermediate` AND `level ≥ 10` | 8 turns |

Precedence: buffer > class > rank. If none match, the streak-breaker is disabled.

An attack or equipment effect can enable/tune the streak-breaker by applying a `StreakBreakerCrit` buffer (via `BufKinds::StreakBreakerCrit`) with `value` = desired threshold.

---

## Dodge & block system

### Dodge probability — same softcap

$$P_{\text{dodge}} = \frac{\text{Dodge stat}}{100 + \text{Dodge stat}} \times 100$$

### Class behaviour

| Class | On dodge condition met |
|---|---|
| Standard / Healer / Mage / Warrior | **Dodges** — attack has no effect |
| Berserker | Always **blocks** — takes 10 % of the intended damage |

Ultimate-level attacks (`level = ULTIMATE_LEVEL = 13`) can never be dodged or blocked.

### Streak-breaker (dodge)

Same mechanism as crit. Counter increments on each non-dodge. Berserkers are excluded (they never truly dodge, they always block).

| Activation condition | Default threshold N |
|---|---|
| `StreakBreakerDodge` buffer active | buffer's `value` field |
| `Rank::Advanced` AND `level ≥ 5` | 5 turns |
| `Rank::Intermediate` AND `level ≥ 10` | 8 turns |

---

## Effects, HOTs & DOTs

An `EffectParam` defines:

| Field | Meaning |
|---|---|
| `nb_turns` | Duration; 1 = instant effect only |
| `buffer.kind` | What the effect does (see `BufKinds`) |
| `buffer.value` | Magnitude |
| `buffer.stats_name` | Which stat is affected (e.g. `"HP"`) |
| `target_kind` | `"Ennemie"` or `"Allié"` |
| `reach` | `"Individuel"` or `"Zone"` |
| `is_magic_atk` | Selects which power stat scales the damage |

HOT/DOT effects use `BufKinds` of type:
- `ChangeCurrentStatByValue` — absolute value change each turn
- `ChangeCurrentStatByPercentage` — percentage of max stat each turn
- `DecreasingRateOnTurn` — success rate decreases each turn
- `RepeatAsManyAsPossible` — applies as many times as possible

Processing rules:
- An effect launched on turn `T` starts ticking on turn `T+1`.
- `counter_turn` increments each turn; effect expires when `counter_turn == nb_turns`.
- `process_hot_and_dot()` runs at the start of each turn and returns the total amount to apply as HP change.

---

## Buffers & debuffers

A `Buffer` has `kind: BufKinds`, `value: i64`, `is_percent: bool`, and optional `stats_name`.

Key `BufKinds` and their roles:

| Kind | Role |
|---|---|
| `DamageTxPercent` | Increases outgoing damage by % |
| `DamageRxPercent` | Increases incoming damage taken by % |
| `HealTxPercent` | Increases outgoing heal by % |
| `HealRxPercent` | Increases incoming heal by % |
| `DamageCritCapped` | Bonus crit multiplier for excess crit stat above 60 |
| `NextHealAtkIsCrit` | Passive: next heal attack is guaranteed critical |
| `MultiValue` | Multiplies heal output |
| `BoostedByHots` | Heal boosted by active HOTs |
| `ChangeCurrentStatByValue` | Directly modifies a stat's current value |
| `ChangeMaxStatByValue` | Modifies a stat's max (adjusts current by ratio) |
| `ChangeMaxStatByPercentage` | Same but by % |
| `BlockHealAtk` | Prevents the target from receiving heals |
| `CooldownTurnsNumber` | Puts an attack on cooldown |
| `StreakBreakerCrit` | Enables/tunes crit streak-breaker (value = threshold) |
| `StreakBreakerDodge` | Enables/tunes dodge streak-breaker (value = threshold) |
| `IsDamageTxHealNeedyAlly` | Converts TX damage into heals on the most-wounded ally |
| `ApplyEffectInit` | Stores the number of times an effect repeats |
| `DecreasingRateOnTurn` | Decreasing success-rate HOT/DOT |

Buffers on `CharacterRoundsInfo.all_buffers` are accumulated with `update_buffer()` (same kind → value accumulates; new kind → appended).

---

## Aggro system

Aggro determines which character is preferentially targeted by boss AIs.

```mermaid
flowchart LR
    A["Each effect applied on an enemy"]
    A --> B["Compute aggro\nreal_amount × aggro_rate"]
    B --> C["Store in tx_rx Aggro slot\nfor current turn"]
    C --> D["init_aggro_on_turn\nsum last 5 turns"]
    D --> E["Character Aggro stat updated"]
    E --> F["Boss AI picks target\nwith highest Aggro"]
```

`NB_TURN_SUM_AGGRO = 5`: only the last 5 turns contribute to the aggro total. This prevents old, inactive characters from holding aggro indefinitely.

---

## Experience & level-up

Experience needed for the next level is calculated by `build_exp_to_next_level(rank, class, level)`.

On level-up:
1. All stats in `STATS_TO_LEVEL_UP` increase by 10 % of `max_raw`.
2. `exp_to_next_level` is recomputed for the new level.
3. Any excess experience carries over.

---

## Equipment

Equipment applies stat bonuses tracked separately from effect buffers:
- `buf_equip_value` — flat bonus from equipment
- `buf_equip_percent` — percentage bonus from equipment

This separation means equipment bonuses are applied at load time and survive effect resets, while in-fight buffers (from effects) are managed independently.

Equipment can be assigned per character in `offlines/equipment/characters/` or looted at the end of a scenario based on the hero's class.
