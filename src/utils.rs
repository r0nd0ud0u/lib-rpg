#[cxx::bridge]
mod utils {
    extern "Rust" {
        fn build_effect_name(raw_effect: &str, stats_name: &str) -> String;
    }
}


/**
 * @brief Utils::BuildEffectName
 * Returns the concatenation of effect str and stats str
 * If the effect str name is empty => only the stats str
 * If the stats str name is empty => only the effect str
 */
pub fn build_effect_name(raw_effect: &str, stats_name: &str) -> String {
    let mut effect_name = "".to_string();
    if raw_effect.is_empty() && !stats_name.is_empty() {
        effect_name = stats_name.to_string();
    } else if !raw_effect.is_empty() && stats_name.is_empty() {
        effect_name = raw_effect.to_string();
    } else if !raw_effect.is_empty() && !stats_name.is_empty() {
        effect_name = format!("{}-{}", stats_name, raw_effect);
    }
    effect_name.to_string()
}

#[cfg(test)]
mod tests {
    use crate::utils::build_effect_name;

    #[test]
    fn build_effect_name_works() {
        // case args not empty
        let mut str = build_effect_name("effect", "stats");
        assert_eq!("stats-effect", str);
        // case effect str empty
        str = build_effect_name("", "stats");
        assert_eq!("stats", str);
        // case stats empty
        str = build_effect_name("effect", "");
        assert_eq!("effect", str);
        // case both args empty
        str = build_effect_name("", "");
        assert!(str.is_empty());
    }
}