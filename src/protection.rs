/// Engine protection system — RPM/MAP/CLT/oil/AFR/EGT limits + watchdog architecture.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtectionAction {
    #[default]
    None,
    IgnitionRetard,
    SparkCut,
    FuelEnrich,
    FuelCut,
    SparkAndFuelCut,
    LimpMode,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProtectionThreshold {
    pub warning: f32,
    pub action: f32,
    pub action_type: ProtectionAction,
}

impl ProtectionThreshold {
    pub fn new(warning: f32, action: f32, action_type: ProtectionAction) -> Self {
        Self { warning, action, action_type }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProtectionConfig {
    pub rpm_max: ProtectionThreshold,
    pub map_max_kpa: ProtectionThreshold,
    pub oil_min_kpa: ProtectionThreshold,
    pub clt_max_c: ProtectionThreshold,
    pub afr_lean_limit: ProtectionThreshold,
    pub egt1_max_c: ProtectionThreshold,
    pub egt2_max_c: ProtectionThreshold,
}

impl Default for ProtectionConfig {
    fn default() -> Self {
        Self {
            rpm_max:       ProtectionThreshold::new(7200.0, 7500.0, ProtectionAction::SparkCut),
            map_max_kpa:   ProtectionThreshold::new(230.0,  250.0,  ProtectionAction::SparkAndFuelCut),
            oil_min_kpa:   ProtectionThreshold::new(150.0,  100.0,  ProtectionAction::LimpMode),
            clt_max_c:     ProtectionThreshold::new(100.0,  110.0,  ProtectionAction::FuelEnrich),
            afr_lean_limit:ProtectionThreshold::new(15.5,   16.0,   ProtectionAction::FuelEnrich),
            egt1_max_c:    ProtectionThreshold::new(900.0,  1000.0, ProtectionAction::FuelEnrich),
            egt2_max_c:    ProtectionThreshold::new(900.0,  1000.0, ProtectionAction::FuelEnrich),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProtectionState {
    pub rpm_protect: bool,
    pub map_protect: bool,
    pub oil_protect: bool,
    pub afr_protect: bool,
    pub coolant_protect: bool,
    pub egt_protect: bool,
    pub active_action: ProtectionAction,
    /// Protective fuel enrichment from EGT/CLT (multiplicative)
    pub fuel_enrich_factor: f32,
    /// Overrev spark cut counter (cylinder events to cut)
    pub overrev_cut_events: u8,
}

#[derive(Debug, Clone)]
pub struct ProtectionManager {
    pub config: ProtectionConfig,
}

impl ProtectionManager {
    pub fn new(config: ProtectionConfig) -> Self {
        Self { config }
    }

    /// Evaluate all protection conditions. Call at fuel/ign loop rate.
    /// Returns the most severe action to apply.
    pub fn evaluate(
        &self,
        state: &mut ProtectionState,
        rpm: f32,
        map_kpa: f32,
        oil_pressure_kpa: f32,
        coolant_c: f32,
        afr: f32,
        egt1_c: f32,
        egt2_c: f32,
    ) -> ProtectionAction {
        state.rpm_protect = false;
        state.map_protect = false;
        state.oil_protect = false;
        state.coolant_protect = false;
        state.afr_protect = false;
        state.egt_protect = false;
        state.fuel_enrich_factor = 1.0;

        let mut worst = ProtectionAction::None;

        // RPM overrev
        if rpm > self.config.rpm_max.action {
            state.rpm_protect = true;
            worst = Self::escalate(worst, self.config.rpm_max.action_type);
        }

        // Overboost
        if map_kpa > self.config.map_max_kpa.action {
            state.map_protect = true;
            worst = Self::escalate(worst, self.config.map_max_kpa.action_type);
        }

        // Low oil pressure
        if oil_pressure_kpa < self.config.oil_min_kpa.action {
            state.oil_protect = true;
            worst = Self::escalate(worst, self.config.oil_min_kpa.action_type);
        }

        // Overtemp coolant
        if coolant_c > self.config.clt_max_c.action {
            state.coolant_protect = true;
            worst = Self::escalate(worst, self.config.clt_max_c.action_type);
            // Protective enrichment: 5% per °C over threshold
            let over = coolant_c - self.config.clt_max_c.action;
            state.fuel_enrich_factor = (1.0 + over * 0.05).min(1.3);
        }

        // Lean AFR protection
        if afr > self.config.afr_lean_limit.action && afr < 20.0 {
            state.afr_protect = true;
            worst = Self::escalate(worst, self.config.afr_lean_limit.action_type);
            let lean_factor = (afr - self.config.afr_lean_limit.action) * 0.1;
            state.fuel_enrich_factor = (state.fuel_enrich_factor + lean_factor).min(1.3);
        }

        // EGT protection
        let max_egt = egt1_c.max(egt2_c);
        if max_egt > self.config.egt1_max_c.action {
            state.egt_protect = true;
            worst = Self::escalate(worst, self.config.egt1_max_c.action_type);
            let over = max_egt - self.config.egt1_max_c.action;
            state.fuel_enrich_factor = (state.fuel_enrich_factor + over * 0.003).min(1.4);
        }

        state.active_action = worst;
        worst
    }

    /// Return the more severe of two actions.
    fn escalate(current: ProtectionAction, new: ProtectionAction) -> ProtectionAction {
        let severity = |a: ProtectionAction| match a {
            ProtectionAction::None => 0,
            ProtectionAction::IgnitionRetard => 1,
            ProtectionAction::FuelEnrich => 2,
            ProtectionAction::SparkCut => 3,
            ProtectionAction::FuelCut => 4,
            ProtectionAction::SparkAndFuelCut => 5,
            ProtectionAction::LimpMode => 6,
            ProtectionAction::Shutdown => 7,
        };
        if severity(new) > severity(current) { new } else { current }
    }
}

// ─── Software Watchdog ────────────────────────────────────────────────────────

pub const SW_WD_TASKS: usize = 8;

/// Per-task watchdog counter. Each RTOS task must call kick() within its deadline.
#[derive(Debug, Clone, Copy, Default)]
pub struct SoftwareWatchdog {
    pub deadline_ms: [u32; SW_WD_TASKS],
    pub last_kick_ms: [u32; SW_WD_TASKS],
    pub task_name: [u8; SW_WD_TASKS], // ASCII identifier
}

impl SoftwareWatchdog {
    pub fn configure_task(&mut self, task_id: usize, deadline_ms: u32, name: u8) {
        if task_id < SW_WD_TASKS {
            self.deadline_ms[task_id] = deadline_ms;
            self.task_name[task_id] = name;
        }
    }

    pub fn kick(&mut self, task_id: usize, now_ms: u32) {
        if task_id < SW_WD_TASKS {
            self.last_kick_ms[task_id] = now_ms;
        }
    }

    /// Check all tasks. Returns true if any task missed its deadline.
    pub fn check(&self, now_ms: u32) -> bool {
        for i in 0..SW_WD_TASKS {
            if self.deadline_ms[i] == 0 { continue; }
            let elapsed = now_ms.saturating_sub(self.last_kick_ms[i]);
            if elapsed > self.deadline_ms[i] {
                return true; // task missed deadline
            }
        }
        false
    }

    /// Returns the first overdue task ID, or None.
    pub fn overdue_task(&self, now_ms: u32) -> Option<usize> {
        for i in 0..SW_WD_TASKS {
            if self.deadline_ms[i] == 0 { continue; }
            let elapsed = now_ms.saturating_sub(self.last_kick_ms[i]);
            if elapsed > self.deadline_ms[i] { return Some(i); }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protection_manager_rpm_overrev_cuts_spark() {
        let mgr = ProtectionManager::new(ProtectionConfig::default());
        let mut state = ProtectionState::default();
        let action = mgr.evaluate(&mut state, 8000.0, 100.0, 300.0, 85.0, 14.7, 800.0, 800.0);
        assert!(state.rpm_protect);
        assert!(matches!(action, ProtectionAction::SparkCut | ProtectionAction::SparkAndFuelCut));
    }

    #[test]
    fn protection_manager_overboost_cuts_both() {
        let mgr = ProtectionManager::new(ProtectionConfig::default());
        let mut state = ProtectionState::default();
        let action = mgr.evaluate(&mut state, 4000.0, 280.0, 300.0, 85.0, 14.7, 800.0, 800.0);
        assert!(state.map_protect);
        assert_eq!(action, ProtectionAction::SparkAndFuelCut);
    }

    #[test]
    fn protection_no_fault_returns_none() {
        let mgr = ProtectionManager::new(ProtectionConfig::default());
        let mut state = ProtectionState::default();
        let action = mgr.evaluate(&mut state, 3000.0, 120.0, 300.0, 85.0, 14.7, 800.0, 800.0);
        assert_eq!(action, ProtectionAction::None);
        assert_eq!(state.fuel_enrich_factor, 1.0);
    }

    #[test]
    fn protection_egt_enriches_fuel() {
        let mgr = ProtectionManager::new(ProtectionConfig::default());
        let mut state = ProtectionState::default();
        mgr.evaluate(&mut state, 4000.0, 120.0, 300.0, 85.0, 14.7, 1050.0, 800.0);
        assert!(state.egt_protect);
        assert!(state.fuel_enrich_factor > 1.0);
    }

    #[test]
    fn sw_watchdog_detects_overdue_task() {
        let mut wd = SoftwareWatchdog::default();
        wd.configure_task(0, 100, b'A'); // 100ms deadline
        wd.kick(0, 0);
        assert!(!wd.check(50));  // 50ms elapsed, ok
        assert!(wd.check(150)); // 150ms elapsed, overdue
    }

    #[test]
    fn sw_watchdog_no_fault_when_kicked() {
        let mut wd = SoftwareWatchdog::default();
        wd.configure_task(1, 50, b'B');
        wd.kick(1, 0);
        wd.kick(1, 30); // kicked at 30ms
        assert!(!wd.check(70)); // 40ms since last kick, ok
    }

    #[test]
    fn escalate_most_severe_wins() {
        let result = ProtectionManager::escalate(
            ProtectionAction::SparkCut,
            ProtectionAction::LimpMode,
        );
        assert_eq!(result, ProtectionAction::LimpMode);
    }
}
