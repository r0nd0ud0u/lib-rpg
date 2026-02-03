use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use std::{path::Path, vec};

use serde::{Deserialize, Serialize};

use crate::{
    character::CharacterType,
    common::{
        all_target_const::{TARGET_ALLY, TARGET_ENNEMY},
        reach_const::INDIVIDUAL,
        stats_const::*,
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
}

#[cfg(test)]
mod tests {
    use crate::{
        attack_type::AttackType,
        common::{
            all_target_const::{TARGET_ALLY, TARGET_ENNEMY},
            character_json_key::STANDARD_CLASS,
            effect_const::EFFECT_VALUE_CHANGE,
            reach_const::INDIVIDUAL,
            stats_const::*,
        },
        testing_atk::{build_atk_damage_indiv, build_atk_heal1_indiv},
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
        assert!(!atk_dmg.has_only_heal_effect());

        let mut atk_heal = build_atk_heal1_indiv();
        assert!(atk_heal.has_only_heal_effect());

        atk_heal.target = TARGET_ENNEMY.to_owned();
        assert!(!atk_heal.has_only_heal_effect());
    }
}
