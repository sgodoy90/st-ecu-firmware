/// Boost control — PID + boost-by-gear 3D table + scramble + map sets.

pub const BOOST_GEAR_TABLE_GEARS: usize = 8;
pub const BOOST_GEAR_TABLE_RPM_BINS: usize = 8;

/// Boost-by-gear table: [gear][rpm_bin] → target boost kPa
#[derive(Debug, Clone, Copy)]
pub struct BoostByGearTable {
    pub target_kpa: [[f32; BOOST_GEAR_TABLE_RPM_BINS]; BOOST_GEAR_TABLE_GEARS],
    pub rpm_axis: [f32; BOOST_GEAR_TABLE_RPM_BINS],
}

impl Default for BoostByGearTable {
    fn default() -> Self {
        // Reduce boost in lower gears for traction, full boost from gear 3+
        let base = [[120.0, 140.0, 160.0, 180.0, 200.0, 200.0, 200.0, 200.0], // gear 1 (reduced)
                    [130.0, 155.0, 175.0, 195.0, 210.0, 210.0, 210.0, 210.0], // gear 2
                    [140.0, 165.0, 190.0, 210.0, 220.0, 220.0, 220.0, 220.0], // gear 3
                    [140.0, 165.0, 195.0, 215.0, 225.0, 225.0, 225.0, 225.0], // gear 4
                    [140.0, 165.0, 195.0, 215.0, 225.0, 225.0, 225.0, 225.0], // gear 5
                    [140.0, 165.0, 195.0, 215.0, 225.0, 225.0, 225.0, 225.0], // gear 6
                    [140.0, 165.0, 195.0, 215.0, 225.0, 225.0, 225.0, 225.0], // gear 7
                    [140.0, 165.0, 195.0, 215.0, 225.0, 225.0, 225.0, 225.0]];// gear 8
        Self {
            target_kpa: base,
            rpm_axis: [1500., 2000., 2500., 3000., 3500., 4000., 4500., 5000.],
        }
    }
}

impl BoostByGearTable {
    pub fn target_for(&self, gear: u8, rpm: f32) -> f32 {
        let gear_idx = ((gear as usize).saturating_sub(1)).min(BOOST_GEAR_TABLE_GEARS - 1);
        let row = &self.target_kpa[gear_idx];
        // 1D interpolation along RPM axis
        let n = self.rpm_axis.len();
        if rpm <= self.rpm_axis[0] { return row[0]; }
        if rpm >= self.rpm_axis[n-1] { return row[n-1]; }
        for i in 0..n-1 {
            if rpm >= self.rpm_axis[i] && rpm < self.rpm_axis[i+1] {
                let frac = (rpm - self.rpm_axis[i]) / (self.rpm_axis[i+1] - self.rpm_axis[i]);
                return row[i] * (1.0 - frac) + row[i+1] * frac;
            }
        }
        row[n-1]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoostConfig {
    pub pid_p: f32,
    pub pid_i: f32,
    pub pid_d: f32,
    pub pid_min: f32,
    pub pid_max: f32,
    pub integral_limit: f32,
    pub scramble_boost_kpa: f32,
    pub scramble_duration_ms: u32,
    pub overboost_cut_kpa: f32,
    pub overboost_cut_delay_ms: u16,
    pub gear_table_enabled: bool,
}

impl Default for BoostConfig {
    fn default() -> Self {
        Self {
            pid_p: 0.5,
            pid_i: 0.05,
            pid_d: 0.1,
            pid_min: 0.0,
            pid_max: 1.0,
            integral_limit: 0.6,
            scramble_boost_kpa: 240.0,
            scramble_duration_ms: 5000,
            overboost_cut_kpa: 250.0,
            overboost_cut_delay_ms: 100,
            gear_table_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BoostState {
    pub target_kpa: f32,
    pub actual_kpa: f32,
    pub duty_pct: u8,
    pub pid_integral: f32,
    pub pid_error_prev: f32,
    pub scramble_active: bool,
    pub scramble_timer_ms: u32,
    pub overboost_flag: bool,
    pub active_map_set: u8,
}

pub struct BoostController {
    pub config: BoostConfig,
    pub gear_table: BoostByGearTable,
}

impl BoostController {
    pub fn new(config: BoostConfig) -> Self {
        Self { config, gear_table: BoostByGearTable::default() }
    }

    pub fn set_target(&self, state: &mut BoostState, gear: u8, rpm: f32) {
        state.target_kpa = if state.scramble_active {
            self.config.scramble_boost_kpa
        } else if self.config.gear_table_enabled && gear > 0 {
            self.gear_table.target_for(gear, rpm)
        } else {
            // Fallback flat target
            200.0
        };
    }

    pub fn update_pid(&self, state: &mut BoostState, map_kpa: f32, baro_kpa: f32, dt_s: f32) {
        let boost_kpa = map_kpa - baro_kpa;
        state.actual_kpa = boost_kpa;
        let error = state.target_kpa - boost_kpa;
        let p = self.config.pid_p * error;
        state.pid_integral = (state.pid_integral + self.config.pid_i * error * dt_s)
            .max(-self.config.integral_limit)
            .min(self.config.integral_limit);
        let d = if dt_s > 0.0 {
            self.config.pid_d * (error - state.pid_error_prev) / dt_s
        } else { 0.0 };
        state.pid_error_prev = error;
        let raw_duty = (p + state.pid_integral + d).max(0.0).min(1.0);
        state.duty_pct = (raw_duty * 100.0) as u8;
        // Overboost protection
        state.overboost_flag = boost_kpa > self.config.overboost_cut_kpa;
    }

    pub fn tick_scramble(&self, state: &mut BoostState, dt_ms: u32) {
        if state.scramble_active {
            if state.scramble_timer_ms <= dt_ms {
                state.scramble_active = false;
                state.scramble_timer_ms = 0;
            } else {
                state.scramble_timer_ms -= dt_ms;
            }
        }
    }

    pub fn activate_scramble(&self, state: &mut BoostState) {
        state.scramble_active = true;
        state.scramble_timer_ms = self.config.scramble_duration_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gear_table_gear1_less_than_gear3() {
        let table = BoostByGearTable::default();
        let g1 = table.target_for(1, 3000.0);
        let g3 = table.target_for(3, 3000.0);
        assert!(g1 < g3, "gear 1 should have less boost than gear 3");
    }

    #[test]
    fn scramble_times_out() {
        let ctrl = BoostController::new(BoostConfig::default());
        let mut state = BoostState::default();
        ctrl.activate_scramble(&mut state);
        assert!(state.scramble_active);
        ctrl.tick_scramble(&mut state, 5001);
        assert!(!state.scramble_active);
    }

    #[test]
    fn overboost_flag_when_above_threshold() {
        let ctrl = BoostController::new(BoostConfig::default());
        let mut state = BoostState::default();
        state.target_kpa = 200.0;
        ctrl.update_pid(&mut state, 355.0, 101.0, 0.05); // 254 kPa boost > 250 threshold
        assert!(state.overboost_flag);
    }

    #[test]
    fn pid_increases_duty_when_below_target() {
        let ctrl = BoostController::new(BoostConfig::default());
        let mut state = BoostState::default();
        state.target_kpa = 200.0;
        ctrl.update_pid(&mut state, 150.0, 101.0, 0.05); // only 49 kPa boost
        assert!(state.duty_pct > 0, "duty should be positive when below target");
    }
}
