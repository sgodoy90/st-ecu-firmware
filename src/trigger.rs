#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerDecoderPreset {
    pub key: &'static str,
    pub label: &'static str,
    pub family: &'static str,
    pub decoder: &'static str,
    pub pattern_kind: &'static str,
    pub primary_input_label: &'static str,
    pub secondary_input_label: Option<&'static str>,
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

pub const SUPPORTED_TRIGGER_DECODERS: [TriggerDecoderPreset; 5] = [
    TriggerDecoderPreset {
        key: "generic_60_2",
        label: "Generic 60-2 + Home",
        family: "Universal Missing-Tooth",
        decoder: "missing_tooth_60_2",
        pattern_kind: "missing_tooth",
        primary_input_label: "Crank VR/Hall",
        secondary_input_label: Some("Cam Home"),
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
