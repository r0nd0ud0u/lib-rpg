use crate::{
    common::stats_const::HP,
    effect::{self, EffectParam},
    players_manager::GameAtkEffects,
};

/// ExtendedCharacter
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct CharacterFightInfo {
    /// Fight information: Is the random character targeted by the current attack of other character
    #[serde(default, rename = "is_random_target")]
    pub is_random_target: bool,
    /// Fight information: TODO is_heal_atk_blocked
    #[serde(default, rename = "is_heal_atk_blocked")]
    pub is_heal_atk_blocked: bool,
    /// Fight information: Playing the first round of that tour
    #[serde(default, rename = "is_first_round")]
    pub is_first_round: bool,
}

impl Default for CharacterFightInfo {
    fn default() -> Self {
        CharacterFightInfo {
            is_random_target: false,
            is_heal_atk_blocked: false,
            is_first_round: true,
        }
    }
}

/// ExtendedCharacter
#[derive(Default, Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq)]
pub struct HotsBufs {
    pub hot_nb: u64,
    pub dot_nb: u64,
    pub buf_nb: u64,
    pub debuf_nb: u64,
    pub hot_txt: Vec<String>,
    pub dot_txt: Vec<String>,
    pub buf_txt: Vec<String>,
    pub debuf_txt: Vec<String>,
}
impl CharacterFightInfo {
    /// Output: hot, dot, buf, debuf
    pub fn get_hot_and_buf_nbs_txts(all_effects: &Vec<GameAtkEffects>) -> HotsBufs {
        let mut hots_bufs = HotsBufs::default();
        for e in all_effects {
            if e.all_atk_effects.nb_turns < 2 {
                continue;
            }
            let txt = Self::get_hot_and_buf_texts(&e.all_atk_effects);
            if effect::is_hot(
                &e.all_atk_effects.effect_type,
                &e.all_atk_effects.stats_name,
                e.all_atk_effects.value,
            ) {
                hots_bufs.hot_nb += 1;
                hots_bufs.hot_txt.push(txt);
            } else if e.all_atk_effects.stats_name == HP {
                hots_bufs.dot_nb += 1;
                hots_bufs.dot_txt.push(txt);
            } else if e.all_atk_effects.value > 0 {
                hots_bufs.buf_nb += 1;
                hots_bufs.buf_txt.push(txt);
            } else {
                hots_bufs.debuf_nb += 1;
                hots_bufs.debuf_txt.push(txt);
            }
        }
        hots_bufs
    }

    fn get_hot_and_buf_texts(ep: &EffectParam) -> String {
        if ep.stats_name.is_empty() {
            format!("{}: {}", ep.effect_type, ep.value)
        } else {
            format!("{}-{}: {}", ep.effect_type, ep.stats_name, ep.value)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        character_mod::fight_information::{CharacterFightInfo, HotsBufs},
        players_manager::GameAtkEffects,
        testing_effect::{
            build_buf_effect_individual, build_debuf_effect_individual,
            build_dmg_effect_individual, build_dot_effect_individual, build_hot_effect_individual,
        },
    };

    #[test]
    fn unit_get_hot_and_buf_nbs() {
        let result = CharacterFightInfo::get_hot_and_buf_nbs_txts(&vec![]);
        assert_eq!(result, HotsBufs::default());
        let mut all_effects: Vec<GameAtkEffects> = vec![];
        // add a 1-turn-effect
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_dmg_effect_individual(),
            ..Default::default()
        });
        let result = CharacterFightInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(result, HotsBufs::default());
        // add a 2-turn-effect HOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_hot_effect_individual(),
            ..Default::default()
        });
        let result = CharacterFightInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 0,
                buf_nb: 0,
                debuf_nb: 0,
                hot_txt: vec!["Changement par valeur-HP: 30".to_owned()],
                dot_txt: vec![],
                buf_txt: vec![],
                debuf_txt: vec![]
            }
        );
        // add a 3-turn-effect DOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_dot_effect_individual(),
            ..Default::default()
        });
        let result = CharacterFightInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                buf_nb: 0,
                debuf_nb: 0,
                hot_txt: vec!["Changement par valeur-HP: 30".to_owned()],
                dot_txt: vec!["Changement par valeur-HP: -20".to_owned()],
                buf_txt: vec![],
                debuf_txt: vec![]
            }
        );
        // add a 3-turn-effect DOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_buf_effect_individual(),
            ..Default::default()
        });
        let result = CharacterFightInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                buf_nb: 1,
                debuf_nb: 0,
                hot_txt: vec!["Changement par valeur-HP: 30".to_owned()],
                dot_txt: vec!["Changement par valeur-HP: -20".to_owned()],
                buf_txt: vec!["Changement par valeur-Magic armor: 20".to_owned()],
                debuf_txt: vec![]
            }
        );
        // add a 3-turn-effect DOT
        all_effects.push(GameAtkEffects {
            all_atk_effects: build_debuf_effect_individual(),
            ..Default::default()
        });
        let result = CharacterFightInfo::get_hot_and_buf_nbs_txts(&all_effects);
        assert_eq!(
            result,
            HotsBufs {
                hot_nb: 1,
                dot_nb: 1,
                buf_nb: 1,
                debuf_nb: 1,
                hot_txt: vec!["Changement par valeur-HP: 30".to_owned()],
                dot_txt: vec!["Changement par valeur-HP: -20".to_owned()],
                buf_txt: vec!["Changement par valeur-Magic armor: 20".to_owned()],
                debuf_txt: vec!["Changement par valeur-Magic armor: -20".to_owned()]
            }
        );
    }
}
