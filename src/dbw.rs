/// DBW (Drive-By-Wire) E-Throttle — dual TPS, PID + bias, 4 profiles, HRTIM H743.
///
/// Safety: if TPS1 and TPS2 diverge >5%, limp mode (10% throttle cap).
/// Profiles: Sport / Eco / Rain / Drag — each with its own pedal map.
/// H743: peak-and-hold current control via HRTIM for H-bridge motor (less heat, more precise).

pub const PEDAL_MAP_POINTS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DbwProfile {
    #[default]
    Sport,
    Eco,
    Rain,
    Drag,
}

/// Pedal-to-throttle map: 8 pedal % breakpoints → 8 throttle % targets
#[derive(Debug, Clone, Copy)]
pub struct PedalMap {
    pub pedal_pct: [f32; PEDAL_MAP_POINTS],
    pub throttle_pct: [f32; PEDAL_MAP_POINTS],
    pub aggression: f32, // 0.5 = lazy, 2.0 = aggressive
}

impl Default for PedalMap {
    fn default() -> Self {
        Self {
            pedal_pct: [0., 10., 20., 30., 40., 60., 80., 100.],
            throttle_pct: [0., 10., 20., 30., 45., 65., 85., 100.],
            aggression: 1.0,
        }
    }
}

impl PedalMap {
    pub fn sport() -> Self {
        Self {
            pedal_pct: [0., 10., 20., 30., 40., 60., 80., 100.],
            throttle_pct: [0., 15., 30., 48., 65., 82., 92., 100.],
            aggression: 1.4,
        }
    }

    pub fn eco() -> Self {
        Self {
            pedal_pct: [0., 10., 20., 30., 40., 60., 80., 100.],
            throttle_pct: [0., 6., 12., 20., 30., 50., 75., 100.],
            aggression: 0.7,
        }
    }

    pub fn rain() -> Self {
        Self {
            pedal_pct: [0., 10., 20., 30., 40., 60., 80., 100.],
            throttle_pct: [0., 4., 9., 15., 24., 42., 68., 100.],
            aggression: 0.6,
        }
    }

    pub fn drag() -> Self {
        // Linear 1:1 for maximum throttle authority
        Self {
            pedal_pct: [0., 10., 20., 30., 40., 60., 80., 100.],
            throttle_pct: [0., 10., 20., 30., 40., 60., 80., 100.],
            aggression: 2.0,
        }
    }

