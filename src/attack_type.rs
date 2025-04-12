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
    use crate::attack_type::AttackType;

    #[test]
    fn unit_try_new_from_json() {
        let file_path = "./tests/attack/test/SimpleAtk.json"; // Path to the JSON file
        let c = AttackType::try_new_from_json(file_path);
        assert!(c.is_ok());
        let c = c.unwrap();
    }
}
