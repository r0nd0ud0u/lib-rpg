use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::character_mod::buffers::BufKinds;

/// A single stat/combat modifier granted by a talent. Turned into a `Buffer` (always
/// `is_passive: true, is_passive_enabled: true`) when the talent is unlocked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TalentEffect {
    pub kind: BufKinds,
    #[serde(rename = "stats-name")]
    pub stats_name: String,
    pub value: i64,
    #[serde(rename = "is-percent")]
    pub is_percent: bool,
}

impl Default for TalentEffect {
    fn default() -> Self {
        TalentEffect {
            kind: BufKinds::DefaultBuf,
            stats_name: String::new(),
            value: 0,
            is_percent: false,
        }
    }
}

/// One node of a hero's talent tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TalentDef {
    /// Stable unique key within the tree, e.g. "thalia_verdant_growth_3"
    pub id: String,
    /// Path this talent belongs to, matches a `TalentPath.key`
    pub path: String,
    /// 1..=5, tier 5 is always the path's capstone
    pub tier: u8,
    /// Skill points required to unlock this node
    pub cost: u64,
    /// Capstones are mutually exclusive across the 3 paths of a tree
    pub is_capstone: bool,
    /// Talent id(s) that must already be unlocked (linear per path: previous tier)
    pub requires: Vec<String>,
    pub name_en: String,
    pub name_fr: String,
    pub description_en: String,
    pub description_fr: String,
    /// Usually one effect; capstones may grant two
    pub effects: Vec<TalentEffect>,
}

/// One of a hero's 3 thematic branches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TalentPath {
    pub key: String,
    pub name_en: String,
    pub name_fr: String,
    pub talents: Vec<TalentDef>,
}

/// Static talent tree definition for one hero. Loaded once at startup from
/// `offlines/talents/<universe>/<character-name>.json`; shared across all sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TalentTree {
    /// Matches `Character.db_full_name`
    pub hero_key: String,
    pub paths: Vec<TalentPath>,
}

impl TalentTree {
    pub fn try_new_from_json<P: AsRef<Path>>(path: P) -> Result<TalentTree> {
        crate::utils::read_from_json(path)
    }

    pub fn find_talent(&self, talent_id: &str) -> Option<&TalentDef> {
        self.paths
            .iter()
            .flat_map(|p| p.talents.iter())
            .find(|t| t.id == talent_id)
    }

    /// The capstone (tier 5) talent id of every path other than `path_key`.
    pub fn other_capstones(&self, path_key: &str) -> Vec<&str> {
        self.paths
            .iter()
            .filter(|p| p.key != path_key)
            .flat_map(|p| p.talents.iter())
            .filter(|t| t.is_capstone)
            .map(|t| t.id.as_str())
            .collect()
    }

    /// Content-authoring safety net: every talent id must be unique within the tree,
    /// and every `requires` entry must point at another talent id that actually exists.
    pub fn validate(&self) -> Result<()> {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        let all_talents: Vec<&TalentDef> =
            self.paths.iter().flat_map(|p| p.talents.iter()).collect();
        for talent in &all_talents {
            if !seen.insert(talent.id.as_str()) {
                anyhow::bail!(
                    "Talent tree '{}' has a duplicate talent id '{}'",
                    self.hero_key,
                    talent.id
                );
            }
        }
        for talent in &all_talents {
            for req in &talent.requires {
                if !seen.contains(req.as_str()) {
                    anyhow::bail!(
                        "Talent tree '{}': talent '{}' requires unknown talent id '{}'",
                        self.hero_key,
                        talent.id,
                        req
                    );
                }
            }
        }
        Ok(())
    }
}

/// Per-save-file talent progress for a single character.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TalentBoard {
    /// Total skill points ever earned (levels + milestone bonuses)
    pub skill_points: u64,
    /// Total skill points spent on currently-unlocked talents
    pub spent: u64,
    /// Ids of unlocked talents, in unlock order (needed to reverse cleanly on respec)
    pub unlocked: Vec<String>,
    /// `true` while the player has not yet opened the Talents tab since points were
    /// last granted — drives the notification badge, mirroring
    /// `EquipmentInventory::is_new` / `Inventory::has_unseen_equipment`.
    pub has_unseen_points: bool,
}

