/// Launch control — presets 1-5, two-step, three-step, rolling launch.
///
/// State machine: Idle → Armed → Active → Releasing
///
/// TwoStep:    RPM limiter on clutch switch. Builds boost at launch RPM.
/// ThreeStep:  RPM limiter on launch button, step to higher RPM on release.
/// Rolling:    Slip-based engagement for rolling starts. Monitors front vs rear wheel speed.
///             slip_error = target_slip - actual_slip → PID → spark cut

pub const MAX_LAUNCH_PRESETS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LaunchMode {
    #[default]
    Disabled,
    TwoStep,
    ThreeStep,
    Rolling,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaunchPreset {
    pub enabled: bool,
    pub mode: LaunchMode,
    /// RPM limiter (two-step RPM / launch RPM)
    pub rpm_limit: u16,
    /// Three-step upper RPM (after button release)
    pub three_step_rpm_high: u16,
    /// Spark cut percentage (0–100)
    pub spark_cut_pct: u8,
    /// Fuel cut percentage (0–100)
    pub fuel_cut_pct: u8,
    /// Ignition retard during launch (degrees)
    pub retard_deg: f32,
    /// Target wheel slip for rolling launch (%)
    pub slip_target_pct: f32,
    /// Rolling launch PID gains
    pub rolling_pid_p: f32,
    pub rolling_pid_i: f32,
}

impl Default for LaunchPreset {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: LaunchMode::TwoStep,
            rpm_limit: 4500,
            three_step_rpm_high: 5500,
            spark_cut_pct: 60,
            fuel_cut_pct: 0,
            retard_deg: 15.0,
            slip_target_pct: 8.0,
            rolling_pid_p: 2.0,
            rolling_pid_i: 0.5,
        }
    }
}

