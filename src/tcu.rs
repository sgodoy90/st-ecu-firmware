use crate::live_data::transmission_status;

const OFFLINE_PERIOD_SAMPLES: u32 = 240;
const OFFLINE_WINDOW_SAMPLES: u32 = 12;
const REQUEST_PERIOD_SAMPLES: u32 = 30;
const REQUEST_STEP_CS: u16 = 2;
const DEFAULT_SHIFT_TIMER_CS: u16 = 36;
const STALE_REQUEST_CS: u16 = 46;
const TIMEOUT_REQUEST_CS: u16 = 62;
const RESULT_HOLD_TICKS: u8 = 8;
const RX_STALE_PERIOD_SAMPLES: u32 = 170;
const RX_STALE_WINDOW_SAMPLES: u32 = 10;
const TX_STALE_PERIOD_SAMPLES: u32 = 210;
const TX_STALE_WINDOW_SAMPLES: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransmissionSample {
    pub status_flags: u8,
    pub requested_gear: u8,
    pub torque_reduction_pct: u8,
    pub torque_reduction_timer_cs: u16,
    pub shift_result_code: u8,
    pub shift_request_counter: u8,
    pub shift_timeout_counter: u8,
    pub shift_fault_code: u8,
    pub state_code: u8,
    pub request_age_cs: u16,
    pub ack_counter: u8,
}

#[derive(Debug, Clone)]
pub struct ExternalTcuRuntime {
    shift_active: bool,
    shift_timer_cs: u16,
    shift_request_counter: u8,
    shift_timeout_counter: u8,
    shift_fault_code: u8,
    state_code: u8,
    request_age_cs: u16,
    ack_counter: u8,
    requested_gear: u8,
    shift_result_code: u8,
    hold_ticks: u8,
    rx_stream_fresh: bool,
    tx_stream_fresh: bool,
}

impl Default for ExternalTcuRuntime {
    fn default() -> Self {
        Self {
            shift_active: false,
            shift_timer_cs: 0,
            shift_request_counter: 0,
            shift_timeout_counter: 0,
            shift_fault_code: 0,
            state_code: 0,
            request_age_cs: 0,
            ack_counter: 0,
            requested_gear: 0,
            shift_result_code: 0,
            hold_ticks: 0,
            rx_stream_fresh: false,
            tx_stream_fresh: false,
        }
    }
}

impl ExternalTcuRuntime {
    pub fn tick(&mut self, sample_counter: u32) -> TransmissionSample {
        let link_online = sample_counter % OFFLINE_PERIOD_SAMPLES >= OFFLINE_WINDOW_SAMPLES;
        let (rx_stream_fresh, tx_stream_fresh) =
            self.compute_stream_freshness(sample_counter, link_online);
        self.rx_stream_fresh = rx_stream_fresh;
        self.tx_stream_fresh = tx_stream_fresh;

        if !link_online {
            if self.shift_active || self.requested_gear > 0 {
                self.raise_fault(1, 6, 5, false);
            } else {
                self.state_code = 6;
            }
            self.shift_active = false;
            self.shift_timer_cs = 0;
            self.request_age_cs = 0;
            self.requested_gear = 0;
            self.hold_ticks = self.hold_ticks.max(2);
        } else {
            if self.state_code == 6 && self.hold_ticks == 0 {
                self.clear_to_idle();
            }

            if !self.shift_active
                && self.hold_ticks == 0
                && sample_counter % REQUEST_PERIOD_SAMPLES == 1
            {
                self.start_shift_request();
            }

            if self.shift_active {
                self.step_active_shift(self.rx_stream_fresh, self.tx_stream_fresh);
            } else if self.hold_ticks > 0 {
                self.hold_ticks = self.hold_ticks.saturating_sub(1);
                if self.hold_ticks == 0 && self.state_code != 6 {
                    self.clear_to_idle();
                }
            }
        }

        let mut status_flags = 0u8;
        if link_online {
            status_flags |= transmission_status::TCU_LINK_ONLINE;
        }
        if self.rx_stream_fresh {
            status_flags |= transmission_status::RX_STREAM_FRESH;
        }
        if self.tx_stream_fresh {
            status_flags |= transmission_status::TX_STREAM_FRESH;
        }

        let mut torque_reduction_pct = 0u8;
        if self.shift_active {
            status_flags |= transmission_status::TORQUE_INTERVENTION_REQUESTED;
            if self.state_code >= 2 {
                torque_reduction_pct = if self.state_code >= 3 { 26 } else { 14 };
            }
            if self.state_code >= 3 {
                status_flags |= transmission_status::SHIFT_IN_PROGRESS;
                status_flags |= transmission_status::TORQUE_INTERVENTION_ACTIVE;
            }
        }

        TransmissionSample {
            status_flags,
            requested_gear: self.requested_gear,
            torque_reduction_pct,
            torque_reduction_timer_cs: self.shift_timer_cs,
            shift_result_code: self.shift_result_code,
            shift_request_counter: self.shift_request_counter,
            shift_timeout_counter: self.shift_timeout_counter,
            shift_fault_code: self.shift_fault_code,
            state_code: self.state_code,
            request_age_cs: self.request_age_cs,
            ack_counter: self.ack_counter,
        }
    }

