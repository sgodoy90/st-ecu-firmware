#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    Unsynced,
    CrankOnly,
    FullSync,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineRuntime {
    pub rpm: f32,
    pub sync_state: SyncState,
}

impl Default for EngineRuntime {
    fn default() -> Self {
        Self {
            rpm: 0.0,
            sync_state: SyncState::Unsynced,
        }
    }
}