impl LaunchPreset {
    pub fn preset_1() -> Self { Self { enabled: true, rpm_limit: 4000, spark_cut_pct: 50, retard_deg: 12.0, ..Default::default() } }
    pub fn preset_2() -> Self { Self { enabled: true, rpm_limit: 4500, spark_cut_pct: 60, retard_deg: 15.0, ..Default::default() } }
    pub fn preset_3() -> Self { Self { enabled: true, rpm_limit: 5000, spark_cut_pct: 70, retard_deg: 18.0, ..Default::default() } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LaunchPhase {
    #[default]
    Idle,
    Armed,
    Active,
    Releasing,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LaunchState {
    pub phase: LaunchPhase,
    pub active_preset: u8,
    pub spark_cut_active: bool,
    pub fuel_cut_active: bool,
    pub retard_active: bool,
    pub slip_error: f32,
    pub rolling_pid_integral: f32,
    pub hard_cut_flag: bool,
    /// Cut ratio: 0.0–1.0 (fraction of events cut)
    pub cut_ratio: f32,
    pub rpm_at_launch: u16,
}

pub struct LaunchController {
    pub presets: [LaunchPreset; MAX_LAUNCH_PRESETS],
}

impl Default for LaunchController {
    fn default() -> Self {
        Self {
            presets: [
                LaunchPreset::preset_1(),
                LaunchPreset::preset_2(),
                LaunchPreset::preset_3(),
                LaunchPreset::default(),
                LaunchPreset::default(),
            ],
        }
    }
}

impl LaunchController {
    pub fn active_preset(&self, state: &LaunchState) -> &LaunchPreset {
        &self.presets[state.active_preset.min(MAX_LAUNCH_PRESETS as u8 - 1) as usize]
    }

    /// Update launch state machine.
    /// clutch_pressed: true while clutch pedal is down
    /// launch_button: true while launch button held
    /// rpm: current RPM
    /// slip_pct: actual wheel slip % (rear_speed / front_speed - 1) × 100
    pub fn update(
        &self,
        state: &mut LaunchState,
        clutch_pressed: bool,
        launch_button: bool,
        rpm: f32,
        vss_kmh: f32,
        rear_speed_kmh: f32,
        dt_s: f32,
    ) {
        let preset = self.active_preset(state);
        if !preset.enabled {
            state.phase = LaunchPhase::Idle;
            state.spark_cut_active = false;
            state.fuel_cut_active = false;
            state.cut_ratio = 0.0;
            return;
        }

        match preset.mode {
            LaunchMode::Disabled => {
                state.phase = LaunchPhase::Idle;
                state.spark_cut_active = false;
            }

            LaunchMode::TwoStep => {
                match state.phase {
                    LaunchPhase::Idle => {
                        if clutch_pressed { state.phase = LaunchPhase::Armed; }
                    }
                    LaunchPhase::Armed => {
                        if !clutch_pressed { state.phase = LaunchPhase::Idle; return; }
                        // Engage at throttle application
                        state.phase = LaunchPhase::Active;
                        state.rpm_at_launch = rpm as u16;
                    }
                    LaunchPhase::Active => {
                        if !clutch_pressed {
                            state.phase = LaunchPhase::Releasing;
                            return;
                        }
                        // RPM limiter: cut spark/fuel above limit
                        if rpm > preset.rpm_limit as f32 {
                            state.hard_cut_flag = true;
                            state.spark_cut_active = preset.spark_cut_pct > 0;
                            state.fuel_cut_active = preset.fuel_cut_pct > 0;
                            state.cut_ratio = preset.spark_cut_pct as f32 / 100.0;
                        } else {
                            state.hard_cut_flag = false;
                            state.spark_cut_active = false;
                            state.fuel_cut_active = false;
                            state.cut_ratio = 0.0;
                        }
                        state.retard_active = true;
                    }
                    LaunchPhase::Releasing => {
                        state.spark_cut_active = false;
                        state.fuel_cut_active = false;
                        state.retard_active = false;
                        state.cut_ratio = 0.0;
                        if vss_kmh > 10.0 { state.phase = LaunchPhase::Idle; }
                    }
                }
            }

            LaunchMode::ThreeStep => {
                match state.phase {
                    LaunchPhase::Idle => {
                        if launch_button { state.phase = LaunchPhase::Armed; }
                    }
                    LaunchPhase::Armed => {
                        if rpm > preset.rpm_limit as f32 {
                            state.spark_cut_active = true;
                            state.cut_ratio = preset.spark_cut_pct as f32 / 100.0;
                        } else {
                            state.spark_cut_active = false;
                            state.cut_ratio = 0.0;
                        }
                        state.retard_active = true;
                        if !launch_button {
                            // Step to high RPM
                            state.phase = LaunchPhase::Active;
                        }
                    }
                    LaunchPhase::Active => {
                        if rpm > preset.three_step_rpm_high as f32 {
                            state.spark_cut_active = true;
                        } else {
                            state.spark_cut_active = false;
                        }
                        if vss_kmh > 20.0 { state.phase = LaunchPhase::Releasing; }
                    }
                    LaunchPhase::Releasing => {
                        state.spark_cut_active = false;
                        state.cut_ratio = 0.0;
                        state.retard_active = false;
                        state.phase = LaunchPhase::Idle;
                    }
                }
            }

            LaunchMode::Rolling => {
                // Slip-based engagement
                let slip_actual = if vss_kmh > 5.0 {
                    (rear_speed_kmh / vss_kmh - 1.0) * 100.0
                } else { 0.0 };
                state.slip_error = preset.slip_target_pct - slip_actual;

                // PID on slip error → spark cut ratio
                state.rolling_pid_integral = (state.rolling_pid_integral
                    + preset.rolling_pid_i * state.slip_error * dt_s)
                    .max(-1.0).min(1.0);
                let pid_output = preset.rolling_pid_p * state.slip_error + state.rolling_pid_integral;
                state.cut_ratio = (-pid_output).max(0.0).min(1.0); // cut more when over-slip
                state.spark_cut_active = state.cut_ratio > 0.05;
                state.phase = if launch_button { LaunchPhase::Active } else { LaunchPhase::Idle };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_step_arms_on_clutch() {
        let ctrl = LaunchController::default();
        let mut state = LaunchState::default();
        ctrl.update(&mut state, true, false, 0.0, 0.0, 0.0, 0.05);
        assert_eq!(state.phase, LaunchPhase::Armed);
    }

    #[test]
    fn two_step_disarms_without_clutch() {
        let ctrl = LaunchController::default();
        let mut state = LaunchState { phase: LaunchPhase::Armed, ..Default::default() };
        ctrl.update(&mut state, false, false, 0.0, 0.0, 0.0, 0.05);
        assert_eq!(state.phase, LaunchPhase::Idle);
    }

    #[test]
    fn two_step_cuts_above_rpm_limit() {
        let ctrl = LaunchController::default();
        let mut state = LaunchState { phase: LaunchPhase::Active, ..Default::default() };
        let preset = ctrl.active_preset(&state);
        let limit = preset.rpm_limit as f32;
        ctrl.update(&mut state, true, false, limit + 100.0, 0.0, 0.0, 0.05);
        assert!(state.spark_cut_active);
        assert!(state.hard_cut_flag);
    }

    #[test]
    fn rolling_launch_computes_slip_error() {
        let mut ctrl = LaunchController::default();
        ctrl.presets[0].mode = LaunchMode::Rolling;
        ctrl.presets[0].slip_target_pct = 10.0;
        let mut state = LaunchState::default();
        // 15% slip: rear 115 km/h vs front 100 km/h
        ctrl.update(&mut state, false, true, 100.0, 100.0, 115.0, 0.05);
        // slip_actual = 15%, target = 10% → error = -5 (over-slip)
        assert!(state.slip_error < 0.0, "expected negative slip error (over-slip)");
        assert!(state.spark_cut_active, "should cut spark when over-slip");
    }

    #[test]
    fn preset_1_defaults_are_valid() {
        let p = LaunchPreset::preset_1();
        assert!(p.enabled);
        assert!(p.rpm_limit > 0);
    }

    #[test]
    fn five_presets_created() {
        let ctrl = LaunchController::default();
        assert_eq!(ctrl.presets.len(), MAX_LAUNCH_PRESETS);
    }
}
