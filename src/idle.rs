/// Idle speed control — PID + ignition correction + dashpot + DSG step.
///
/// Idle ignition correction runs at the ignition callback rate (every tooth event),
/// NOT at the idle task rate. This gives fast response for idle stability.

/// Idle configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdleConfig {
    /// PID gains for idle valve position
    pub pid_p: f32,
    pub pid_i: f32,
    pub pid_d: f32,
    /// PID output limits (valve duty 0–1)
    pub pid_min: f32,
    pub pid_max: f32,
    /// Maximum integral accumulation (anti-windup)
    pub integral_limit: f32,
    /// Idle ignition correction gain (degrees per RPM error)
    pub ign_correction_gain: f32,
    /// Max ignition correction (±degrees)
    pub ign_correction_max_deg: f32,
    /// Dashpot: extra valve opening when decelerating to idle (0–1)
    pub dashpot_duty: f32,
    /// Dashpot decay rate per cycle (0–1)
    pub dashpot_decay: f32,
    /// RPM dead-band around target (below this, no correction)
    pub rpm_deadband: f32,
    /// DSG/dual-clutch step (extra RPM to add when drive gear engaged)
    pub dsg_step_rpm: u16,
    pub dsg_detection_enabled: bool,
    /// Idle warmup target table: 8 CLT points → target RPM
    pub warmup_clt: [f32; 8],
    pub warmup_target_rpm: [u16; 8],
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            pid_p: 0.4,
            pid_i: 0.02,
            pid_d: 0.1,
            pid_min: 0.0,
            pid_max: 1.0,
            integral_limit: 0.5,
            ign_correction_gain: 0.02,
            ign_correction_max_deg: 5.0,
            dashpot_duty: 0.3,
            dashpot_decay: 0.95,
            rpm_deadband: 30.0,
            dsg_step_rpm: 150,
            dsg_detection_enabled: false,
            warmup_clt: [-20.0, 0.0, 20.0, 40.0, 60.0, 70.0, 80.0, 90.0],
            warmup_target_rpm: [1400, 1300, 1200, 1100, 1000, 950, 900, 850],
        }
    }
}

/// Idle controller state
#[derive(Debug, Clone, Copy, Default)]
pub struct IdleState {
    pub target_rpm: u16,
    pub valve_duty: f32,
    pub pid_integral: f32,
    pub pid_error_prev: f32,
    pub ign_correction_deg: f32,
    pub in_idle_control: bool,
    pub dashpot_active: bool,
    pub dashpot_duty_current: f32,
    pub dsg_step_active: bool,
}

pub struct IdleController {
    pub config: IdleConfig,
}

impl IdleController {
    pub fn new(config: IdleConfig) -> Self {
        Self { config }
    }

    /// Determine target RPM from CLT warmup table.
    pub fn target_rpm_for_clt(&self, clt_c: f32, dsg_engaged: bool) -> u16 {
        let base = interpolate_1d(
            &self.config.warmup_clt,
            &self.config.warmup_target_rpm.map(|v| v as f32),
            clt_c,
        ) as u16;
        if dsg_engaged && self.config.dsg_detection_enabled {
            base + self.config.dsg_step_rpm
        } else {
            base
        }
    }

    /// Main idle PID loop. Call at idle task rate (10–20 Hz).
    /// Returns updated valve_duty (0.0–1.0).
    pub fn update_pid(
        &self,
        state: &mut IdleState,
        rpm: f32,
        tps_pct: f32,
        dt_s: f32,
    ) {
        // Only engage idle control when throttle is closed
        state.in_idle_control = tps_pct < 2.0 && rpm < (state.target_rpm as f32 + 500.0);

        if !state.in_idle_control {
            // Dashpot: keep valve slightly open during decel
            if tps_pct < 5.0 && rpm > state.target_rpm as f32 + 200.0 {
                state.dashpot_active = true;
                state.dashpot_duty_current = self.config.dashpot_duty;
            } else {
                state.dashpot_active = false;
                state.dashpot_duty_current *= self.config.dashpot_decay;
            }
            state.valve_duty = state.dashpot_duty_current;
            return;
        }

        let error = state.target_rpm as f32 - rpm;

        // Dead-band
        if error.abs() < self.config.rpm_deadband {
            return;
        }

        // PID
        let p = self.config.pid_p * error;
        state.pid_integral = (state.pid_integral + self.config.pid_i * error * dt_s)
            .max(-self.config.integral_limit)
            .min(self.config.integral_limit);
        let d = if dt_s > 0.0 {
            self.config.pid_d * (error - state.pid_error_prev) / dt_s
        } else { 0.0 };
        state.pid_error_prev = error;

        state.valve_duty = (state.valve_duty + p + state.pid_integral + d)
            .max(self.config.pid_min)
            .min(self.config.pid_max);
    }