    pub fn map_pedal(&self, pedal: f32) -> f32 {
        let n = PEDAL_MAP_POINTS;
        if pedal <= self.pedal_pct[0] { return self.throttle_pct[0]; }
        if pedal >= self.pedal_pct[n-1] { return self.throttle_pct[n-1]; }
        for i in 0..n-1 {
            if pedal >= self.pedal_pct[i] && pedal < self.pedal_pct[i+1] {
                let frac = (pedal - self.pedal_pct[i]) / (self.pedal_pct[i+1] - self.pedal_pct[i]);
                return self.throttle_pct[i] * (1.0 - frac) + self.throttle_pct[i+1] * frac;
            }
        }
        self.throttle_pct[n-1]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DbwConfig {
    pub pid_p: f32,
    pub pid_i: f32,
    pub pid_d: f32,
    pub pid_min: f32,
    pub pid_max: f32,
    pub integral_limit: f32,
    /// TPS1 vs TPS2 plausibility tolerance (%)
    pub tps_plausibility_threshold_pct: f32,
    /// Limp mode throttle cap (%)
    pub limp_throttle_cap_pct: f32,
    /// Active profile
    pub active_profile: DbwProfile,
}

impl Default for DbwConfig {
    fn default() -> Self {
        Self {
            pid_p: 0.8,
            pid_i: 0.05,
            pid_d: 0.15,
            pid_min: -1.0,
            pid_max: 1.0,
            integral_limit: 0.5,
            tps_plausibility_threshold_pct: 5.0,
            limp_throttle_cap_pct: 10.0,
            active_profile: DbwProfile::Sport,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DbwState {
    pub target_pct: f32,
    pub actual_pct: f32,
    pub actual2_pct: f32,
    pub pid_integral: f32,
    pub pid_error_prev: f32,
    pub motor_duty: f32,    // -1.0 to 1.0 (direction + magnitude)
    pub limp_mode: bool,
    pub tps_error: bool,
    pub active_profile: DbwProfile,
    // H743 peak-and-hold state
    pub peak_phase: bool,
    pub peak_timer_us: u32,
}

pub struct DbwController {
    pub config: DbwConfig,
    pub maps: [PedalMap; 4], // Sport, Eco, Rain, Drag
}

impl Default for DbwController {
    fn default() -> Self {
        Self {
            config: DbwConfig::default(),
            maps: [PedalMap::sport(), PedalMap::eco(), PedalMap::rain(), PedalMap::drag()],
        }
    }
}

impl DbwController {
    pub fn active_map(&self, state: &DbwState) -> &PedalMap {
        match state.active_profile {
            DbwProfile::Sport => &self.maps[0],
            DbwProfile::Eco   => &self.maps[1],
            DbwProfile::Rain  => &self.maps[2],
            DbwProfile::Drag  => &self.maps[3],
        }
    }

    /// Update DBW control loop. Call at high-rate task (1 kHz on target).
    pub fn update(
        &self,
        state: &mut DbwState,
        tps1_pct: f32,
        tps2_pct: f32,
        pedal_pct: f32,
        tc_etb_cut_pct: f32, // from traction control (0–100)
        dt_s: f32,
    ) {
        // TPS plausibility check
        let tps_diverge = (tps1_pct - tps2_pct).abs();
        if tps_diverge > self.config.tps_plausibility_threshold_pct {
            state.tps_error = true;
            state.limp_mode = true;
        }
        state.actual_pct = tps1_pct;
        state.actual2_pct = tps2_pct;

        // Determine target from pedal map
        let map = self.active_map(state);
        let mapped_target = map.map_pedal(pedal_pct);

        // Apply TC cut
        let tc_cut_factor = 1.0 - (tc_etb_cut_pct / 100.0).min(1.0);
        let uncapped_target = mapped_target * tc_cut_factor;

        state.target_pct = if state.limp_mode {
            uncapped_target.min(self.config.limp_throttle_cap_pct)
        } else {
            uncapped_target
        };

        // PID
        let error = state.target_pct - state.actual_pct;
        let p = self.config.pid_p * error;
        state.pid_integral = (state.pid_integral + self.config.pid_i * error * dt_s)
            .max(-self.config.integral_limit)
            .min(self.config.integral_limit);
        let d = if dt_s > 0.0 {
            self.config.pid_d * (error - state.pid_error_prev) / dt_s
        } else { 0.0 };
        state.pid_error_prev = error;

        state.motor_duty = (p + state.pid_integral + d)
            .max(self.config.pid_min)
            .min(self.config.pid_max);
    }

    /// H743-only: set peak-and-hold parameters for H-bridge motor.
    /// peak_us: duration at full current (µs), then switch to hold_duty.
    pub fn set_hrtim_peak_hold(state: &mut DbwState, peak_duty: f32, hold_duty: f32, peak_us: u32) {
        state.peak_phase = true;
        state.peak_timer_us = peak_us;
        // Firmware: write peak_duty to HRTIM compare, set timer to switch to hold_duty after peak_us
        let _ = (peak_duty, hold_duty); // register writes on target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pedal_map_sport_more_aggressive() {
        let sport = PedalMap::sport();
        let eco = PedalMap::eco();
        // At 30% pedal, sport should give more throttle
        let sport_t = sport.map_pedal(30.0);
        let eco_t = eco.map_pedal(30.0);
        assert!(sport_t > eco_t, "sport: {sport_t}, eco: {eco_t}");
    }

    #[test]
    fn drag_map_linear() {
        let drag = PedalMap::drag();
        // At 50% pedal → ~50% throttle
        let t = drag.map_pedal(50.0);
        assert!((t - 50.0).abs() < 5.0, "drag map should be near-linear, got {t}");
    }

    #[test]
    fn limp_mode_caps_throttle() {
        let ctrl = DbwController::default();
        let mut state = DbwState::default();
        state.limp_mode = true;
        ctrl.update(&mut state, 5.0, 5.0, 80.0, 0.0, 0.001);
        assert!(state.target_pct <= ctrl.config.limp_throttle_cap_pct + 0.1);
    }

    #[test]
    fn tps_divergence_triggers_limp() {
        let ctrl = DbwController::default();
        let mut state = DbwState::default();
        // TPS1 = 20%, TPS2 = 60% → >5% divergence
        ctrl.update(&mut state, 20.0, 60.0, 30.0, 0.0, 0.001);
        assert!(state.tps_error);
        assert!(state.limp_mode);
    }

    #[test]
    fn tc_cut_reduces_target() {
        let ctrl = DbwController::default();
        let mut state = DbwState::default();
        ctrl.update(&mut state, 50.0, 50.0, 80.0, 50.0, 0.001); // 50% TC cut
        // Target should be reduced from normal 80% pedal mapping
        let base_target = PedalMap::sport().map_pedal(80.0);
        assert!(state.target_pct < base_target);
    }
}
