use crate::trigger::{TriggerCapture, TriggerDecoderPreset, TriggerToothLog, SUPPORTED_TRIGGER_DECODERS};
use crate::trigger_model::{distributor_even, missing_tooth, EngineStroke, TriggerEdgePolicy};

#[cfg(test)]
const PAGE0_MAGIC: [u8; 4] = *b"STC2";
#[cfg(test)]
const PAGE0_VERSION: u8 = 1;
const PAGE0_OFFSET_ENGINE_CYLINDERS: usize = 5;
const PAGE0_OFFSET_TRIGGER_TYPE: usize = 44;
const PAGE0_OFFSET_TRIGGER_TOTAL_TEETH: usize = 46;
const PAGE0_OFFSET_TRIGGER_MISSING_TEETH: usize = 48;
const PAGE0_OFFSET_TRIGGER_CAM_SYNC: usize = 55;
const PAGE0_OFFSET_TRIGGER_SECONDARY_ENABLED: usize = 57;

const TRIGGER_TYPE_MISSING_TOOTH: u8 = 0;
const TRIGGER_TYPE_SUBARU_6_7: u8 = 1;
const TRIGGER_TYPE_4G63: u8 = 2;
const TRIGGER_TYPE_GM_LS: u8 = 3;
const TRIGGER_TYPE_HONDA_D: u8 = 4;
const TRIGGER_TYPE_HONDA_K: u8 = 5;
const TRIGGER_TYPE_NISSAN_VQ: u8 = 6;
const TRIGGER_TYPE_NISSAN_SR20: u8 = 7;
const TRIGGER_TYPE_TOYOTA_3UZFE: u8 = 8;
const TRIGGER_TYPE_FORD_ST170: u8 = 9;
const TRIGGER_TYPE_VR_HALL: u8 = 10;
const TRIGGER_TYPE_DISTRIBUTOR_4CYL: u8 = 11;
const TRIGGER_TYPE_DISTRIBUTOR_6CYL: u8 = 12;
const TRIGGER_TYPE_SINGLE_TOOTH: u8 = 13;
const TRIGGER_TYPE_CUSTOM: u8 = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeMode {
    MissingTooth {
        total_teeth: u16,
        missing_teeth: u16,
    },
    Distributor {
        cylinders: u8,
    },
    DualWheel {
        primary_teeth: u16,
        secondary_events: u8,
    },
    CatalogDefault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Page0TriggerConfig {
    cylinders: u8,
    trigger_type: u8,
    total_teeth: u16,
    missing_teeth: u16,
    cam_sync: bool,
    secondary_enabled: bool,
}

impl Default for Page0TriggerConfig {
    fn default() -> Self {
        Self {
            cylinders: 4,
            trigger_type: TRIGGER_TYPE_MISSING_TOOTH,
            total_teeth: 60,
            missing_teeth: 2,
            cam_sync: true,
            secondary_enabled: true,
        }
    }
}

impl Page0TriggerConfig {
    fn parse(payload: &[u8]) -> Option<Self> {
        if payload.len() <= PAGE0_OFFSET_TRIGGER_SECONDARY_ENABLED {
            return None;
        }

        let mut config = Self::default();
        let cylinders = payload[PAGE0_OFFSET_ENGINE_CYLINDERS];
        config.cylinders = sanitize_cylinder_count(cylinders);
        config.trigger_type = payload[PAGE0_OFFSET_TRIGGER_TYPE];
        config.total_teeth = u16::from_be_bytes([
            payload[PAGE0_OFFSET_TRIGGER_TOTAL_TEETH],
            payload[PAGE0_OFFSET_TRIGGER_TOTAL_TEETH + 1],
        ]);
        config.missing_teeth = u16::from_be_bytes([
            payload[PAGE0_OFFSET_TRIGGER_MISSING_TEETH],
            payload[PAGE0_OFFSET_TRIGGER_MISSING_TEETH + 1],
        ]);
        config.cam_sync = payload[PAGE0_OFFSET_TRIGGER_CAM_SYNC] != 0;
        config.secondary_enabled = payload[PAGE0_OFFSET_TRIGGER_SECONDARY_ENABLED] != 0;
        Some(config)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TriggerSelection {
    preset_key: &'static str,
    mode: RuntimeMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RuntimeProfile {
    engine_cycle_deg: u16,
    primary_events_per_cycle: u16,
    secondary_events_per_cycle: u16,
    dominant_gap_ratio: Option<f32>,
    sync_gap_tooth_count: u8,
    reference_event_index: u16,
}

#[derive(Debug, Clone, PartialEq)]
struct WaveformSnapshot {
    sync_state: &'static str,
    trigger_rpm: u16,
    capture_window_us: u32,
    sample_period_us: u16,
    tooth_count: u16,
    sync_gap_tooth_count: u8,
    primary_edge_count: u16,
    secondary_edge_count: u16,
    primary_samples: Vec<u8>,
    secondary_samples: Vec<u8>,
    reference_event_index: u16,
    dominant_gap_ratio: f32,
    tooth_intervals_us: Vec<u32>,
    secondary_event_indexes: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct TriggerRuntime {
    active_preset_key: &'static str,
    mode: RuntimeMode,
    cylinders: u8,
    cam_sync: bool,
    secondary_enabled: bool,
    sample_counter: u32,
    sync_loss_counter: u32,
    synced_cycles: u32,
}

impl Default for TriggerRuntime {
    fn default() -> Self {
        Self {
            active_preset_key: "generic_60_2",
            mode: RuntimeMode::MissingTooth {
                total_teeth: 60,
                missing_teeth: 2,
            },
            cylinders: 4,
            cam_sync: true,
            secondary_enabled: true,
            sample_counter: 0,
            sync_loss_counter: 0,
            synced_cycles: 0,
        }
    }
}

impl TriggerRuntime {
    pub fn apply_page0_payload(&mut self, payload: &[u8]) {
        let config = Page0TriggerConfig::parse(payload).unwrap_or_default();
        let selection = select_decoder_from_page0(config);
        let preset = find_preset(selection.preset_key);

        self.active_preset_key = preset.key;
        self.mode = selection.mode;
        self.cylinders = sanitize_cylinder_count(config.cylinders);
        self.cam_sync = config.cam_sync;
        self.secondary_enabled = config.secondary_enabled;
    }

    pub fn observe_tick(&mut self, sample_counter: u32) {
        let _ = self.advance_state(sample_counter);
    }

    pub fn trigger_capture(&mut self, sample_counter: u32) -> TriggerCapture {
        let sync_state = self.advance_state(sample_counter);
        let preset = self.active_preset();
        let waveform = self.build_waveform(sync_state, preset);

        TriggerCapture {
            preset_key: preset.key,
            preset_label: preset.label,
            sync_state: waveform.sync_state,
            trigger_rpm: waveform.trigger_rpm,
            sync_loss_counter: self.sync_loss_counter,
            synced_cycles: self.synced_cycles,
            engine_cycle_deg: self.runtime_profile(preset).engine_cycle_deg,
            capture_window_us: waveform.capture_window_us,
            sample_period_us: waveform.sample_period_us,
            primary_label: preset.primary_input_label,
            secondary_label: preset.secondary_input_label,
            tooth_count: waveform.tooth_count,
            sync_gap_tooth_count: waveform.sync_gap_tooth_count,
            primary_edge_count: waveform.primary_edge_count,
            secondary_edge_count: waveform.secondary_edge_count,
            primary_samples: waveform.primary_samples,
            secondary_samples: waveform.secondary_samples,
        }
    }

    pub fn trigger_tooth_log(&mut self, sample_counter: u32) -> TriggerToothLog {
        let sync_state = self.advance_state(sample_counter);
        let preset = self.active_preset();
        let waveform = self.build_waveform(sync_state, preset);

        TriggerToothLog {
            preset_key: preset.key,
            preset_label: preset.label,
            sync_state: waveform.sync_state,
            trigger_rpm: waveform.trigger_rpm,
            engine_cycle_deg: self.runtime_profile(preset).engine_cycle_deg,
            primary_label: preset.primary_input_label,
            secondary_label: preset.secondary_input_label,
            reference_event_index: waveform.reference_event_index,
            dominant_gap_ratio: waveform.dominant_gap_ratio,
            tooth_intervals_us: waveform.tooth_intervals_us,
            secondary_event_indexes: waveform.secondary_event_indexes,
        }
    }

    fn advance_state(&mut self, sample_counter: u32) -> &'static str {
        self.sample_counter = self.sample_counter.wrapping_add(1);
        if sample_counter > self.sample_counter {
            self.sample_counter = sample_counter;
        }

        if self.sample_counter % 173 == 0 {
            self.sync_loss_counter = self.sync_loss_counter.saturating_add(1);
        }

        let sync_state = if self.sample_counter % 173 == 0 {
            "syncing"
        } else {
            "locked"
        };

        if sync_state == "locked" {
            self.synced_cycles = self.synced_cycles.saturating_add(1);
        }

        sync_state
    }

    fn active_preset(&self) -> &'static TriggerDecoderPreset {
        find_preset(self.active_preset_key)
    }

    fn runtime_profile(&self, preset: &'static TriggerDecoderPreset) -> RuntimeProfile {
        match self.mode {
            RuntimeMode::MissingTooth {
                total_teeth,
                missing_teeth,
            } => {
                let (sanitized_total, sanitized_missing) =
                    sanitize_missing_tooth_layout(total_teeth, missing_teeth);
                let corr = missing_tooth(
                    sanitized_total,
                    sanitized_missing,
                    EngineStroke::FourStroke,
                    TriggerEdgePolicy::RisingOnly,
                )
                .unwrap_or_else(|_| {
                    missing_tooth(
                        60,
                        2,
                        EngineStroke::FourStroke,
                        TriggerEdgePolicy::RisingOnly,
                    )
                    .expect("60-2 correlation must remain valid")
                });

                RuntimeProfile {
                    engine_cycle_deg: corr.engine_cycle_deg,
                    primary_events_per_cycle: corr.primary_events_per_cycle.max(4),
                    secondary_events_per_cycle: if self.secondary_enabled || self.cam_sync {
                        corr.secondary_events_per_cycle.max(1)
                    } else {
                        0
                    },
                    dominant_gap_ratio: corr.dominant_gap_ratio,
                    sync_gap_tooth_count: sanitized_missing.min(u8::MAX as u16) as u8,
                    reference_event_index: corr.primary_events_per_cycle / 2,
                }
            }
            RuntimeMode::Distributor { cylinders } => {
                let normalized = sanitize_cylinder_count(cylinders);
                let corr = distributor_even(
                    normalized,
                    EngineStroke::FourStroke,
                    TriggerEdgePolicy::RisingOnly,
                )
                .unwrap_or_else(|_| {
                    distributor_even(
                        4,
                        EngineStroke::FourStroke,
                        TriggerEdgePolicy::RisingOnly,
                    )
                    .expect("4-cylinder distributor correlation must remain valid")
                });

                RuntimeProfile {
                    engine_cycle_deg: corr.engine_cycle_deg,
                    primary_events_per_cycle: corr.primary_events_per_cycle.max(4),
                    secondary_events_per_cycle: if self.secondary_enabled || self.cam_sync {
                        1
                    } else {
                        0
                    },
                    dominant_gap_ratio: None,
                    sync_gap_tooth_count: 0,
                    reference_event_index: 0,
                }
            }
            RuntimeMode::DualWheel {
                primary_teeth,
                secondary_events,
            } => {
                let primary = primary_teeth.clamp(4, 96);
                let secondary = if self.secondary_enabled || self.cam_sync {
                    secondary_events.max(1) as u16
                } else {
                    0
                };
                RuntimeProfile {
                    engine_cycle_deg: 720,
                    primary_events_per_cycle: primary.saturating_mul(2),
                    secondary_events_per_cycle: secondary,
                    dominant_gap_ratio: None,
                    sync_gap_tooth_count: 0,
                    reference_event_index: primary / 2,
                }
            }
            RuntimeMode::CatalogDefault => RuntimeProfile {
                engine_cycle_deg: preset.expected_engine_cycle_deg,
                primary_events_per_cycle: oem_nominal_events_for_key(preset.key),
                secondary_events_per_cycle: if preset.requires_secondary
                    || self.secondary_enabled
                    || self.cam_sync
                {
                    2
                } else {
                    0
                },
                dominant_gap_ratio: if preset.sync_strategy == "missing_tooth_plus_home" {
                    Some(1.45)
                } else {
                    None
                },
                sync_gap_tooth_count: if preset.sync_strategy == "missing_tooth_plus_home" {
                    1
                } else {
                    0
                },
                reference_event_index: 0,
            },
        }
    }

    fn build_waveform(
        &self,
        sync_state: &'static str,
        preset: &'static TriggerDecoderPreset,
    ) -> WaveformSnapshot {
        let profile = self.runtime_profile(preset);
        let trigger_rpm = 780u16.saturating_add((self.sample_counter % 180) as u16);
        let capture_window_us = capture_window_us(profile.engine_cycle_deg, trigger_rpm);
        let sample_count = 128usize;
        let sample_period_us = (capture_window_us / sample_count as u32)
            .max(1)
            .min(u16::MAX as u32) as u16;

        let requested_events = profile.primary_events_per_cycle.max(4) as usize;
        let event_count = requested_events.clamp(6, 96);
        let base_interval_us = (capture_window_us / event_count as u32).max(1);
        let mut intervals = Vec::with_capacity(event_count);
        for index in 0..event_count {
            let jitter_step = (base_interval_us / 40).max(1) as i32;
            let jitter = ((self.sample_counter.wrapping_add(index as u32)) % 7) as i32 - 3;
            let value = (base_interval_us as i32 + jitter * jitter_step).max(1) as u32;
            intervals.push(value);
        }

        let gap_index = event_count / 2;
        if let Some(ratio) = profile.dominant_gap_ratio {
            if let Some(gap_interval) = intervals.get_mut(gap_index) {
                let scaled = ((*gap_interval as f32) * ratio).round().max(1.0) as u32;
                *gap_interval = scaled;
            }
        }

        let dominant_gap_ratio = if profile.dominant_gap_ratio.is_some() && event_count > 1 {
            let dominant = intervals[gap_index] as f32;
            let mut baseline_sum = 0u64;
            let mut baseline_count = 0u64;
            for (index, interval) in intervals.iter().enumerate() {
                if index == gap_index {
                    continue;
                }
                baseline_sum = baseline_sum.saturating_add(*interval as u64);
                baseline_count = baseline_count.saturating_add(1);
            }
            if baseline_count > 0 {
                let baseline = (baseline_sum as f32 / baseline_count as f32).max(1.0);
                (dominant / baseline).max(1.0)
            } else {
                1.0
            }
        } else {
            1.0
        };

        let total_interval_us = intervals
            .iter()
            .copied()
            .fold(0u32, u32::saturating_add)
            .max(1);

        let mut primary_samples = vec![0u8; sample_count];
        let mut pulse_positions = Vec::with_capacity(intervals.len());
        let mut elapsed = 0u32;
        for interval in &intervals {
            elapsed = elapsed.saturating_add(*interval);
            let ratio = elapsed as f32 / total_interval_us as f32;
            let pos = ((sample_count as f32 - 2.0) * ratio).round() as usize;
            let clamped = pos.min(sample_count.saturating_sub(2));
            pulse_positions.push(clamped);
            primary_samples[clamped] = 1;
            primary_samples[clamped + 1] = 1;
        }

        let secondary_count = profile.secondary_events_per_cycle as usize;
        let secondary_slots = secondary_count.clamp(0, 8);
        let mut secondary_event_indexes = Vec::with_capacity(secondary_slots);
        if secondary_slots > 0 {
            for slot in 0..secondary_slots {
                let mut index = ((slot + 1) * intervals.len()) / (secondary_slots + 1);
                index = index.clamp(0, intervals.len().saturating_sub(1));
                secondary_event_indexes.push(index as u16);
            }
            secondary_event_indexes.sort_unstable();
            secondary_event_indexes.dedup();
        }

        let mut secondary_samples = vec![0u8; sample_count];
        for index in &secondary_event_indexes {
            let position = pulse_positions
                .get(*index as usize)
                .copied()
                .unwrap_or(sample_count / 2)
                .min(sample_count.saturating_sub(3));
            for sample in &mut secondary_samples[position..position + 3] {
                *sample = 1;
            }
        }

        let reference_event_index = if profile.dominant_gap_ratio.is_some() {
            gap_index as u16
        } else {
            profile
                .reference_event_index
                .min(intervals.len().saturating_sub(1) as u16)
        };

        WaveformSnapshot {
            sync_state,
            trigger_rpm,
            capture_window_us,
            sample_period_us,
            tooth_count: intervals.len() as u16,
            sync_gap_tooth_count: profile.sync_gap_tooth_count,
            primary_edge_count: (intervals.len() as u16).saturating_mul(2),
            secondary_edge_count: (secondary_event_indexes.len() as u16).saturating_mul(2),
            primary_samples,
            secondary_samples,
            reference_event_index,
            dominant_gap_ratio,
            tooth_intervals_us: intervals,
            secondary_event_indexes,
        }
    }
}

fn select_decoder_from_page0(config: Page0TriggerConfig) -> TriggerSelection {
    let distributor_cyl = normalize_distributor_cylinders(config.cylinders);
    let distributor_key = match distributor_cyl {
        8 => "distributor_basic_8",
        6 => "distributor_basic_6",
        _ => "distributor_basic_4",
    };

    let missing_mode = || {
        let (total_teeth, missing_teeth) =
            sanitize_missing_tooth_layout(config.total_teeth, config.missing_teeth);
        let preset_key = if total_teeth == 36 && missing_teeth == 1 {
            "generic_36_1"
        } else {
            "generic_60_2"
        };
        TriggerSelection {
            preset_key,
            mode: RuntimeMode::MissingTooth {
                total_teeth,
                missing_teeth,
            },
        }
    };

    match config.trigger_type {
        TRIGGER_TYPE_MISSING_TOOTH => missing_mode(),
        TRIGGER_TYPE_SUBARU_6_7 => TriggerSelection {
            preset_key: "subaru_6_7",
            mode: RuntimeMode::CatalogDefault,
        },
        TRIGGER_TYPE_4G63 => TriggerSelection {
            preset_key: "mitsubishi_4g63",
            mode: RuntimeMode::CatalogDefault,
        },
        TRIGGER_TYPE_GM_LS => TriggerSelection {
            preset_key: "gm_24x",
            mode: RuntimeMode::CatalogDefault,
        },
        TRIGGER_TYPE_HONDA_D | TRIGGER_TYPE_HONDA_K => TriggerSelection {
            preset_key: "honda_k20_12_1",
            mode: RuntimeMode::CatalogDefault,
        },
        TRIGGER_TYPE_NISSAN_VQ => TriggerSelection {
            preset_key: "nissan_vq_36_2_2_2",
            mode: RuntimeMode::CatalogDefault,
        },
        TRIGGER_TYPE_NISSAN_SR20 => TriggerSelection {
            preset_key: "nissan_360_slot",
            mode: RuntimeMode::CatalogDefault,
        },
        TRIGGER_TYPE_TOYOTA_3UZFE => TriggerSelection {
            preset_key: "toyota_36_2_2_2",
            mode: RuntimeMode::CatalogDefault,
        },
        TRIGGER_TYPE_FORD_ST170 => TriggerSelection {
            preset_key: "ford_st170",
            mode: RuntimeMode::CatalogDefault,
        },
        TRIGGER_TYPE_VR_HALL => {
            if config.secondary_enabled || config.cam_sync {
                let primary_teeth = config.total_teeth.clamp(4, 96);
                TriggerSelection {
                    preset_key: "dual_wheel",
                    mode: RuntimeMode::DualWheel {
                        primary_teeth,
                        secondary_events: 1,
                    },
                }
            } else {
                missing_mode()
            }
        }
        TRIGGER_TYPE_DISTRIBUTOR_4CYL | TRIGGER_TYPE_DISTRIBUTOR_6CYL => TriggerSelection {
            preset_key: distributor_key,
            mode: RuntimeMode::Distributor {
                cylinders: distributor_cyl,
            },
        },
        TRIGGER_TYPE_SINGLE_TOOTH => TriggerSelection {
            preset_key: "dual_wheel",
            mode: RuntimeMode::DualWheel {
                primary_teeth: config.total_teeth.clamp(4, 96),
                secondary_events: if config.secondary_enabled { 2 } else { 1 },
            },
        },
        TRIGGER_TYPE_CUSTOM => {
            if config.secondary_enabled || config.cam_sync {
                TriggerSelection {
                    preset_key: "dual_wheel",
                    mode: RuntimeMode::DualWheel {
                        primary_teeth: config.total_teeth.clamp(4, 96),
                        secondary_events: if config.secondary_enabled { 2 } else { 1 },
                    },
                }
            } else if config.missing_teeth > 0 {
                missing_mode()
            } else {
                TriggerSelection {
                    preset_key: distributor_key,
                    mode: RuntimeMode::Distributor {
                        cylinders: distributor_cyl,
                    },
                }
            }
        }
        _ => missing_mode(),
    }
}

fn find_preset(key: &str) -> &'static TriggerDecoderPreset {
    SUPPORTED_TRIGGER_DECODERS
        .iter()
        .find(|preset| preset.key == key)
        .unwrap_or(&SUPPORTED_TRIGGER_DECODERS[0])
}

fn sanitize_cylinder_count(value: u8) -> u8 {
    if value == 0 {
        4
    } else {
        value.min(16)
    }
}

fn normalize_distributor_cylinders(value: u8) -> u8 {
    let cylinders = sanitize_cylinder_count(value);
    if cylinders >= 8 {
        8
    } else if cylinders >= 6 {
        6
    } else {
        4
    }
}

fn sanitize_missing_tooth_layout(total_teeth: u16, missing_teeth: u16) -> (u16, u16) {
    let total = if total_teeth < 4 { 60 } else { total_teeth.min(512) };
    let mut missing = if missing_teeth == 0 {
        2
    } else {
        missing_teeth.min(total.saturating_sub(1))
    };
    if missing == 0 {
        missing = 1;
    }
    (total, missing)
}

fn oem_nominal_events_for_key(key: &str) -> u16 {
    match key {
        "honda_k20_12_1" => 12,
        "toyota_36_2_2_2" => 64,
        "gm_24x" => 48,
        "nissan_360_slot" => 90,
        "subaru_6_7" => 12,
        "mitsubishi_4g63" => 8,
        "mazda_36_2_1" => 70,
        "ford_st170" => 70,
        "gm_58x" => 116,
        "toyota_2jz_vvti" => 68,
        "bmw_m54_60_2" => 116,
        "ford_coyote" => 70,
        "subaru_ej_36_2_2_2" => 64,
        "mazda_bp_4_1" => 8,
        "nissan_vq_36_2_2_2" => 64,
        "dual_wheel" => 48,
        _ => 36,
    }
}

fn capture_window_us(engine_cycle_deg: u16, rpm: u16) -> u32 {
    let safe_rpm = rpm.max(1) as u32;
    let rev_us = 60_000_000u32 / safe_rpm;
    rev_us
        .saturating_mul(engine_cycle_deg as u32)
        .saturating_div(360)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::{
        TriggerRuntime, PAGE0_MAGIC, PAGE0_OFFSET_ENGINE_CYLINDERS, PAGE0_OFFSET_TRIGGER_TYPE,
        PAGE0_VERSION, TRIGGER_TYPE_DISTRIBUTOR_4CYL, TRIGGER_TYPE_MISSING_TOOTH,
        TRIGGER_TYPE_SINGLE_TOOTH,
    };

    const PAGE0_OFFSET_TOTAL_TEETH: usize = super::PAGE0_OFFSET_TRIGGER_TOTAL_TEETH;
    const PAGE0_OFFSET_MISSING_TEETH: usize = super::PAGE0_OFFSET_TRIGGER_MISSING_TEETH;
    const PAGE0_OFFSET_SECONDARY_ENABLED: usize = super::PAGE0_OFFSET_TRIGGER_SECONDARY_ENABLED;

    fn seeded_page0() -> Vec<u8> {
        let mut page = vec![0u8; 512];
        page[0..4].copy_from_slice(&PAGE0_MAGIC);
        page[4] = PAGE0_VERSION;
        page
    }

    #[test]
    fn defaults_to_generic_missing_tooth() {
        let mut runtime = TriggerRuntime::default();
        let capture = runtime.trigger_capture(1);

        assert_eq!(capture.preset_key, "generic_60_2");
        assert!(capture.primary_edge_count > 0);
        assert_eq!(capture.primary_samples.len(), capture.secondary_samples.len());
    }

    #[test]
    fn distributor_mapping_uses_engine_cylinder_count() {
        let mut runtime = TriggerRuntime::default();
        let mut page = seeded_page0();
        page[PAGE0_OFFSET_ENGINE_CYLINDERS] = 8;
        page[PAGE0_OFFSET_TRIGGER_TYPE] = TRIGGER_TYPE_DISTRIBUTOR_4CYL;

        runtime.apply_page0_payload(&page);
        let capture = runtime.trigger_capture(2);
        assert_eq!(capture.preset_key, "distributor_basic_8");
    }

    #[test]
    fn single_tooth_maps_to_dual_wheel_profile() {
        let mut runtime = TriggerRuntime::default();
        let mut page = seeded_page0();
        page[PAGE0_OFFSET_TRIGGER_TYPE] = TRIGGER_TYPE_SINGLE_TOOTH;
        page[PAGE0_OFFSET_SECONDARY_ENABLED] = 1;
        page[PAGE0_OFFSET_TOTAL_TEETH..PAGE0_OFFSET_TOTAL_TEETH + 2]
            .copy_from_slice(&24u16.to_be_bytes());

        runtime.apply_page0_payload(&page);
        let capture = runtime.trigger_capture(3);
        assert_eq!(capture.preset_key, "dual_wheel");
        assert!(capture.secondary_edge_count > 0);
    }

    #[test]
    fn missing_tooth_36_1_selects_generic_36_1() {
        let mut runtime = TriggerRuntime::default();
        let mut page = seeded_page0();
        page[PAGE0_OFFSET_TRIGGER_TYPE] = TRIGGER_TYPE_MISSING_TOOTH;
        page[PAGE0_OFFSET_TOTAL_TEETH..PAGE0_OFFSET_TOTAL_TEETH + 2]
            .copy_from_slice(&36u16.to_be_bytes());
        page[PAGE0_OFFSET_MISSING_TEETH..PAGE0_OFFSET_MISSING_TEETH + 2]
            .copy_from_slice(&1u16.to_be_bytes());

        runtime.apply_page0_payload(&page);
        let capture = runtime.trigger_capture(4);
        assert_eq!(capture.preset_key, "generic_36_1");
        assert!(capture.sync_gap_tooth_count >= 1);
    }
}
