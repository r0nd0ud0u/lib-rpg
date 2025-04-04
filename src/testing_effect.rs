use crate::{
    common::{all_target_const::*, effect_const::*, reach_const::*, stats_const::*},
    effect::EffectParam,
};

pub fn build_cooldown_effect() -> EffectParam {
    EffectParam {
        effect_type: EFFECT_NB_COOL_DOWN.to_owned(),
        nb_turns: 3,
        sub_value_effect: 0,
        target: TARGET_HIMSELF.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        stats_name: "".to_owned(),
        value: 0,
        number_of_applies: 1,
        ..Default::default()
    }
}

pub fn build_dot_effect_individual() -> EffectParam {
    EffectParam {
        effect_type: EFFECT_VALUE_CHANGE.to_owned(),
        nb_turns: 3,
        sub_value_effect: 0,
        target: TARGET_ENNEMY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        stats_name: HP.to_owned(),
        value: -20,
        number_of_applies: 1,
        ..Default::default()
    }
}

pub fn build_dot_effect_zone() -> EffectParam {
    EffectParam {
        effect_type: EFFECT_VALUE_CHANGE.to_owned(),
        nb_turns: 3,
        sub_value_effect: 0,
        target: TARGET_ENNEMY.to_owned(),
        reach: ZONE.to_owned(),
        stats_name: HP.to_owned(),
        value: -20,
        number_of_applies: 1,
        ..Default::default()
    }
}

pub fn build_dmg_effect_individual() -> EffectParam {
    EffectParam {
        effect_type: EFFECT_VALUE_CHANGE.to_owned(),
        nb_turns: 1,
        sub_value_effect: 0,
        target: TARGET_ENNEMY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        stats_name: HP.to_owned(),
        value: -30,
        number_of_applies: 1,
        ..Default::default()
    }
}

pub fn build_hot_effect_individual() -> EffectParam {
    EffectParam {
        effect_type: EFFECT_VALUE_CHANGE.to_owned(),
        nb_turns: 2,
        sub_value_effect: 0,
        target: TARGET_ALLY.to_owned(),
        reach: INDIVIDUAL.to_owned(),
        stats_name: HP.to_owned(),
        value: 30,
        number_of_applies: 1,
        ..Default::default()
    }
}

pub fn build_hot_effect_zone() -> EffectParam {
    EffectParam {
        effect_type: EFFECT_VALUE_CHANGE.to_owned(),
        nb_turns: 3,
        sub_value_effect: 0,
        target: TARGET_ALLY.to_owned(),
        reach: ZONE.to_owned(),
        stats_name: HP.to_owned(),
        value: 40,
        number_of_applies: 1,
        ..Default::default()
    }
}

pub fn build_hot_effect_all() -> EffectParam {
    EffectParam {
        effect_type: EFFECT_VALUE_CHANGE.to_owned(),
        nb_turns: 3,
        sub_value_effect: 0,
        target: TARGET_ALL_HEROES.to_owned(),
        reach: ZONE.to_owned(),
        stats_name: HP.to_owned(),
        value: 20,
        number_of_applies: 1,
        ..Default::default()
    }
}
