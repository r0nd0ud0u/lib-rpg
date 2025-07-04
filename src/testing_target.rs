#![allow(dead_code)]

use crate::target::TargetInfo;

#[cfg(not(tarpaulin_include))]
pub fn build_target_boss_indiv() -> TargetInfo {
    TargetInfo {
        name: "Boss1".to_owned(),
        is_targeted: true,
        _is_boss: true,
        _is_reach_rand: false,
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_target_angmar_indiv() -> TargetInfo {
    TargetInfo {
        name: "Angmar".to_owned(),
        is_targeted: true,
        _is_boss: true,
        _is_reach_rand: false,
    }
}
