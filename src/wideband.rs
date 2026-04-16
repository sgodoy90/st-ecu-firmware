const HEATER_WARMUP_CS: u16 = 260;
const PRIMARY_FAULT_HOLD_CS: u16 = 42;
const SECONDARY_FAULT_HOLD_CS: u16 = 28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidebandSource {
    IntegratedController,
    AnalogFallback,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidebandSample {
    pub heater_ready: bool,
    pub calibration_ready: bool,
    pub integrated_active: bool,
    pub analog_fallback: bool,
    pub primary_fault: bool,
    pub secondary_fault: bool,
    pub lambda_primary: f32,
    pub lambda_secondary: f32,
    pub source: WidebandSource,
}

#[derive(Debug, Clone)]
pub struct WidebandRuntime {
    heater_warmup_remaining_cs: u16,
    primary_fault_hold_cs: u16,
    secondary_fault_hold_cs: u16,
}

impl Default for WidebandRuntime {
    fn default() -> Self {
        Self {
            heater_warmup_remaining_cs: HEATER_WARMUP_CS,
            primary_fault_hold_cs: 0,
            secondary_fault_hold_cs: 0,
        }
    }
}

impl WidebandRuntime {
    pub fn tick(&mut self, sample_counter: u32, engine_running: bool) -> WidebandSample {
        if !engine_running {
            self.heater_warmup_remaining_cs = HEATER_WARMUP_CS;
            self.primary_fault_hold_cs = 0;
            self.secondary_fault_hold_cs = 0;
            return WidebandSample {
                heater_ready: false,
                calibration_ready: false,
                integrated_active: false,
                analog_fallback: true,
                primary_fault: false,
                secondary_fault: false,
                lambda_primary: 1.02,
                lambda_secondary: 1.03,
                source: WidebandSource::Unknown,
            };
        }

        if self.heater_warmup_remaining_cs > 0 {
            self.heater_warmup_remaining_cs =
                self.heater_warmup_remaining_cs.saturating_sub(2);
        }
        let heater_ready = self.heater_warmup_remaining_cs == 0;
        let calibration_ready = heater_ready;

        // Deterministic fault injection windows for diagnostics visibility.
        if heater_ready && sample_counter % 220 == 0 {
            self.primary_fault_hold_cs = PRIMARY_FAULT_HOLD_CS;
        }
        if heater_ready && sample_counter % 360 == 0 {
            self.secondary_fault_hold_cs = SECONDARY_FAULT_HOLD_CS;
        }

        if self.primary_fault_hold_cs > 0 {
            self.primary_fault_hold_cs = self.primary_fault_hold_cs.saturating_sub(2);
        }
        if self.secondary_fault_hold_cs > 0 {
            self.secondary_fault_hold_cs = self.secondary_fault_hold_cs.saturating_sub(2);
        }

        let primary_fault = self.primary_fault_hold_cs > 0;
        let secondary_fault = self.secondary_fault_hold_cs > 0;
        let integrated_active = heater_ready && !primary_fault && !secondary_fault;
        let analog_fallback = !integrated_active;
        let source = if integrated_active {
            WidebandSource::IntegratedController
        } else {
            WidebandSource::AnalogFallback
        };

        let ripple = ((sample_counter % 17) as f32 - 8.0) * 0.0008;
        let lambda_primary = if integrated_active {
            0.998 + ripple
        } else {
            1.028 + ripple
        };
        let lambda_secondary = if integrated_active {
            1.004 + ripple * 0.8
        } else {
            1.034 + ripple * 0.8
        };

        WidebandSample {
            heater_ready,
            calibration_ready,
            integrated_active,
            analog_fallback,
            primary_fault,
            secondary_fault,
            lambda_primary,
            lambda_secondary,
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{WidebandRuntime, WidebandSource};

    #[test]
    fn transitions_from_warmup_to_integrated_mode() {
        let mut runtime = WidebandRuntime::default();
        let mut last = runtime.tick(0, true);
        for tick in 1..200 {
            last = runtime.tick(tick, true);
        }

        assert!(last.heater_ready, "heater never became ready");
        assert!(last.calibration_ready, "calibration never became ready");
        assert!(last.integrated_active, "controller never became integrated-active");
        assert_eq!(last.source, WidebandSource::IntegratedController);
    }

    #[test]
    fn deterministic_fault_window_forces_analog_fallback() {
        let mut runtime = WidebandRuntime::default();

        // Fast-forward warmup.
        for tick in 0..200 {
            runtime.tick(tick, true);
        }

        let fault_sample = runtime.tick(220, true);
        assert!(fault_sample.primary_fault);
        assert!(fault_sample.analog_fallback);
        assert_eq!(fault_sample.source, WidebandSource::AnalogFallback);
    }

    #[test]
    fn engine_stop_resets_heater_state() {
        let mut runtime = WidebandRuntime::default();
        for tick in 0..180 {
            runtime.tick(tick, true);
        }

        let stopped = runtime.tick(181, false);
        assert!(!stopped.heater_ready);
        assert_eq!(stopped.source, WidebandSource::Unknown);
    }
}
