#![allow(dead_code)]
#[cfg(not(tarpaulin_include))]
use crate::character_mod::{
    buffers::{BufKinds, Buffer},
    effect::{EffectParam, ProcessedEffectParam},
};
use crate::common::constants::{all_target_const::*, reach_const::*, stats_const::*};

#[cfg(not(tarpaulin_include))]
pub fn build_cooldown_effect() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 1,
            sub_value_effect: 0,
            target_kind: TARGET_HIMSELF.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            buffer: Buffer {
                kind: BufKinds::CooldownTurnsNumber,
                value: 3,
                is_percent: false,
                stats_name: String::new(),
                is_passive_enabled: false,
            },
            ..Default::default()
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_heal_atk_blocked() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 1,
            sub_value_effect: 0,
            target_kind: TARGET_ALL_ALLIES.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            buffer: Buffer {
                kind: BufKinds::BlockHealAtk,
                value: 0,
                is_percent: false,
                stats_name: HP.to_owned(),
                is_passive_enabled: false,
            },
            ..Default::default()
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_dot_effect_individual() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 3,
            sub_value_effect: 0,
            target_kind: TARGET_ENNEMY.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            buffer: Buffer {
                kind: BufKinds::ChangeCurrentStatByValue,
                value: -20,
                is_percent: false,
                stats_name: HP.to_owned(),
                is_passive_enabled: false,
            },
            ..Default::default()
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_dot_effect_zone() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 3,
            sub_value_effect: 0,
            target_kind: TARGET_ENNEMY.to_owned(),
            reach: ZONE.to_owned(),
            buffer: Buffer {
                kind: BufKinds::ChangeCurrentStatByValue,
                value: -20,
                is_percent: false,
                stats_name: HP.to_owned(),
                is_passive_enabled: false,
            },
            ..Default::default()
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_dmg_effect_individual() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 1,
            sub_value_effect: 0,
            target_kind: TARGET_ENNEMY.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            buffer: Buffer {
                kind: BufKinds::ChangeCurrentStatByValue,
                value: -30,
                is_percent: false,
                stats_name: HP.to_owned(),
                is_passive_enabled: false,
            },
            is_magic_atk: false,
            conditions: vec![],
            is_passive: false,
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_dmg_effect_zone() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 1,
            sub_value_effect: 0,
            target_kind: TARGET_ENNEMY.to_owned(),
            reach: ZONE.to_owned(),
            buffer: Buffer {
                kind: BufKinds::ChangeCurrentStatByValue,
                value: -30,
                is_percent: false,
                stats_name: HP.to_owned(),
                is_passive_enabled: false,
            },
            is_magic_atk: false,
            conditions: vec![],
            is_passive: false,
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_hot_effect_individual() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 2,
            sub_value_effect: 0,
            target_kind: TARGET_ALLY.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            buffer: Buffer {
                kind: BufKinds::ChangeCurrentStatByValue,
                value: 30,
                is_percent: false,
                stats_name: HP.to_owned(),
                is_passive_enabled: false,
            },
            is_magic_atk: false,
            conditions: vec![],
            ..Default::default()
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_hot_effect_zone() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 3,
            sub_value_effect: 0,
            target_kind: TARGET_ALLY.to_owned(),
            reach: ZONE.to_owned(),
            buffer: Buffer {
                kind: BufKinds::ChangeCurrentStatByPercentage,
                value: 30,
                is_percent: false,
                stats_name: HP.to_owned(),
                is_passive_enabled: false,
            },
            is_magic_atk: false,
            conditions: vec![],
            ..Default::default()
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_hot_effect_all() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 3,
            sub_value_effect: 0,
            target_kind: TARGET_ALL_ALLIES.to_owned(),
            reach: ZONE.to_owned(),
            buffer: Buffer {
                kind: BufKinds::ChangeCurrentStatByValue,
                value: 20,
                is_percent: false,
                stats_name: HP.to_owned(),
                is_passive_enabled: false,
            },
            is_magic_atk: false,
            conditions: vec![],
            ..Default::default()
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_effect_max_stats() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 3,
            sub_value_effect: 0,
            target_kind: TARGET_ENNEMY.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            buffer: Buffer {
                kind: BufKinds::ChangeMaxStatByValue,
                value: -20,
                is_percent: false,
                stats_name: HP.to_owned(),
                is_passive_enabled: false,
            },
            is_magic_atk: false,
            conditions: vec![],
            ..Default::default()
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_debuf_effect_individual() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 3,
            sub_value_effect: 0,
            target_kind: TARGET_ENNEMY.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            buffer: Buffer {
                kind: BufKinds::ChangeCurrentStatByValue,
                value: -20,
                is_percent: false,
                stats_name: MAGICAL_ARMOR.to_owned(),
                is_passive_enabled: false,
            },
            is_magic_atk: false,
            conditions: vec![],
            ..Default::default()
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_buf_effect_individual() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            nb_turns: 3,
            sub_value_effect: 0,
            target_kind: TARGET_ENNEMY.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            buffer: Buffer {
                kind: BufKinds::ChangeCurrentStatByValue,
                value: 20,
                is_percent: false,
                stats_name: MAGICAL_ARMOR.to_owned(),
                is_passive_enabled: false,
            },
            is_magic_atk: false,
            conditions: vec![],
            ..Default::default()
        },
        number_of_applies: 1,
        ..Default::default()
    }
}

#[cfg(not(tarpaulin_include))]
pub fn build_buf_effect_individual_speed_regen() -> ProcessedEffectParam {
    ProcessedEffectParam {
        input_effect_param: EffectParam {
            buffer: Buffer {
                kind: BufKinds::ChangeMaxStatByValue,
                value: 10,
                is_percent: false,
                stats_name: SPEED_REGEN.to_owned(),
                is_passive_enabled: false,
            },
            nb_turns: 3,
            sub_value_effect: 0,
            target_kind: TARGET_HIMSELF.to_owned(),
            reach: INDIVIDUAL.to_owned(),
            ..Default::default()
        },
        number_of_applies: 6,
        ..Default::default()
    }
}
