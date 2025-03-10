//use rand::Rng;

/// * Returns the concatenation of effect str and stats str
/// * If the effect str name is empty => only the stats str
///* If the stats str name is empty => only the effect str
pub fn build_effect_name(raw_effect: &str, stats_name: &str, is_cpp: bool) -> String {
    let mut effect_name = "".to_string();
    if raw_effect.is_empty() && !stats_name.is_empty() {
        effect_name = stats_name.to_string();
    } else if !raw_effect.is_empty() && stats_name.is_empty() {
        effect_name = raw_effect.to_string();
    } else if !raw_effect.is_empty() && !stats_name.is_empty() {
        effect_name = format!("{}-{}", stats_name, raw_effect);
    }
    if is_cpp {
        effect_name.to_string() + "\0"
    } else {
        effect_name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::build_effect_name;

    #[test]
    fn unit_build_effect_name_works() {
        // case args not empty
        let mut str = build_effect_name("effect", "stats", false);
        assert_eq!("stats-effect", str);
        // case effect str empty
        str = build_effect_name("", "stats", false);
        assert_eq!("stats", str);
        // case stats empty
        str = build_effect_name("effect", "", false);
        assert_eq!("effect", str);
        // case both args empty
        str = build_effect_name("", "", false);
        assert!(str.is_empty());
    }
}
