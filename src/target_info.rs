/// Define all the parameters of target info during a round
#[derive(Default, Debug, Clone)]
pub struct TargetInfo {
    pub name: String,
    _is_targeted: bool,
    _is_boss: bool,
    _is_reach_rand: bool,
}

impl TargetInfo {}

#[cfg(test)]
mod tests {}
