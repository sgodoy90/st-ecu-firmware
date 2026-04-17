const ACTIVATION_DELAY_CS: u16 = 20;
const MAX_ACTIVE_DURATION_CS: u16 = 900;
const MIN_EGT_C: f32 = 780.0;
const MAX_ENGINE_DEMAND_PCT: f32 = 8.0;
const GATE_HISTOGRAM_SLOTS: usize = 16;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotationalIdleDiagnostics {
    pub activation_count: u16,
    pub timeout_count: u16,
    pub sync_guard_events_total: u16,
    pub total_active_time_cs: u32,
    pub last_gate_reason_code: u8,
    pub gate_reason_histogram: [u16; GATE_HISTOGRAM_SLOTS],
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
    last_sync_loss_counter: u8,
    scheduler_slot: u8,
    activation_count: u16,
    timeout_count: u16,
    sync_guard_events_total: u16,
    total_active_time_cs: u32,
    last_gate_reason_code: u8,
    gate_reason_histogram: [u16; GATE_HISTOGRAM_SLOTS],
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
            last_sync_loss_counter: 0,
            scheduler_slot: 0,
            activation_count: 0,
            timeout_count: 0,
            sync_guard_events_total: 0,
            total_active_time_cs: 0,
            last_gate_reason_code: 1,
            gate_reason_histogram: [0; GATE_HISTOGRAM_SLOTS],
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
            self.apply_gate(1); // disabled
            return self.sample();
        }

        if protection_active {
            self.apply_gate(10); // protection_active
            return self.sample();
        }

        if sync_delta > 0 {
            self.sync_guard_events_total = self
                .sync_guard_events_total
                .saturating_add(sync_delta as u16);
            self.apply_gate(9); // sync_guard
            return self.sample();
        }

        if egt_c < MIN_EGT_C {
            self.apply_gate(3); // below_egt_threshold
            return self.sample();
        }

        if engine_demand_pct > MAX_ENGINE_DEMAND_PCT {
            self.apply_gate(4); // above_engine_demand_threshold
            return self.sample();
        }

        self.armed = true;
        self.gate_reason_code = 0; // none

        if !self.active {
            self.activation_delay_cs = self.activation_delay_cs.saturating_add(2);
            if self.activation_delay_cs >= ACTIVATION_DELAY_CS {
                self.activate(sample_counter, egt_c, engine_demand_pct);
            }
            return self.sample();
        }

        self.timer_cs = self.timer_cs.saturating_add(2);
        self.total_active_time_cs = self.total_active_time_cs.saturating_add(2);
        self.advance_scheduler(sample_counter, egt_c, engine_demand_pct);

        if self.timer_cs >= MAX_ACTIVE_DURATION_CS {
            self.timeout_count = self.timeout_count.saturating_add(1);
            self.apply_gate(8); // timeout
        }

        self.sample()
    }

    pub fn diagnostics(&self) -> RotationalIdleDiagnostics {
        RotationalIdleDiagnostics {
            activation_count: self.activation_count,
            timeout_count: self.timeout_count,
            sync_guard_events_total: self.sync_guard_events_total,
            total_active_time_cs: self.total_active_time_cs,
            last_gate_reason_code: self.last_gate_reason_code,
            gate_reason_histogram: self.gate_reason_histogram,
        }
    }

    fn activate(&mut self, sample_counter: u32, egt_c: f32, engine_demand_pct: f32) {
        self.active = true;
        self.timer_cs = 0;
        self.activation_count = self.activation_count.saturating_add(1);
        self.scheduler_slot = (sample_counter % 6) as u8;
        self.advance_scheduler(sample_counter, egt_c, engine_demand_pct);
    }

    fn advance_scheduler(&mut self, sample_counter: u32, egt_c: f32, engine_demand_pct: f32) {
        self.scheduler_slot = ((sample_counter / 5) % 6) as u8;

        let egt_bonus = ((egt_c - MIN_EGT_C).max(0.0) / 120.0).clamp(0.0, 1.0) * 14.0;
        let demand_headroom = ((MAX_ENGINE_DEMAND_PCT - engine_demand_pct) / MAX_ENGINE_DEMAND_PCT)
            .clamp(0.0, 1.0)
            * 10.0;
        let base_cut = 24.0 + egt_bonus + demand_headroom;
        let pattern_bias = match self.scheduler_slot {
            0 | 3 => -2.0,
            1 | 4 => 2.0,
            _ => 5.0,
        };
        self.cut_pct = (base_cut + pattern_bias).clamp(18.0, 60.0).round() as u8;
        self.active_cylinders = if self.scheduler_slot % 3 == 0 { 2 } else { 3 };
    }

    fn apply_gate(&mut self, gate_reason_code: u8) {
        if gate_reason_code < GATE_HISTOGRAM_SLOTS as u8 {
            let slot = gate_reason_code as usize;
            self.gate_reason_histogram[slot] = self.gate_reason_histogram[slot].saturating_add(1);
        }
        self.last_gate_reason_code = gate_reason_code;
        self.deactivate(gate_reason_code);
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
            sync_guard_events: self.sync_guard_events_total.min(u8::MAX as u16) as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RotationalIdleRuntime, MAX_ACTIVE_DURATION_CS};

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
        let diagnostics = runtime.diagnostics();

        assert!(!sample.active);
        assert_eq!(sample.gate_reason_code, 9);
        assert!(sample.sync_guard_events > 0);
        assert!(diagnostics.sync_guard_events_total > 0);
        assert_eq!(diagnostics.last_gate_reason_code, 9);
        assert!(diagnostics.gate_reason_histogram[9] > 0);
    }

    #[test]
    fn times_out_after_max_runtime_window_and_tracks_counters() {
        let mut runtime = RotationalIdleRuntime::default();
        let mut timeout_gate_seen = false;

        for tick in 0..640 {
            let sample = runtime.tick(tick, 850.0, 3.0, 0, false);
            if sample.gate_reason_code == 8 {
                timeout_gate_seen = true;
                break;
            }
        }

        let diagnostics = runtime.diagnostics();
        assert!(timeout_gate_seen, "expected timeout gate reason");
        assert!(diagnostics.activation_count > 0, "expected activation counter");
        assert!(diagnostics.timeout_count > 0, "expected timeout counter");
        assert!(
            diagnostics.total_active_time_cs >= MAX_ACTIVE_DURATION_CS as u32,
            "expected active-time accounting to reach timeout window"
        );
        assert!(diagnostics.gate_reason_histogram[8] > 0);
    }

    #[test]
    fn gate_histogram_tracks_multiple_gate_sources() {
        let mut runtime = RotationalIdleRuntime::default();

        runtime.tick(0, 650.0, 2.0, 0, false); // gate 3
        runtime.tick(1, 850.0, 25.0, 0, false); // gate 4
        runtime.tick(2, 850.0, 2.0, 0, true); // gate 10

        let diagnostics = runtime.diagnostics();
        assert!(diagnostics.gate_reason_histogram[3] > 0);
        assert!(diagnostics.gate_reason_histogram[4] > 0);
        assert!(diagnostics.gate_reason_histogram[10] > 0);
        assert_eq!(diagnostics.last_gate_reason_code, 10);
    }
}
