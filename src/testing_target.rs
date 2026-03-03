#![allow(dead_code)]

#[cfg(not(test))]
use crate::target::TargetInfo;

#[cfg(not(tarpaulin_include))]
#[cfg(not(test))]
pub fn build_target_boss_indiv() -> TargetInfo {
    TargetInfo {
        name: "test_boss1".to_owned(),
        is_targeted: true,
        _is_boss: true,
        _is_reach_rand: false,
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg(not(test))]
pub fn build_target_angmar_indiv() -> TargetInfo {
    TargetInfo {
        name: "Angmar".to_owned(),
        is_targeted: true,
        _is_boss: true,
        _is_reach_rand: false,
    }
}
