use serde::{Deserialize, Serialize};

use crate::attack_type::AtksInfo;

#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StatsInGame {
    pub atk_info: AtksInfo,
}
