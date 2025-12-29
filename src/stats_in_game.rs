use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{attack_type::AtksInfo, effect::EffectOutcome};

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StatsInGame {
    pub all_atk_info: Vec<AtksInfo>,
}

impl StatsInGame {
    pub fn update_by_effectoutcome(&mut self, eo: &EffectOutcome) {
        let temp_target: &str = &eo.target_name;
        if let Some(i) = self
            .all_atk_info
            .iter()
            .position(|item| item.atk_name == eo.atk)
        {
            self.all_atk_info[i].nb_use += 1;
            self.all_atk_info[i].all_damages_by_target[temp_target] = eo.real_hp_amount_tx;
        } else {
            let mut im = IndexMap::new();
            im.insert(eo.atk.clone(), eo.real_hp_amount_tx);
            self.all_atk_info.push(AtksInfo {
                atk_name: eo.target_name.clone(),
                nb_use: 1,
                all_damages_by_target: im,
            });
        }
    }
}
