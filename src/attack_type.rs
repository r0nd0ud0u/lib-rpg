use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use std::{path::Path, vec};

use serde::{Deserialize, Serialize};

use crate::{
    character::CharacterType,
    common::{
        all_target_const::{TARGET_ALLY, TARGET_ALL_HEROES, TARGET_ENNEMY, TARGET_ONLY_ALLY},
        effect_const::EFFECT_NB_COOL_DOWN,
        reach_const::INDIVIDUAL,
        stats_const::{BERSERK, HP, MANA, VIGOR},
    },
    effect::EffectParam,
    stats::Stats,
    utils,
};

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AtksInfo {
    pub atk_name: String,
    pub nb_use: i64,
    pub all_damages_by_target: IndexMap<String, i64>, // key target, i64 dmg or heal accumulated
}

#[derive(Debug, Clone, PartialEq)]
pub struct LauncherAtkInfo {
    pub name: String,
    pub kind: CharacterType,
    pub stats: Stats,
    pub atk_type: AttackType,
}

/// Defines the parameters of an attack.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct AttackType {
    /// Name of the attack
    #[serde(rename = "Nom")]
    pub name: String,
    #[serde(rename = "Niveau")]
    pub level: u64,
    #[serde(rename = "Coût de mana")]
    pub mana_cost: u64,
    #[serde(rename = "Coût de vigueur")]
    pub vigor_cost: u64,
    #[serde(rename = "Coût de rage")]
    pub berseck_cost: u64,
    /// TODO is there any sense for target and reach ? those are defined for each effect of that attack
    #[serde(rename = "Cible")]
    pub target: String,
    #[serde(rename = "Portée")]
    pub reach: String,
    #[serde(rename = "Photo")]
    pub name_photo: String,
    #[serde(rename = "Effet")]
    pub all_effects: Vec<EffectParam>,
    #[serde(rename = "Forme")]
    pub form: String,
    #[serde(rename = "Aggro")]
    pub aggro: i64,
    #[serde(rename = "Durée")]
    pub turns_duration: i64,
}

impl Default for AttackType {
    fn default() -> Self {
        AttackType {
            name: "".to_owned(),
            level: 0,
            mana_cost: 0,
            vigor_cost: 0,
            berseck_cost: 0,
            target: TARGET_ALLY.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            name_photo: "".to_owned(),
            all_effects: vec![],
            form: "".to_owned(),
            aggro: 0,
            turns_duration: 0,
        }
    }
}

impl AttackType {
    pub fn try_new_from_json<P: AsRef<Path>>(path: P) -> Result<AttackType> {
        utils::read_from_json::<_, AttackType>(&path)
            .map_err(|_| anyhow!("Unknown file: {:?}", path.as_ref()))
    }

    pub fn has_only_heal_effect(&self) -> bool {
        let mut is_only_heal_effect = false;
        for e in &self.all_effects {
            if e.stats_name == HP && e.value < 0 {
                return false;
            }
            if e.stats_name == HP && e.value > 0 {
                is_only_heal_effect = true;
            }
        }
        if self.target != TARGET_ENNEMY && is_only_heal_effect {
            return true;
        }
        false
    }

