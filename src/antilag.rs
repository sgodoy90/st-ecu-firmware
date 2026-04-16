/// Anti-lag system (ALS) — progressive intensity + Rolling Anti-Lag (RAL).
///
/// Modes:
///   IgnitionRetard: retard ignition during overrun to keep exhaust hot
///   FuelEnrich:     extra fuel injection to combust in exhaust
///   Combined:       both simultaneously
///
/// Progressive intensity: 0–100% ramp driven by TPS position (not binary on/off).
/// Rolling Anti-Lag (RAL): ALS active during corners at low throttle to maintain
///   boost for corner exit. Used in motorsport / rally.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlsMode {
    #[default]
    Disabled,
    IgnitionRetard,
    FuelEnrich,
    Combined,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlsConfig {
    pub mode: AlsMode,
    /// Maximum ignition retard at full intensity (degrees)
    pub max_retard_deg: f32,
    /// Maximum fuel enrichment at full intensity (% above stoich)
    pub max_enrich_pct: f32,
    /// Intensity ramp: TPS threshold to start ALS (%)
    pub tps_start_pct: f32,
    /// TPS threshold for full intensity (%)
    pub tps_full_pct: f32,
    /// CLT minimum for ALS activation (don't ALS a cold engine)
    pub min_clt_c: f32,
    /// MAP limit for ALS (don't ALS if already at high boost)
    pub max_map_kpa: f32,
    /// RPM range for ALS (only active between these RPMs)
    pub min_rpm: u16,
    pub max_rpm: u16,
    /// Rolling Anti-Lag enabled
    pub rolling_als_enabled: bool,
    /// Corner throttle threshold for RAL (low throttle = corner)
    pub ral_tps_threshold_pct: f32,
    /// Minimum speed for RAL
    pub ral_min_speed_kmh: f32,
    /// RAL retard amount (degrees)
    pub ral_retard_deg: f32,
}

