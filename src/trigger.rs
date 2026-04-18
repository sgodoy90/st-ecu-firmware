#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerDecoderPreset {
    pub key: &'static str,
    pub label: &'static str,
    pub family: &'static str,
    pub decoder: &'static str,
    pub pattern_kind: &'static str,
    pub primary_input_label: &'static str,
    pub secondary_input_label: Option<&'static str>,
    pub primary_sensor_kind: &'static str,
    pub secondary_sensor_kind: Option<&'static str>,
    pub edge_policy: &'static str,
    pub sync_strategy: &'static str,
    pub primary_pattern_hint: &'static str,
    pub secondary_pattern_hint: Option<&'static str>,
    pub reference_description: &'static str,
    pub expected_engine_cycle_deg: u16,
    pub requires_secondary: bool,
    pub supports_sequential: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerCapture {
    pub preset_key: &'static str,
    pub preset_label: &'static str,
    pub sync_state: &'static str,
    pub trigger_rpm: u16,
    pub sync_loss_counter: u32,
    pub synced_cycles: u32,
    pub engine_cycle_deg: u16,
    pub capture_window_us: u32,
    pub sample_period_us: u16,
    pub primary_label: &'static str,
    pub secondary_label: Option<&'static str>,
    pub tooth_count: u16,
    pub sync_gap_tooth_count: u8,
    pub primary_edge_count: u16,
    pub secondary_edge_count: u16,
    pub primary_samples: Vec<u8>,
    pub secondary_samples: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriggerToothLog {
    pub preset_key: &'static str,
    pub preset_label: &'static str,
    pub sync_state: &'static str,
    pub trigger_rpm: u16,
    pub engine_cycle_deg: u16,
    pub primary_label: &'static str,
    pub secondary_label: Option<&'static str>,
    pub reference_event_index: u16,
    pub dominant_gap_ratio: f32,
    pub tooth_intervals_us: Vec<u32>,
    pub secondary_event_indexes: Vec<u16>,
}

pub const SUPPORTED_TRIGGER_DECODERS: [TriggerDecoderPreset; 16] = [
    TriggerDecoderPreset {
        key: "generic_60_2",
        label: "Generic 60-2 + Home",
        family: "Universal Missing-Tooth",
        decoder: "missing_tooth_60_2",
        pattern_kind: "missing_tooth",
        primary_input_label: "Crank VR/Hall",
        secondary_input_label: Some("Cam Home"),
        primary_sensor_kind: "vr_or_hall",
        secondary_sensor_kind: Some("hall_or_optical"),
        edge_policy: "configurable",
        sync_strategy: "missing_tooth_plus_home",
        primary_pattern_hint: "60-2 crank wheel on the primary input",
        secondary_pattern_hint: Some("Single home or cam-sync event every 720 deg"),
        reference_description:
            "Locks on the missing-tooth gap and confirms engine phase from the home input.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "honda_k20_12_1",
        label: "Honda K20 / K24",
        family: "Honda K-Series",
        decoder: "oem_honda_k_12_1",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (CKP)",
        secondary_input_label: Some("Cam / TDC (CMP)"),
        primary_sensor_kind: "hall",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "ckp_plus_cmp_phase",
        primary_pattern_hint: "12 CKP windows on the crank pattern",
        secondary_pattern_hint: Some("Honda K cam and TDC phase windows"),
        reference_description:
            "Uses CKP window timing plus CMP phase windows to identify the full engine cycle.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "toyota_36_2_2_2",
        label: "Toyota 36-2-2-2",
        family: "Toyota VVT-i",
        decoder: "oem_toyota_36_2_2_2",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (NE)",
        secondary_input_label: Some("Cam (G)"),
        primary_sensor_kind: "vr",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "ckp_plus_cmp_phase",
        primary_pattern_hint: "36-2-2-2 crank pattern on the NE input",
        secondary_pattern_hint: Some("Toyota G cam phase pulses"),
        reference_description:
            "Uses the NE gap structure and G cam pulses to synchronize crank position and phase.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "gm_24x",
        label: "GM 24x",
        family: "GM LS",
        decoder: "oem_gm_24x",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank 24x",
        secondary_input_label: Some("Cam Sync"),
        primary_sensor_kind: "hall",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "ckp_plus_cmp_phase",
        primary_pattern_hint: "24x crank pattern on the primary input",
        secondary_pattern_hint: Some("Single cam-sync event per engine cycle"),
        reference_description:
            "Synchronizes from the 24x crank pattern and validates engine phase from cam sync.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "nissan_360_slot",
        label: "Nissan 360 Slot",
        family: "Nissan CAS",
        decoder: "oem_nissan_360_slot",
        pattern_kind: "oem_pattern",
        primary_input_label: "Outer Track",
        secondary_input_label: Some("Inner Track"),
        primary_sensor_kind: "optical",
        secondary_sensor_kind: Some("optical"),
        edge_policy: "decoder_defined",
        sync_strategy: "dual_track_cas",
        primary_pattern_hint: "360-slot outer optical track",
        secondary_pattern_hint: Some("Inner optical sync track"),
        reference_description:
            "Combines outer-track position events with inner-track sync slots for full phase lock.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "subaru_6_7",
        label: "Subaru 6/7",
        family: "Subaru EJ",
        decoder: "oem_subaru_6_7",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (6-tooth)",
        secondary_input_label: Some("Cam (7-window)"),
        primary_sensor_kind: "hall",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "ckp_plus_cmp_phase",
        primary_pattern_hint: "Subaru 6-slot crank pattern on CKP",
        secondary_pattern_hint: Some("Subaru 7-window cam phase pattern"),
        reference_description: "Combines Subaru crank and cam windows for phase-accurate sync.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "mitsubishi_4g63",
        label: "Mitsubishi 4G63",
        family: "Mitsubishi EVO",
        decoder: "oem_mitsubishi_4g63",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (CAS primary)",
        secondary_input_label: Some("Cam (CAS phase)"),
        primary_sensor_kind: "optical",
        secondary_sensor_kind: Some("optical"),
        edge_policy: "decoder_defined",
        sync_strategy: "dual_track_cas",
        primary_pattern_hint: "4G63 optical/cas primary track",
        secondary_pattern_hint: Some("4G63 phase track for cylinder identification"),
        reference_description:
            "Uses the Mitsubishi CAS dual-track arrangement for full engine phase lock.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "mazda_36_2_1",
        label: "Mazda 36-2-1",
        family: "Mazda MZR",
        decoder: "oem_mazda_36_2_1",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (36-2-1)",
        secondary_input_label: Some("Cam Sync"),
        primary_sensor_kind: "hall",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "ckp_plus_cmp_phase",
        primary_pattern_hint: "Mazda 36-2-1 CKP sequence",
        secondary_pattern_hint: Some("Single CMP phase sync event"),
        reference_description: "Identifies missing-tooth subgrouping and validates phase with CMP.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "ford_st170",
        label: "Ford ST170",
        family: "Ford Zetec",
        decoder: "oem_ford_st170",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (36-1)",
        secondary_input_label: Some("Cam Sync"),
        primary_sensor_kind: "vr",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "missing_tooth_plus_home",
        primary_pattern_hint: "Ford 36-1 crank wheel",
        secondary_pattern_hint: Some("Single cam event per cycle"),
        reference_description: "Uses the 36-1 crank gap and cam-home event to synchronize phase.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "gm_58x",
        label: "GM LS 58x",
        family: "GM Gen IV / V LS",
        decoder: "oem_gm_58x",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank 58x",
        secondary_input_label: Some("Cam Sync"),
        primary_sensor_kind: "hall",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "ckp_plus_cmp_phase",
        primary_pattern_hint: "GM 58x crank pattern on CKP",
        secondary_pattern_hint: Some("Single cam-sync event per 720 deg"),
        reference_description:
            "Uses the GM 58x crank wheel and cam-sync pulse to lock crank position and phase.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "toyota_2jz_vvti",
        label: "Toyota 2JZ VVT-i",
        family: "Toyota JZ",
        decoder: "oem_toyota_2jz_vvti",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (36-2)",
        secondary_input_label: Some("Cam (VVT-i)"),
        primary_sensor_kind: "vr",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "missing_tooth_plus_home",
        primary_pattern_hint: "2JZ 36-2 crank wheel on NE input",
        secondary_pattern_hint: Some("Single VVT-i cam phase event per cycle"),
        reference_description:
            "Combines 36-2 crank tooth timing with cam phase sync for full 720 deg lock.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "bmw_m54_60_2",
        label: "BMW M54 / S54",
        family: "BMW M5x/S5x",
        decoder: "oem_bmw_m54_60_2",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (60-2)",
        secondary_input_label: Some("Dual VANOS Cam"),
        primary_sensor_kind: "hall",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "missing_tooth_plus_home",
        primary_pattern_hint: "BMW 60-2 crank pattern",
        secondary_pattern_hint: Some("Cam phase windows from VANOS cam sensors"),
        reference_description:
            "Uses 60-2 crank position and cam phase windows to resolve full cycle and phase.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "ford_coyote",
        label: "Ford Coyote 5.0",
        family: "Ford Modular Ti-VCT",
        decoder: "oem_ford_coyote",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (36-1)",
        secondary_input_label: Some("Dual Cam Sync"),
        primary_sensor_kind: "hall",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "missing_tooth_plus_home",
        primary_pattern_hint: "Ford 36-1 crank trigger",
        secondary_pattern_hint: Some("Ti-VCT cam phase windows"),
        reference_description:
            "Locks from the 36-1 crank gap and validates cylinder phase from cam windows.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "subaru_ej_36_2_2_2",
        label: "Subaru EJ 36-2-2-2",
        family: "Subaru EJ Phase 2",
        decoder: "oem_subaru_ej_36_2_2_2",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (36-2-2-2)",
        secondary_input_label: Some("Cam Phase"),
        primary_sensor_kind: "hall",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "ckp_plus_cmp_phase",
        primary_pattern_hint: "Subaru phase-2 36-2-2-2 CKP wheel",
        secondary_pattern_hint: Some("Subaru cam phase windows"),
        reference_description:
            "Decodes Subaru phase-2 gap pattern and cam windows for sequential phase lock.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "mazda_bp_4_1",
        label: "Mazda BP 4+1",
        family: "Mazda BP / Miata",
        decoder: "oem_mazda_bp_4_1",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (4-tooth)",
        secondary_input_label: Some("Cam (1-tooth)"),
        primary_sensor_kind: "hall_or_optical",
        secondary_sensor_kind: Some("hall_or_optical"),
        edge_policy: "decoder_defined",
        sync_strategy: "ckp_plus_cmp_phase",
        primary_pattern_hint: "4 evenly spaced crank events per engine revolution",
        secondary_pattern_hint: Some("Single cam phase event every 720 deg"),
        reference_description:
            "Mazda BP trigger family using a simple crank pattern plus cam phase marker.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
    TriggerDecoderPreset {
        key: "nissan_vq_36_2_2_2",
        label: "Nissan VQ 36-2-2-2",
        family: "Nissan VQ",
        decoder: "oem_nissan_vq_36_2_2_2",
        pattern_kind: "oem_pattern",
        primary_input_label: "Crank (36-2-2-2)",
        secondary_input_label: Some("Cam Phase"),
        primary_sensor_kind: "hall",
        secondary_sensor_kind: Some("hall"),
        edge_policy: "decoder_defined",
        sync_strategy: "ckp_plus_cmp_phase",
        primary_pattern_hint: "VQ crank wheel with 36-2-2-2 structure",
        secondary_pattern_hint: Some("Nissan cam phase events"),
        reference_description:
            "Uses VQ crank gap grouping and cam phase pulses to keep stable sync and phase.",
        expected_engine_cycle_deg: 720,
        requires_secondary: true,
        supports_sequential: true,
    },
];

fn pulse_train(sample_count: usize, starts: &[usize], width: usize) -> Vec<u8> {
    let mut samples = vec![0u8; sample_count];
    for start in starts {
        let end = (*start + width).min(sample_count);
        for sample in &mut samples[*start..end] {
            *sample = 1;
        }
    }
    samples
}

pub fn sample_trigger_capture() -> TriggerCapture {
    let sample_count = 120;
    let primary_starts = [4usize, 14, 24, 34, 44, 54, 64, 74, 84, 94, 104, 114];
    let secondary_starts = [22usize, 82];

    TriggerCapture {
        preset_key: "honda_k20_12_1",
        preset_label: "Honda K20 / K24",
        sync_state: "locked",
        trigger_rpm: 862,
        sync_loss_counter: 0,
        synced_cycles: 148,
        engine_cycle_deg: 720,
        capture_window_us: 9_000,
        sample_period_us: 75,
        primary_label: "Crank (CKP)",
        secondary_label: Some("Cam / TDC (CMP)"),
        tooth_count: 12,
        sync_gap_tooth_count: 0,
        primary_edge_count: 24,
        secondary_edge_count: 4,
        primary_samples: pulse_train(sample_count, &primary_starts, 2),
        secondary_samples: pulse_train(sample_count, &secondary_starts, 6),
    }
}

pub fn sample_trigger_tooth_log() -> TriggerToothLog {
    TriggerToothLog {
        preset_key: "honda_k20_12_1",
        preset_label: "Honda K20 / K24",
        sync_state: "locked",
        trigger_rpm: 862,
        engine_cycle_deg: 720,
        primary_label: "Crank (CKP)",
        secondary_label: Some("Cam / TDC (CMP)"),
        reference_event_index: 2,
        dominant_gap_ratio: 1.0,
        tooth_intervals_us: vec![697, 699, 701, 700, 698, 701, 699, 700, 702, 698, 700, 701],
        secondary_event_indexes: vec![2, 8],
    }
}

#[cfg(test)]
mod tests {
    use super::SUPPORTED_TRIGGER_DECODERS;

    #[test]
    fn decoder_catalog_includes_wave1_oem_families() {
        let expected_keys = [
            "generic_60_2",
            "honda_k20_12_1",
            "toyota_36_2_2_2",
            "nissan_360_slot",
            "gm_24x",
            "subaru_6_7",
            "mitsubishi_4g63",
            "mazda_36_2_1",
            "ford_st170",
            "gm_58x",
            "toyota_2jz_vvti",
            "bmw_m54_60_2",
            "ford_coyote",
            "subaru_ej_36_2_2_2",
            "nissan_vq_36_2_2_2",
            "mazda_bp_4_1",
        ];
        assert_eq!(SUPPORTED_TRIGGER_DECODERS.len(), expected_keys.len());
        for key in expected_keys {
            assert!(
                SUPPORTED_TRIGGER_DECODERS.iter().any(|preset| preset.key == key),
                "missing trigger decoder preset key: {key}"
            );
        }
    }

    #[test]
    fn decoder_catalog_preserves_key_decoder_contract() {
        let expected_pairs = [
            ("mitsubishi_4g63", "oem_mitsubishi_4g63"),
            ("subaru_ej_36_2_2_2", "oem_subaru_ej_36_2_2_2"),
            ("mazda_bp_4_1", "oem_mazda_bp_4_1"),
        ];
        for (key, decoder) in expected_pairs {
            let preset = SUPPORTED_TRIGGER_DECODERS
                .iter()
                .find(|preset| preset.key == key)
                .expect("expected preset key to exist");
            assert_eq!(
                preset.decoder, decoder,
                "decoder id mismatch for key {key}"
            );
        }
    }

    #[test]
    fn generic_missing_tooth_preset_remains_configurable() {
        let preset = SUPPORTED_TRIGGER_DECODERS
            .iter()
            .find(|preset| preset.key == "generic_60_2")
            .expect("generic_60_2 must be present");
        assert_eq!(preset.pattern_kind, "missing_tooth");
        assert_eq!(preset.edge_policy, "configurable");
        assert_eq!(preset.sync_strategy, "missing_tooth_plus_home");
        assert!(preset.requires_secondary);
        assert!(preset.supports_sequential);
    }
}
