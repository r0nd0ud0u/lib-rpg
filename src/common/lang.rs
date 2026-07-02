/// UI/content language selector. Deliberately dioxus-free — lib-rpg has no
/// dioxus dependency; the consuming app converts its own locale type into
/// this enum at the UI boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Fr,
}

#[cfg(test)]
mod tests {
    use super::Lang;

    #[test]
    fn unit_lang_default_is_en() {
        assert_eq!(Lang::default(), Lang::En);
    }
}