    /////////////////////////////////////////
    /// \brief Character::CanBeLaunched
    /// The attak can be launched if the character has enough mana, vigor and
    /// berseck.
    /// If the atk can be launched, true is returned and the optional<QString> is
    /// set to nullopt Otherwise the boolean is false and a reason must be set.
    ///
    pub fn can_be_launched(
        &self,
        character_level: u64,
        is_heal_atk_blocked: bool, // TODO pass other argument -> global passive power ?
        stats: &Stats,
    ) -> bool {
        // needed level too high
        if character_level < self.level {
            return false;
        }

        // that attack has a cooldown
        for e in &self.all_effects {
            if e.effect_type == EFFECT_NB_COOL_DOWN && e.nb_turns - e.counter_turn > 0 {
                return false;
            }
            // TODO test atk
            if e.stats_name == HP
                && (e.target == TARGET_ALLY
                    || e.target == TARGET_ONLY_ALLY
                    || e.target == TARGET_ALL_HEROES)
                && is_heal_atk_blocked
            {
                return false;
            }
        }

        // atk cost enough ?
        let mana = &stats.all_stats[MANA];
        let vigor = &stats.all_stats[VIGOR];
        let berserk = &stats.all_stats[BERSERK];

        if self.mana_cost * mana.max / 100 > mana.current
            || self.vigor_cost * vigor.max / 100 > vigor.current
            || self.berseck_cost > berserk.current
        {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        attack_type::AttackType,
        character::Character,
        common::{
            all_target_const::TARGET_ENNEMY,
            character_json_key::STANDARD_CLASS,
            effect_const::EFFECT_VALUE_CHANGE,
            reach_const::INDIVIDUAL,
            stats_const::{BERSERK, HP, MANA, VIGOR},
        },
        testing_atk::{build_atk_damage_indiv, build_atk_heal1_indiv},
        testing_effect::{build_cooldown_effect, build_heal_atk_blocked},
    };

    #[test]
    fn unit_try_new_from_json() {
        // existence
        let file_path = "./tests/offlines/attack/test/hehe.json"; // Path to the JSON file
        let atk_type = AttackType::try_new_from_json(file_path);
        assert!(atk_type.is_err());

        let file_path = "./tests/offlines/attack/test/SimpleAtk.json"; // Path to the JSON file
        let atk_type = AttackType::try_new_from_json(file_path);
        assert!(atk_type.is_ok());
        let atk_type: AttackType = atk_type.unwrap();
        assert_eq!(atk_type.name, "SimpleAtk");
        assert_eq!(atk_type.level, 1);
        assert_eq!(atk_type.mana_cost, 9);
        assert_eq!(atk_type.vigor_cost, 0);
        assert_eq!(atk_type.berseck_cost, 0);
        assert_eq!(atk_type.target, TARGET_ENNEMY);
        assert_eq!(atk_type.reach, INDIVIDUAL);
        assert_eq!(atk_type.name_photo, "SimpleAtk.png");
        assert_eq!(atk_type.form, STANDARD_CLASS);
        assert_eq!(atk_type.aggro, 0);
        // decode the effect
        assert_eq!(atk_type.all_effects.len(), 1);
        assert_eq!(atk_type.all_effects[0].stats_name, HP);
        assert_eq!(atk_type.all_effects[0].value, -35);
        assert_eq!(atk_type.all_effects[0].target, TARGET_ENNEMY);
        assert_eq!(atk_type.all_effects[0].reach, INDIVIDUAL);
        assert_eq!(atk_type.all_effects[0].effect_type, EFFECT_VALUE_CHANGE);
        assert_eq!(atk_type.all_effects[0].sub_value_effect, 0);
    }

    #[test]
    fn unit_has_only_heal_effect() {
        let atk_dmg = build_atk_damage_indiv();
        assert_eq!(false, atk_dmg.has_only_heal_effect());

        let mut atk_heal = build_atk_heal1_indiv();
        assert_eq!(true, atk_heal.has_only_heal_effect());

        atk_heal.target = TARGET_ENNEMY.to_owned();
        assert_eq!(false, atk_heal.has_only_heal_effect());
    }

    #[test]
    fn unit_can_be_launched() {
        let mut atk_type = self::AttackType::default();
        let root_path = "./tests/offlines";
        let c1 =
            Character::try_new_from_json("./tests/offlines/characters/test.json", root_path, false)
                .unwrap();
        // nominal case
        atk_type.level = 1;
        atk_type.mana_cost = 0;
        atk_type.vigor_cost = 0;
        atk_type.berseck_cost = 0;
        let result = atk_type.can_be_launched(1, false, &c1.stats);
        assert!(result);
        // character level too low
        let result = atk_type.can_be_launched(0, false, &c1.stats);
        assert!(!result);
        // not enough mana
        atk_type.mana_cost = c1.stats.all_stats[MANA].current + 100;
        let result = atk_type.can_be_launched(1, false, &c1.stats);
        assert!(!result);
        // heal atk blocked
        atk_type.all_effects.push(build_heal_atk_blocked());
        atk_type.mana_cost = c1.stats.all_stats[MANA].current / 100;
        let result = atk_type.can_be_launched(1, true, &c1.stats);
        assert!(!result);
        // active cooldown
        atk_type.all_effects.clear();
        atk_type.all_effects.push(build_cooldown_effect());
        let result = atk_type.can_be_launched(1, false, &c1.stats);
        assert!(!result);
        // inactive cooldown
        atk_type.all_effects.clear();
        let mut effect = build_cooldown_effect();
        effect.counter_turn = effect.nb_turns;
        atk_type.all_effects.push(effect);
        let result = atk_type.can_be_launched(1, false, &c1.stats);
        assert!(result);
        // not enough berseck
        atk_type.berseck_cost = c1.stats.all_stats[BERSERK].current + 100;
        let result = atk_type.can_be_launched(1, false, &c1.stats);
        assert!(!result);
        // not enough vigor
        atk_type.berseck_cost = c1.stats.all_stats[BERSERK].current;
        atk_type.vigor_cost = c1.stats.all_stats[VIGOR].current + 100;
        let result = atk_type.can_be_launched(1, false, &c1.stats);
        assert!(!result);
        // enough energy
        atk_type.berseck_cost = c1.stats.all_stats[BERSERK].current;
        atk_type.vigor_cost = c1.stats.all_stats[VIGOR].current / 100;
        atk_type.mana_cost = c1.stats.all_stats[MANA].current / 100;
        let result = atk_type.can_be_launched(1, false, &c1.stats);
        assert!(result);
    }
}
