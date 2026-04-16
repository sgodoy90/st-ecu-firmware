/// Traction control — slip PID + 3 preset slots + ETB/spark cut.
///
/// slip_error = target_slip - actual_slip → PID → ETB position cut + spark retard
/// Presets: Dry / Wet / Gravel (or user-named)

pub const MAX_TC_PRESETS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TcPreset {
    pub enabled: bool,
    pub name: [u8; 16], // ASCII
    /// Target wheel slip (%)
    pub slip_target_pct: f32,
    /// PID gains
    pub pid_p: f32,
    pub pid_i: f32,
    pub pid_d: f32,
    /// Maximum spark retard from TC (degrees)
    pub spark_retard_max_deg: f32,
    /// Maximum ETB cut (0–100%)
    pub etb_cut_max_pct: f32,
    /// Minimum speed to engage TC (km/h)
    pub min_speed_kmh: f32,
    /// Maximum TC intervention beyond which TC disables to prevent stall
    pub max_intervention_pct: f32,
}

impl TcPreset {
    pub fn dry() -> Self {
        let mut name = [0u8; 16];
        name[..3].copy_from_slice(b"Dry");
        Self {
            enabled: true, name,
            slip_target_pct: 5.0,
            pid_p: 3.0, pid_i: 0.8, pid_d: 0.3,
            spark_retard_max_deg: 8.0,
            etb_cut_max_pct: 20.0,
            min_speed_kmh: 10.0,
            max_intervention_pct: 50.0,
        }
    }

    pub fn wet() -> Self {
        let mut name = [0u8; 16];
        name[..3].copy_from_slice(b"Wet");
        Self {
            enabled: true, name,
            slip_target_pct: 3.0,
            pid_p: 4.0, pid_i: 1.0, pid_d: 0.5,
            spark_retard_max_deg: 12.0,
            etb_cut_max_pct: 35.0,
            min_speed_kmh: 5.0,
            max_intervention_pct: 70.0,
        }
    }

    pub fn gravel() -> Self {
        let mut name = [0u8; 16];
        name[..6].copy_from_slice(b"Gravel");
        Self {
            enabled: true, name,
            slip_target_pct: 15.0,
            pid_p: 1.5, pid_i: 0.3, pid_d: 0.1,
            spark_retard_max_deg: 5.0,
            etb_cut_max_pct: 10.0,
            min_speed_kmh: 15.0,
            max_intervention_pct: 30.0,
        }
    }
}

impl Default for TcPreset {
    fn default() -> Self {
        Self {
            enabled: false,
            name: [0u8; 16],
            slip_target_pct: 5.0,
            pid_p: 3.0, pid_i: 0.8, pid_d: 0.3,
            spark_retard_max_deg: 8.0,
            etb_cut_max_pct: 20.0,
            min_speed_kmh: 10.0,
            max_intervention_pct: 50.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TractionState {
    pub active: bool,
    pub active_preset: u8, // 0–2
    pub slip_actual_pct: f32,
    pub slip_error: f32,
    pub pid_integral: f32,
    pub pid_error_prev: f32,
    pub spark_retard_deg: f32,
    pub etb_cut_pct: f32,
}

pub struct TractionController {
    pub presets: [TcPreset; MAX_TC_PRESETS],
}

impl Default for TractionController {
    fn default() -> Self {
        Self {
            presets: [TcPreset::dry(), TcPreset::wet(), TcPreset::gravel()],
        }
    }
}

impl TractionController {
    pub fn active_preset(&self, state: &TractionState) -> &TcPreset {
        &self.presets[state.active_preset.min(MAX_TC_PRESETS as u8 - 1) as usize]
    }

    /// Update traction control. Call at engine loop rate (~50 Hz).
    pub fn update(
        &self,
        state: &mut TractionState,
        front_speed_kmh: f32,
        rear_speed_kmh: f32,
        vss_kmh: f32,
        tc_switch_enabled: bool,
        dt_s: f32,
    ) {
        let preset = self.active_preset(state);

        if !preset.enabled || !tc_switch_enabled || vss_kmh < preset.min_speed_kmh {
            state.active = false;
            state.spark_retard_deg = 0.0;
            state.etb_cut_pct = 0.0;
            state.pid_integral *= 0.9; // decay integral
            return;
        }

        // Calculate actual slip
        let ref_speed = front_speed_kmh.max(1.0);
        state.slip_actual_pct = ((rear_speed_kmh / ref_speed) - 1.0) * 100.0;
        state.slip_error = preset.slip_target_pct - state.slip_actual_pct;

        // Activate if over-slip
        let over_slip = state.slip_actual_pct > preset.slip_target_pct + 0.5;
        state.active = over_slip;

        if !over_slip {
            // Decay intervention
            state.spark_retard_deg = (state.spark_retard_deg - 0.5).max(0.0);
            state.etb_cut_pct = (state.etb_cut_pct - 1.0).max(0.0);
            state.pid_integral *= 0.98;
            return;
        }

        // PID on slip error (negative error = over-slip → positive intervention needed)
        let error = -state.slip_error; // inverted: we cut when positive
        let p = preset.pid_p * error;
        state.pid_integral = (state.pid_integral + preset.pid_i * error * dt_s)
            .max(0.0).min(preset.max_intervention_pct / 100.0);
        let d = if dt_s > 0.0 {
            preset.pid_d * (error - state.pid_error_prev) / dt_s
        } else { 0.0 };
        state.pid_error_prev = error;

        let intervention = (p + state.pid_integral + d).max(0.0).min(1.0);

        // Apply spark retard and ETB cut proportionally
        state.spark_retard_deg = (intervention * preset.spark_retard_max_deg)
            .min(preset.spark_retard_max_deg);
        state.etb_cut_pct = (intervention * preset.etb_cut_max_pct)
            .min(preset.etb_cut_max_pct);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tc_inactive_at_low_speed() {
        let ctrl = TractionController::default();
        let mut state = TractionState::default();
        ctrl.update(&mut state, 5.0, 6.0, 4.0, true, 0.05); // below min_speed
        assert!(!state.active);
    }

    #[test]
    fn tc_cuts_on_excessive_slip() {
        let ctrl = TractionController::default();
        let mut state = TractionState::default();
        // Rear 30 km/h vs front 20 km/h = 50% slip, target = 5%
        ctrl.update(&mut state, 20.0, 30.0, 20.0, true, 0.05);
        assert!(state.active);
        assert!(state.spark_retard_deg > 0.0 || state.etb_cut_pct > 0.0);
    }

    #[test]
    fn tc_no_cut_on_target_slip() {
        let ctrl = TractionController::default();
        let mut state = TractionState::default();
        // Rear 21 km/h vs front 20 km/h = 5% slip = exactly target
        ctrl.update(&mut state, 20.0, 21.0, 20.0, true, 0.05);
        assert!(!state.active);
    }

    #[test]
    fn three_presets_distinct() {
        let ctrl = TractionController::default();
        let dry = &ctrl.presets[0];
        let wet = &ctrl.presets[1];
        let gravel = &ctrl.presets[2];
        assert!(dry.slip_target_pct != gravel.slip_target_pct);
        assert!(wet.etb_cut_max_pct > dry.etb_cut_max_pct);
        assert!(gravel.slip_target_pct > dry.slip_target_pct);
    }

    #[test]
    fn tc_disabled_by_switch() {
        let ctrl = TractionController::default();
        let mut state = TractionState::default();
        ctrl.update(&mut state, 20.0, 30.0, 20.0, false, 0.05); // switch off
        assert!(!state.active);
    }
}