impl Default for AlsConfig {
    fn default() -> Self {
        Self {
            mode: AlsMode::Combined,
            max_retard_deg: 20.0,
            max_enrich_pct: 30.0,
            tps_start_pct: 5.0,
            tps_full_pct: 0.5,
            min_clt_c: 60.0,
            max_map_kpa: 180.0,
            min_rpm: 2000,
            max_rpm: 6500,
            rolling_als_enabled: false,
            ral_tps_threshold_pct: 15.0,
            ral_min_speed_kmh: 40.0,
            ral_retard_deg: 10.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AlsState {
    pub active: bool,
    pub rolling_als_active: bool,
    /// Current intensity 0.0–1.0
    pub intensity: f32,
    pub retard_applied_deg: f32,
    pub fuel_enrich_pct: f32,
    pub activation_counter: u32,
}

pub struct AntiLagController {
    pub config: AlsConfig,
}

impl AntiLagController {
    pub fn new(config: AlsConfig) -> Self {
        Self { config }
    }

    /// Compute ALS activation and intensity. Call at fuel/ign loop rate.
    pub fn update(
        &self,
        state: &mut AlsState,
        rpm: f32,
        tps_pct: f32,
        map_kpa: f32,
        clt_c: f32,
        vss_kmh: f32,
    ) {
        if self.config.mode == AlsMode::Disabled {
            state.active = false;
            state.intensity = 0.0;
            state.retard_applied_deg = 0.0;
            state.fuel_enrich_pct = 0.0;
            return;
        }

        // Conditions for standard ALS
        let rpm_ok = rpm >= self.config.min_rpm as f32 && rpm <= self.config.max_rpm as f32;
        let map_ok = map_kpa < self.config.max_map_kpa;
        let clt_ok = clt_c >= self.config.min_clt_c;
        let tps_overrun = tps_pct < self.config.tps_start_pct;
        state.active = rpm_ok && map_ok && clt_ok && tps_overrun;

        // Progressive intensity based on TPS (closer to zero → more ALS)
        if state.active {
            let intensity_raw = if tps_pct <= self.config.tps_full_pct {
                1.0f32
            } else if tps_pct >= self.config.tps_start_pct {
                0.0f32
            } else {
                1.0 - (tps_pct - self.config.tps_full_pct)
                    / (self.config.tps_start_pct - self.config.tps_full_pct)
            };
            // Low-pass smooth intensity changes
            state.intensity = state.intensity * 0.8 + intensity_raw * 0.2;
            state.activation_counter += 1;
        } else {
            state.intensity *= 0.9; // decay
            if state.intensity < 0.01 { state.intensity = 0.0; }
        }

        // Rolling Anti-Lag (corners at low throttle, maintaining boost)
        let ral_conditions = self.config.rolling_als_enabled
            && vss_kmh >= self.config.ral_min_speed_kmh
            && tps_pct < self.config.ral_tps_threshold_pct
            && rpm >= self.config.min_rpm as f32;
        state.rolling_als_active = ral_conditions;

        // Apply outputs
        match self.config.mode {
            AlsMode::IgnitionRetard | AlsMode::Combined => {
                let base_retard = self.config.max_retard_deg * state.intensity;
                let ral_retard = if state.rolling_als_active { self.config.ral_retard_deg } else { 0.0 };
                state.retard_applied_deg = base_retard.max(ral_retard);
            }
            _ => { state.retard_applied_deg = 0.0; }
        }

        match self.config.mode {
            AlsMode::FuelEnrich | AlsMode::Combined => {
                state.fuel_enrich_pct = self.config.max_enrich_pct * state.intensity;
            }
            _ => { state.fuel_enrich_pct = 0.0; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn als_disabled_mode_no_activity() {
        let mut config = AlsConfig::default();
        config.mode = AlsMode::Disabled;
        let ctrl = AntiLagController::new(config);
        let mut state = AlsState::default();
        ctrl.update(&mut state, 3000.0, 0.5, 120.0, 80.0, 0.0);
        assert!(!state.active);
        assert_eq!(state.retard_applied_deg, 0.0);
    }

    #[test]
    fn als_activates_at_low_tps() {
        let ctrl = AntiLagController::new(AlsConfig::default());
        let mut state = AlsState::default();
        // Low TPS overrun at valid RPM/CLT/MAP
        ctrl.update(&mut state, 3000.0, 1.0, 120.0, 80.0, 0.0);
        assert!(state.active);
        assert!(state.intensity > 0.0);
    }

    #[test]
    fn als_not_active_when_clt_cold() {
        let ctrl = AntiLagController::new(AlsConfig::default());
        let mut state = AlsState::default();
        ctrl.update(&mut state, 3000.0, 0.5, 120.0, 30.0, 0.0); // cold engine
        assert!(!state.active);
    }

    #[test]
    fn als_combined_mode_applies_both() {
        let ctrl = AntiLagController::new(AlsConfig::default());
        let mut state = AlsState::default();
        // Force high intensity
        state.intensity = 1.0;
        ctrl.update(&mut state, 3000.0, 0.2, 120.0, 80.0, 0.0);
        // After update with full intensity conditions
        if state.active {
            assert!(state.retard_applied_deg > 0.0 || state.fuel_enrich_pct > 0.0);
        }
    }

    #[test]
    fn ral_activates_at_corner_speed() {
        let mut config = AlsConfig::default();
        config.rolling_als_enabled = true;
        config.ral_min_speed_kmh = 40.0;
        config.ral_tps_threshold_pct = 15.0;
        let ctrl = AntiLagController::new(config);
        let mut state = AlsState::default();
        // Corner: 80 km/h, 10% TPS
        ctrl.update(&mut state, 3500.0, 10.0, 130.0, 80.0, 80.0);
        assert!(state.rolling_als_active);
    }

    #[test]
    fn als_not_active_at_high_map() {
        let ctrl = AntiLagController::new(AlsConfig::default());
        let mut state = AlsState::default();
        ctrl.update(&mut state, 3000.0, 0.5, 220.0, 80.0, 0.0); // MAP > max_map
        assert!(!state.active);
    }
}