impl TalentBoard {
    pub fn available(&self) -> u64 {
        self.skill_points.saturating_sub(self.spent)
    }

    /// Clear the notification badge — call when the player opens the Talents tab.
    pub fn mark_points_seen(&mut self) {
        self.has_unseen_points = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn make_talent(
        id: &str,
        path: &str,
        tier: u8,
        cost: u64,
        requires: Vec<&str>,
        is_capstone: bool,
        effects: Vec<TalentEffect>,
    ) -> TalentDef {
        TalentDef {
            id: id.to_owned(),
            path: path.to_owned(),
            tier,
            cost,
            is_capstone,
            requires: requires.into_iter().map(str::to_owned).collect(),
            name_en: id.to_owned(),
            name_fr: id.to_owned(),
            description_en: String::new(),
            description_fr: String::new(),
            effects,
        }
    }

    pub fn build_test_tree() -> TalentTree {
        let stat_effect = |value: i64| {
            vec![TalentEffect {
                kind: BufKinds::ChangeMaxStat,
                stats_name: "HP".to_owned(),
                value,
                is_percent: true,
            }]
        };
        TalentTree {
            hero_key: "TestHero".to_owned(),
            paths: vec![
                TalentPath {
                    key: "path_a".to_owned(),
                    name_en: "Path A".to_owned(),
                    name_fr: "Chemin A".to_owned(),
                    talents: vec![
                        make_talent("a1", "path_a", 1, 1, vec![], false, stat_effect(10)),
                        make_talent("a2", "path_a", 2, 1, vec!["a1"], false, stat_effect(15)),
                        make_talent("a5", "path_a", 5, 4, vec!["a2"], true, stat_effect(25)),
                    ],
                },
                TalentPath {
                    key: "path_b".to_owned(),
                    name_en: "Path B".to_owned(),
                    name_fr: "Chemin B".to_owned(),
                    talents: vec![make_talent(
                        "b5",
                        "path_b",
                        5,
                        4,
                        vec![],
                        true,
                        stat_effect(25),
                    )],
                },
            ],
        }
    }

    #[test]
    fn unit_talent_board_available() {
        let board = TalentBoard {
            skill_points: 5,
            spent: 3,
            unlocked: vec![],
            has_unseen_points: false,
        };
        assert_eq!(board.available(), 2);
    }

    #[test]
    fn unit_talent_board_available_saturates() {
        let board = TalentBoard {
            skill_points: 1,
            spent: 3,
            unlocked: vec![],
            has_unseen_points: false,
        };
        assert_eq!(board.available(), 0);
    }

    #[test]
    fn unit_talent_board_mark_points_seen() {
        let mut board = TalentBoard {
            has_unseen_points: true,
            ..Default::default()
        };
        board.mark_points_seen();
        assert!(!board.has_unseen_points);
    }

    #[test]
    fn unit_find_talent() {
        let tree = build_test_tree();
        assert!(tree.find_talent("a1").is_some());
        assert!(tree.find_talent("does_not_exist").is_none());
    }

    #[test]
    fn unit_other_capstones() {
        let tree = build_test_tree();
        assert_eq!(tree.other_capstones("path_a"), vec!["b5"]);
        assert_eq!(tree.other_capstones("path_b"), vec!["a5"]);
    }

    #[test]
    fn unit_validate_accepts_well_formed_tree() {
        let tree = build_test_tree();
        assert!(tree.validate().is_ok());
    }

    #[test]
    fn unit_validate_rejects_duplicate_id() {
        let mut tree = build_test_tree();
        // Duplicate "a1" into path_b
        let dup = tree.paths[0].talents[0].clone();
        tree.paths[1].talents.push(dup);

        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("duplicate talent id"));
    }

    #[test]
    fn unit_validate_rejects_dangling_requires() {
        let mut tree = build_test_tree();
        tree.paths[0].talents[0].requires = vec!["ghost_talent".to_owned()];

        let err = tree.validate().unwrap_err();
        assert!(err.to_string().contains("unknown talent id"));
    }
}
