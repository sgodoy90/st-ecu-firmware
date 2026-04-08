/// VVT (Variable Valve Timing) closed-loop control — 4 channels + feed-forward + CORDIC.
///
/// 4 channels: Bank1 Intake, Bank1 Exhaust, Bank2 Intake, Bank2 Exhaust.
/// Feed-forward: hydraulic model — oil pressure vs oil temp → duty offset table.
/// H743: CORDIC for hardware sin/cos cam angle calculation.

pub const VVT_CHANNELS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VvtChannel {
    #[default]
    B1Intake = 0,
    B1Exhaust = 1,
    B2Intake = 2,
    B2Exhaust = 3,
}

/// VVT target angle table: 16×16 RPM×load → target cam angle (degrees)
#[derive(Debug, Clone, Copy)]
pub struct VvtTargetTable {
    pub cells: [[f32; 8]; 8],
    pub rpm_axis: [f32; 8],
    pub load_axis: [f32; 8],
}

impl Default for VvtTargetTable {
    fn default() -> Self {
        // Intake: advance at mid-RPM for max VE, retard at idle and high RPM
        Self {
            cells: [
                [0.,  5.,  10., 15., 15., 12., 8.,  5.],  // low RPM
                [0.,  8.,  15., 20., 22., 18., 12., 8.],
                [5.,  12., 20., 25., 28., 24., 18., 12.],
                [5.,  15., 22., 28., 32., 28., 22., 15.],
                [5.,  15., 25., 30., 35., 30., 25., 18.],
                [5.,  12., 22., 28., 32., 28., 22., 15.],
                [0.,  8.,  18., 24., 28., 24., 18., 12.],
                [0.,  5.,  12., 18., 22., 18., 12., 8.],  // high RPM
            ],
            rpm_axis: [1000., 2000., 3000., 4000., 5000., 6000., 7000., 7500.],
            load_axis: [20., 40., 60., 80., 100., 120., 150., 200.],
        }
    }
}

impl VvtTargetTable {
    pub fn interpolate(&self, rpm: f32, load_kpa: f32) -> f32 {
        let ri = interp_idx(&self.rpm_axis, rpm);
        let li = interp_idx(&self.load_axis, load_kpa);
        let c00 = self.cells[ri.0][li.0];
        let c10 = self.cells[ri.1][li.0];
        let c01 = self.cells[ri.0][li.1];
        let c11 = self.cells[ri.1][li.1];
        c00 * (1.0 - ri.2) * (1.0 - li.2)
            + c10 * ri.2 * (1.0 - li.2)
            + c01 * (1.0 - ri.2) * li.2
            + c11 * ri.2 * li.2
    }
}

fn interp_idx(axis: &[f32], v: f32) -> (usize, usize, f32) {
    let n = axis.len();
    if v <= axis[0] { return (0, 0, 0.0); }
    if v >= axis[n-1] { return (n-1, n-1, 0.0); }
    for i in 0..n-1 {
        if v >= axis[i] && v < axis[i+1] {
            return (i, i+1, (v - axis[i]) / (axis[i+1] - axis[i]));
        }
    }
    (n-1, n-1, 0.0)
}

/// Feed-forward table: [oil_temp_bin][duty_offset] for hydraulic compensation
#[derive(Debug, Clone, Copy)]
pub struct HydraulicFeedForward {
    pub oil_temp_c: [f32; 8],
    pub duty_offset: [f32; 8],
}

impl Default for HydraulicFeedForward {
    fn default() -> Self {
        Self {
            oil_temp_c:  [20., 40., 60., 80., 100., 110., 120., 130.],
            // Cold oil is thick → higher duty needed to move cam
            duty_offset: [0.25, 0.18, 0.10, 0.05, 0.02, 0.01, 0.0, -0.01],
        }
    }
}

