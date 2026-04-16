const ACTIVATION_DELAY_CS: u16 = 20;
const MAX_ACTIVE_DURATION_CS: u16 = 900;
const MIN_EGT_C: f32 = 780.0;
const MAX_ENGINE_DEMAND_PCT: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotationalIdleSample {
    pub active: bool,
    pub armed: bool,
    pub cut_pct: u8,
    pub timer_cs: u16,
    pub active_cylinders: u8,
    pub gate_reason_code: u8,
    pub sync_guard_events: u8,
}

#[derive(Debug, Clone)]
pub struct RotationalIdleRuntime {
    pub enabled: bool,
    active: bool,
    armed: bool,
    activation_delay_cs: u16,
    timer_cs: u16,
    cut_pct: u8,
    active_cylinders: u8,
    gate_reason_code: u8,
    sync_guard_events: u8,
    last_sync_loss_counter: u8,
}

impl Default for RotationalIdleRuntime {
    fn default() -> Self {
        Self {
            enabled: true,
            active: false,
            armed: false,
            activation_delay_cs: 0,
            timer_cs: 0,
            cut_pct: 0,
            active_cylinders: 0,
            gate_reason_code: 1, // disabled until first successful arm
            sync_guard_events: 0,
            last_sync_loss_counter: 0,
        }
    }
}

impl RotationalIdleRuntime {
    pub fn tick(
        &mut self,
        sample_counter: u32,
        egt_c: f32,
        engine_demand_pct: f32,
        sync_loss_counter: u8,
        protection_active: bool,
    ) -> RotationalIdleSample {
        let sync_delta = sync_loss_counter.wrapping_sub(self.last_sync_loss_counter);
        self.last_sync_loss_counter = sync_loss_counter;

        if !self.enabled {
            self.deactivate(1); // disabled
            return self.sample();
        }

        if protection_active {
            self.deactivate(10); // protection_active
            return self.sample();
        }

        if sync_delta > 0 {
            self.sync_guard_events = self.sync_guard_events.saturating_add(sync_delta);
            self.deactivate(9); // sync_guard
            return self.sample();
        }

        if egt_c < MIN_EGT_C {
            self.deactivate(3); // below_egt_threshold
            return self.sample();
        }

        if engine_demand_pct > MAX_ENGINE_DEMAND_PCT {
            self.deactivate(4); // above_engine_demand_threshold
            return self.sample();
        }

        self.armed = true;
        self.gate_reason_code = 0; // none

        if !self.active {
            self.activation_delay_cs = self.activation_delay_cs.saturating_add(2);
            if self.activation_delay_cs >= ACTIVATION_DELAY_CS {
                self.active = true;
                self.timer_cs = 0;
                self.cut_pct = 24;
                self.active_cylinders = 2;
            }
            return self.sample();
        }

        self.timer_cs = self.timer_cs.saturating_add(2);
        let phase = ((sample_counter / 7) % 4) as u8;
        self.active_cylinders = if phase % 2 == 0 { 2 } else { 3 };
        self.cut_pct = match phase {
            0 => 26,
            1 => 32,
            2 => 38,
            _ => 30,
        };

        if self.timer_cs >= MAX_ACTIVE_DURATION_CS {
            self.deactivate(8); // timeout
        }

        self.sample()
    }

    fn deactivate(&mut self, gate_reason_code: u8) {
        self.active = false;
        self.armed = false;
        self.activation_delay_cs = 0;
        self.timer_cs = 0;
        self.cut_pct = 0;
        self.active_cylinders = 0;
        self.gate_reason_code = gate_reason_code;
    }

    fn sample(&self) -> RotationalIdleSample {
        RotationalIdleSample {
            active: self.active,
            armed: self.armed,
            cut_pct: self.cut_pct,
            timer_cs: self.timer_cs,
            active_cylinders: self.active_cylinders,
            gate_reason_code: self.gate_reason_code,
            sync_guard_events: self.sync_guard_events,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RotationalIdleRuntime;

    #[test]
    fn arms_and_activates_when_egt_and_demand_are_valid() {
        let mut runtime = RotationalIdleRuntime::default();
        let mut saw_active = false;

        for tick in 0..40 {
            let sample = runtime.tick(tick, 840.0, 4.0, 0, false);
            if sample.active {
                saw_active = true;
                break;
            }
        }

        assert!(saw_active, "rotational idle never became active");
    }

    #[test]
    fn sync_loss_event_forces_sync_guard_gate_and_counter() {
        let mut runtime = RotationalIdleRuntime::default();
        // Prime until active
        for tick in 0..30 {
            runtime.tick(tick, 830.0, 3.0, 0, false);
        }
        let sample = runtime.tick(31, 830.0, 3.0, 1, false);

        assert!(!sample.active);
        assert_eq!(sample.gate_reason_code, 9);
        assert!(sample.sync_guard_events > 0);
    }

    #[test]
    fn times_out_after_max_runtime_window() {
        let mut runtime = RotationalIdleRuntime::default();
        let mut timeout_gate_seen = false;

        for tick in 0..520 {
            let sample = runtime.tick(tick, 850.0, 3.0, 0, false);
            if sample.gate_reason_code == 8 {
                timeout_gate_seen = true;
                break;
            }
        }

        assert!(timeout_gate_seen, "expected timeout gate reason");
    }
}
