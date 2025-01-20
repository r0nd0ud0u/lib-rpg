pub fn target_info_new() -> Box<TargetInfo> {
    Box::<TargetInfo>::default()
}

#[derive(Default, Debug, Clone)]
pub struct TargetInfo {
    pub name: String,
    is_targeted: bool,
    is_boss: bool,
    is_reach_rand: bool,
}

impl TargetInfo {
    /// Getters
    pub fn get_name(&self) -> String {
        self.name.to_string()
    }
    pub fn get_is_targeted(&self) -> bool {
        self.is_targeted
    }
    pub fn get_is_boss(&self) -> bool {
        self.is_boss
    }
    pub fn get_is_reach_rand(&self) -> bool {
        self.is_reach_rand
    }
    /// Setters
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string() + "\0";
    }
    pub fn set_is_targeted(&mut self, value: bool) {
        self.is_targeted = value;
    }
    pub fn set_is_boss(&mut self, value: bool) {
        self.is_boss = value;
    }
    pub fn set_is_reach_rand(&mut self, value: bool) {
        self.is_reach_rand = value;
    }
}

#[cfg(test)]
mod tests {
    use super::TargetInfo;

    #[test]
    fn unit_get_name() {
        let mut ti = TargetInfo::default();
        ti.set_name("test");

        let result = ti.get_name();
        assert_eq!("test\0", result);
    }
}