    pub fn shift_in_progress(&self) -> bool {
        self.shift_active && self.state_code >= 3
    }

    fn start_shift_request(&mut self) {
        self.shift_active = true;
        self.shift_request_counter = self.shift_request_counter.wrapping_add(1);
        self.shift_timer_cs = if self.shift_request_counter % 5 == 0 {
            // Every 5th shift request intentionally stretches long enough to exercise
            // timeout handling in desktop diagnostics and parser logic.
            TIMEOUT_REQUEST_CS + 8
        } else {
            DEFAULT_SHIFT_TIMER_CS
        };
        self.shift_fault_code = 0;
        self.shift_result_code = 0;
        self.state_code = 1; // request_pending
        self.request_age_cs = 0;
        self.requested_gear = (self.shift_request_counter % 6) + 1;
    }

    fn step_active_shift(&mut self, rx_stream_fresh: bool, tx_stream_fresh: bool) {
        self.request_age_cs = self.request_age_cs.saturating_add(REQUEST_STEP_CS);
        if self.request_age_cs >= 8 {
            self.state_code = 2; // torque_reduction
        }
        if self.request_age_cs >= 18 {
            self.state_code = 3; // shifting
        }
        self.shift_timer_cs = self.shift_timer_cs.saturating_sub(REQUEST_STEP_CS);

        // Stream watchdog faults are surfaced via explicit fault codes so desktop can
        // classify communication freshness regressions without guessing.
        let timeout_vector_request = self.shift_request_counter % 5 == 0;
        if !timeout_vector_request && !rx_stream_fresh && self.request_age_cs >= 10 {
            self.raise_fault(7, 5, 5, false); // rx_watchdog
            return;
        }
        if !timeout_vector_request && !tx_stream_fresh && self.request_age_cs >= 14 {
            self.raise_fault(8, 5, 5, false); // tx_watchdog
            return;
        }

        // Deterministic failure vectors for diagnostics and parser hardening.
        if self.shift_request_counter % 11 == 0 && self.request_age_cs >= 14 {
            self.raise_fault(4, 5, 4, false); // aborted
            return;
        }
        if self.shift_request_counter % 7 == 0 && self.request_age_cs >= 10 {
            self.raise_fault(3, 5, 3, false); // denied
            return;
        }
        if self.request_age_cs >= TIMEOUT_REQUEST_CS {
            self.shift_timeout_counter = self.shift_timeout_counter.wrapping_add(1);
            self.raise_fault(2, 5, 2, true); // timeout
            return;
        }
        if self.request_age_cs >= STALE_REQUEST_CS && self.shift_request_counter % 13 == 0 {
            self.raise_fault(6, 5, 5, false); // stale_request -> generic fault result
            return;
        }

        if self.shift_timer_cs == 0 {
            self.shift_active = false;
            self.shift_result_code = 1; // completed
            self.state_code = 4; // completed
            self.ack_counter = self.ack_counter.wrapping_add(1);
            self.requested_gear = 0;
            self.request_age_cs = 0;
            self.hold_ticks = RESULT_HOLD_TICKS;
        }
    }

    fn raise_fault(&mut self, fault_code: u8, state_code: u8, result_code: u8, clear_gear: bool) {
        self.shift_active = false;
        self.shift_timer_cs = 0;
        self.shift_fault_code = fault_code;
        self.state_code = state_code;
        self.shift_result_code = result_code;
        self.request_age_cs = 0;
        if clear_gear {
            self.requested_gear = 0;
        }
        self.hold_ticks = RESULT_HOLD_TICKS;
    }

