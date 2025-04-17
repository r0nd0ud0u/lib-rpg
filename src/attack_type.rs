use anyhow::{anyhow, Result};
use std::{path::Path, vec};

use serde::{Deserialize, Serialize};

use crate::{
    common::{
        all_target_const::{TARGET_ALLY, TARGET_ENNEMY},
        reach_const::INDIVIDUAL,
        stats_const::HP,
    },
    effect::EffectParam,
    utils,
};

/// Defines the parameters of an attack.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct AttackType {
    /// Name of the attack
    #[serde(rename = "Nom")]
    pub name: String,
    #[serde(rename = "Niveau")]
    pub level: u8,
    #[serde(rename = "Coût de mana")]
    pub mana_cost: u64,
    #[serde(rename = "Coût de vigueur")]
    pub vigor_cost: u64,
    #[serde(rename = "Coût de rage")]
    pub berseck_cost: u64,
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
        if let Ok(value) = utils::read_from_json::<_, AttackType>(&path) {
            Ok(value)
        } else {
            Err(anyhow!("Unknown file: {:?}", path.as_ref()))
        }
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
            all_target_const::TARGET_ENNEMY, character_json_key::STANDARD_CLASS,
            effect_const::EFFECT_VALUE_CHANGE, reach_const::INDIVIDUAL, stats_const::HP,
        },
    };

    #[test]
    fn unit_try_new_from_json() {
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
}