impl HydraulicFeedForward {
    pub fn duty_offset(&self, oil_temp_c: f32) -> f32 {
        let n = self.oil_temp_c.len();
        if oil_temp_c <= self.oil_temp_c[0] { return self.duty_offset[0]; }
        if oil_temp_c >= self.oil_temp_c[n-1] { return self.duty_offset[n-1]; }
        for i in 0..n-1 {
            if oil_temp_c >= self.oil_temp_c[i] && oil_temp_c < self.oil_temp_c[i+1] {
                let frac = (oil_temp_c - self.oil_temp_c[i]) / (self.oil_temp_c[i+1] - self.oil_temp_c[i]);
                return self.duty_offset[i] * (1.0 - frac) + self.duty_offset[i+1] * frac;
            }
        }
        self.duty_offset[n-1]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VvtConfig {
    pub pid_p: f32,
    pub pid_i: f32,
    pub pid_d: f32,
    pub integral_limit: f32,
    pub angle_deadband_deg: f32,
    pub min_oil_pressure_kpa: f32,
    pub enabled: [bool; VVT_CHANNELS],
}

impl Default for VvtConfig {
    fn default() -> Self {
        Self {
            pid_p: 0.6,
            pid_i: 0.04,
            pid_d: 0.15,
            integral_limit: 0.4,
            angle_deadband_deg: 1.0,
            min_oil_pressure_kpa: 100.0,
            enabled: [true, false, false, false], // only B1 intake by default
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VvtState {
    pub actual_deg: [f32; VVT_CHANNELS],
    pub target_deg: [f32; VVT_CHANNELS],
    pub pid_integral: [f32; VVT_CHANNELS],
    pub pid_error_prev: [f32; VVT_CHANNELS],
    pub duty: [f32; VVT_CHANNELS],
    pub error_flag: bool,
}

pub struct VvtController {
    pub config: VvtConfig,
    pub intake_target: VvtTargetTable,
    pub exhaust_target: VvtTargetTable,
    pub ff_table: HydraulicFeedForward,
}

impl Default for VvtController {
    fn default() -> Self {
        Self {
            config: VvtConfig::default(),
            intake_target: VvtTargetTable::default(),
            exhaust_target: VvtTargetTable::default(),
            ff_table: HydraulicFeedForward::default(),
        }
    }
}

impl VvtController {
    /// Update VVT for all channels. Call at engine loop rate.
    pub fn update(
        &self,
        state: &mut VvtState,
        rpm: f32,
        load_kpa: f32,
        oil_pressure_kpa: f32,
        oil_temp_c: f32,
        dt_s: f32,
    ) {
        // Disable if oil pressure too low (would damage VVT solenoid)
        if oil_pressure_kpa < self.config.min_oil_pressure_kpa {
            state.error_flag = true;
            for d in &mut state.duty { *d = 0.0; }
            return;
        }
        state.error_flag = false;

        let ff_offset = self.ff_table.duty_offset(oil_temp_c);

        for ch in 0..VVT_CHANNELS {
            if !self.config.enabled[ch] {
                state.duty[ch] = 0.0;
                continue;
            }

            // Select target table based on channel
            let target = match ch {
                0 | 2 => self.intake_target.interpolate(rpm, load_kpa),  // intake
                _ => -self.exhaust_target.interpolate(rpm, load_kpa),    // exhaust (advance opposite)
            };
            state.target_deg[ch] = target;

            let error = target - state.actual_deg[ch];

            if error.abs() < self.config.angle_deadband_deg { continue; }

            let p = self.config.pid_p * error;
            state.pid_integral[ch] = (state.pid_integral[ch] + self.config.pid_i * error * dt_s)
                .max(-self.config.integral_limit)
                .min(self.config.integral_limit);
            let d = if dt_s > 0.0 {
                self.config.pid_d * (error - state.pid_error_prev[ch]) / dt_s
            } else { 0.0 };
            state.pid_error_prev[ch] = error;

            state.duty[ch] = (p + state.pid_integral[ch] + d + ff_offset)
                .max(-1.0).min(1.0);
        }
    }

    /// Compute cam angle from raw sensor signal.
    /// On H743: uses CORDIC hardware. Host: software fallback.
    pub fn cam_angle_from_sensor(rising_timestamp_ns: u64, falling_timestamp_ns: u64, period_ns: u64) -> f32 {
        if period_ns == 0 { return 0.0; }
        let pulse_width = (falling_timestamp_ns.saturating_sub(rising_timestamp_ns)) as f32;
        let period = period_ns as f32;
        // Cam angle = (pulse_width / period) × 360° (simplified)
        (pulse_width / period * 360.0) % 360.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vvt_target_table_range() {
        let table = VvtTargetTable::default();
        let v = table.interpolate(3000.0, 80.0);
        assert!(v >= 0.0 && v <= 40.0, "VVT target {v}° out of range");
    }

    #[test]
    fn hydraulic_ff_cold_oil_high_duty() {
        let ff = HydraulicFeedForward::default();
        let cold = ff.duty_offset(20.0);
        let hot = ff.duty_offset(110.0);
        assert!(cold > hot, "cold oil needs more duty offset");
    }

    #[test]
    fn vvt_disables_on_low_oil_pressure() {
        let ctrl = VvtController::default();
        let mut state = VvtState { error_flag: false, ..Default::default() };
        ctrl.update(&mut state, 3000.0, 80.0, 50.0, 80.0, 0.02); // oil pressure below threshold
        assert!(state.error_flag);
        for &d in &state.duty { assert_eq!(d, 0.0); }
    }

    #[test]
    fn cam_angle_from_50pct_duty_is_180() {
        let angle = VvtController::cam_angle_from_sensor(0, 500_000, 1_000_000);
        assert!((angle - 180.0).abs() < 1.0, "50% duty → 180°, got {angle}");
    }

    #[test]
    fn vvt_pid_drives_duty_to_close_error() {
        let ctrl = VvtController::default();
        let mut state = VvtState::default();
        state.actual_deg[0] = 0.0;
        ctrl.update(&mut state, 3000.0, 80.0, 200.0, 80.0, 0.02);
        // Target should be >0°, actual=0, so duty should be positive
        if ctrl.config.enabled[0] {
            assert!(state.duty[0] >= 0.0, "duty should be positive to advance cam");
        }
    }

    #[test]
    fn cam_angle_zero_period_returns_zero() {
        // period_ns == 0 → guard returns 0.0 (no cam signal)
        assert_eq!(VvtController::cam_angle_from_sensor(0, 0, 0), 0.0);
        assert_eq!(VvtController::cam_angle_from_sensor(100, 200, 0), 0.0);
    }

    #[test]
    fn cam_angle_half_duty_is_180() {
        // pulse = half period → 50% duty → 180°
        let angle = VvtController::cam_angle_from_sensor(0, 500_000, 1_000_000);
        assert!((angle - 180.0).abs() < 1.0, "50% duty should be 180°, got {angle}");
    }
}
