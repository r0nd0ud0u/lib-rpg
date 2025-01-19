#[derive(Default, Debug, Clone)]
pub struct AttaqueType2 {
    pub name: String,
    pub level: u8,
    pub mana_cost: u32,
    pub vigor_cost: u32,
    pub berseck_cost: u32,
    pub target: String,
    pub reach: String,
    pub name_photo: String,
    pub all_effects: Vec<crate::effect::EffectParam2>,
    pub form: String,
}

fn default_atk(name: String) -> Box<AttaqueType2> {
    Box::new(AttaqueType2 {
        name,
        ..Default::default()
    })
}

impl AttaqueType2 {
    fn get_level(&self) -> u8 {
        self.level
    }
    fn get_name(&self) -> String {
        self.name.to_string() + "/0"
    }
}
