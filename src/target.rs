use indexmap::IndexSet;

use crate::common::all_target_const::*;

/// Define all the parameters of target info during a round
#[derive(Default, Debug, Clone)]
pub struct TargetInfo {
    pub name: String,
    _is_targeted: bool,
    _is_boss: bool,
    _is_reach_rand: bool,
}

impl TargetInfo {}

pub fn is_target_ally(target: &str) -> bool {
    let targets: IndexSet<&str> = [
        TARGET_ALLY,
        TARGET_ALL_HEROES,
        TARGET_HIMSELF,
        TARGET_ONLY_ALLY,
    ]
    .iter()
    .cloned()
    .collect();
    targets.contains(target)
}

#[cfg(test)]
mod tests {}