    /// Idle ignition correction — call every ignition event (fast rate).
    /// Returns correction in degrees (positive = advance, negative = retard).
    pub fn ign_correction(&self, state: &mut IdleState, rpm: f32) -> f32 {
        if !state.in_idle_control { return 0.0; }
        let error = state.target_rpm as f32 - rpm;
        let correction = (error * self.config.ign_correction_gain)
            .max(-self.config.ign_correction_max_deg)
            .min(self.config.ign_correction_max_deg);
        state.ign_correction_deg = correction;
        correction
    }
}

fn interpolate_1d(x_axis: &[f32], y_axis: &[f32], x: f32) -> f32 {
    let n = x_axis.len();
    if x <= x_axis[0] { return y_axis[0]; }
    if x >= x_axis[n-1] { return y_axis[n-1]; }
    for i in 0..n-1 {
        if x >= x_axis[i] && x < x_axis[i+1] {
            let frac = (x - x_axis[i]) / (x_axis[i+1] - x_axis[i]);
            return y_axis[i] * (1.0 - frac) + y_axis[i+1] * frac;
        }
    }
    y_axis[n-1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_target_warm_engine() {
        let ctrl = IdleController::new(IdleConfig::default());
        let rpm = ctrl.target_rpm_for_clt(85.0, false);
        assert!(rpm >= 800 && rpm <= 1000, "warm idle target: {rpm}");
    }

    #[test]
    fn idle_target_cold_engine() {
        let ctrl = IdleController::new(IdleConfig::default());
        let rpm = ctrl.target_rpm_for_clt(-10.0, false);
        assert!(rpm > 1200, "cold idle should be higher: {rpm}");
    }

    #[test]
    fn dsg_step_adds_rpm() {
        let mut config = IdleConfig::default();
        config.dsg_detection_enabled = true;
        let ctrl = IdleController::new(config);
        let base = ctrl.target_rpm_for_clt(85.0, false);
        let dsg = ctrl.target_rpm_for_clt(85.0, true);
        assert_eq!(dsg - base, config.dsg_step_rpm as u16);
    }

    #[test]
    fn pid_converges_toward_target() {
        let ctrl = IdleController::new(IdleConfig::default());
        let mut state = IdleState::default();
        state.target_rpm = 850;
        let rpm = 800.0;
        // Initial valve should be 0
        assert_eq!(state.valve_duty, 0.0);
        ctrl.update_pid(&mut state, rpm, 0.5, 0.05);
        // RPM is below target → valve duty should increase
        assert!(state.valve_duty > 0.0, "valve duty should open for low RPM");
    }

    #[test]
    fn ign_correction_returns_zero_when_not_idle() {
        let ctrl = IdleController::new(IdleConfig::default());
        let mut state = IdleState::default();
        state.in_idle_control = false;
        let corr = ctrl.ign_correction(&mut state, 2000.0);
        assert_eq!(corr, 0.0);
    }

    #[test]
    fn ign_correction_advances_when_rpm_low() {
        let ctrl = IdleController::new(IdleConfig::default());
        let mut state = IdleState::default();
        state.target_rpm = 850;
        state.in_idle_control = true;
        let corr = ctrl.ign_correction(&mut state, 750.0);
        // Low RPM → positive correction (advance timing for stability)
        assert!(corr > 0.0, "expected advance, got {corr}");
    }
}
