use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct Powers {
    /// Enables the critical of the next heal atk after a critical on damage atk
    #[serde(rename = "is_crit_heal_after_crit")]
    pub is_crit_heal_after_crit: bool,
    /// Enables the power to heal the most needy ally using damage tx of previous turn
    #[serde(rename = "is_damage_tx_heal_needy_ally")]
    pub is_damage_tx_heal_needy_ally: bool,
}