    fn clear_to_idle(&mut self) {
        self.state_code = 0;
        self.shift_fault_code = 0;
        self.shift_result_code = 0;
        self.requested_gear = 0;
        self.request_age_cs = 0;
    }

    fn compute_stream_freshness(&self, sample_counter: u32, link_online: bool) -> (bool, bool) {
        if !link_online {
            return (false, false);
        }

        let rx_stale = sample_counter % RX_STALE_PERIOD_SAMPLES < RX_STALE_WINDOW_SAMPLES;
        let tx_stale = sample_counter % TX_STALE_PERIOD_SAMPLES < TX_STALE_WINDOW_SAMPLES;
        (!rx_stale, !tx_stale)
    }
}

#[cfg(test)]
mod tests {
    use crate::live_data::transmission_status;

    use super::{ExternalTcuRuntime, TransmissionSample};

    fn sample_for_tick(runtime: &mut ExternalTcuRuntime, tick: u32) -> TransmissionSample {
        runtime.tick(tick)
    }

    #[test]
    fn produces_shift_request_and_ack_progress() {
        let mut runtime = ExternalTcuRuntime::default();
        let mut max_request = 0u8;
        let mut max_ack = 0u8;
        let mut saw_shifting = false;

        for tick in 0..240 {
            let sample = sample_for_tick(&mut runtime, tick);
            max_request = max_request.max(sample.shift_request_counter);
            max_ack = max_ack.max(sample.ack_counter);
            saw_shifting |= sample.state_code == 3;
        }

        assert!(max_request > 0, "expected at least one shift request");
        assert!(max_ack > 0, "expected at least one completed shift ack");
        assert!(saw_shifting, "expected to observe shifting state");
    }

    #[test]
    fn reports_offline_windows_and_faults() {
        let mut runtime = ExternalTcuRuntime::default();
        let mut saw_offline = false;
        let mut saw_fault = false;

        for tick in 0..520 {
            let sample = sample_for_tick(&mut runtime, tick);
            saw_offline |= sample.state_code == 6;
            saw_fault |= sample.shift_fault_code != 0;
        }

        assert!(saw_offline, "expected periodic offline window");
        assert!(saw_fault, "expected at least one deterministic fault code");
    }

    #[test]
    fn request_age_moves_when_request_is_active() {
        let mut runtime = ExternalTcuRuntime::default();
        let mut saw_request_age = false;

        for tick in 0..180 {
            let sample = sample_for_tick(&mut runtime, tick);
            if sample.request_age_cs > 0 {
                saw_request_age = true;
                break;
            }
        }

        assert!(saw_request_age, "request age counter never became non-zero");
    }

    #[test]
    fn toggles_rx_tx_freshness_status_bits() {
        let mut runtime = ExternalTcuRuntime::default();
        let mut saw_rx_stale = false;
        let mut saw_tx_stale = false;
        let mut saw_both_fresh = false;

        for tick in 0..420 {
            let sample = sample_for_tick(&mut runtime, tick);
            let rx_fresh = (sample.status_flags & transmission_status::RX_STREAM_FRESH) != 0;
            let tx_fresh = (sample.status_flags & transmission_status::TX_STREAM_FRESH) != 0;
            saw_rx_stale |= !rx_fresh;
            saw_tx_stale |= !tx_fresh;
            saw_both_fresh |= rx_fresh && tx_fresh;
        }

        assert!(saw_rx_stale, "expected deterministic rx stale windows");
        assert!(saw_tx_stale, "expected deterministic tx stale windows");
        assert!(saw_both_fresh, "expected steady-state fresh windows");
    }

    #[test]
    fn raises_watchdog_fault_codes_when_stream_freshness_drops_mid_request() {
        let mut runtime = ExternalTcuRuntime::default();
        let mut saw_rx_watchdog_fault = false;
        let mut saw_tx_watchdog_fault = false;

        for tick in 0..1800 {
            let sample = sample_for_tick(&mut runtime, tick);
            saw_rx_watchdog_fault |= sample.shift_fault_code == 7;
            saw_tx_watchdog_fault |= sample.shift_fault_code == 8;
            if saw_rx_watchdog_fault && saw_tx_watchdog_fault {
                break;
            }
        }

        assert!(saw_rx_watchdog_fault, "expected rx watchdog fault code 7");
        assert!(saw_tx_watchdog_fault, "expected tx watchdog fault code 8");
    }
}
