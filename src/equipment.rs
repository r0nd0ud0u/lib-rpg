use std::path::Path;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::{stats::Stats, utils};

/// Define the parameters of an equipment.
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(default)]
pub struct Equipment {
    /// Name of the equipment
    #[serde(rename = "Categorie")]
    pub category: String,
    /// Type of the equipment
    #[serde(rename = "Nom")]
    pub name: String,
    /// Photo of the equipment
    #[serde(rename = "Nom unique")]
    pub unique_name: String,
    /// Stats of the equipment
    #[serde(rename = "Stats")]
    pub stats: Stats,
}

impl Equipment {
    pub fn try_new_from_json<P: AsRef<Path>>(path: P) -> Result<Equipment> {
        if let Ok(mut value) = utils::read_from_json::<_, Equipment>(&path) {
            value.stats.init();
            Ok(value)
        } else {
            Err(anyhow!("Unknown file: {:?}", path.as_ref()))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::stats_const::*;

    use super::*;

    #[test]
    fn unit_try_new_from_json() {
        let file_path =
            "./tests/equipment/body/right-ring/Anneau de Boromir-4-2024-05-11-12-36-16.json"; // Path to the JSON file
        let equipment = Equipment::try_new_from_json(file_path);
        assert!(equipment.is_ok());
        let equipment = equipment.unwrap();
        assert_eq!(equipment.name, "Anneau de Boromir");
        assert_eq!(equipment.category, "Anneau droit");
        assert_eq!(
            equipment.unique_name,
            "Anneau de Boromir-4-2024-05-11-12-36-16"
        );
        // stats
        // stats - aggro
        assert_eq!(0, equipment.stats.all_stats[AGGRO].buf_equip_percent);
        assert_eq!(0, equipment.stats.all_stats[AGGRO].buf_equip_value);
        // berseck rate
        assert_eq!(4, equipment.stats.all_stats[BERSECK_RATE].buf_equip_value);
        assert_eq!(0, equipment.stats.all_stats[BERSECK_RATE].buf_equip_percent);
    }
}
