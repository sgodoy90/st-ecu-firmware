#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtectionAction {
    None,
    SparkCut,
    FuelCut,
    LimpMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProtectionThreshold {
    pub warning_threshold: f32,
    pub action_threshold: f32,
    pub action: ProtectionAction,
}
