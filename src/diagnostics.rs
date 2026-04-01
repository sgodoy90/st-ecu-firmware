#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DtcSeverity {
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtcCode {
    pub code: &'static str,
    pub severity: DtcSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreezeFrameHeader {
    pub rev_counter: u32,
    pub reason_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreezeFrame {
    pub code: &'static str,
    pub label: &'static str,
    pub reason: &'static str,
    pub rev_counter: u32,
    pub rpm: u16,
    pub map_kpa_x10: u16,
    pub tps_pct_x100: u16,
    pub coolant_c_x10: i16,
    pub lambda_x10000: u16,
    pub vbatt_x100: u16,
    pub gear: u8,
}

pub const SAMPLE_FREEZE_FRAMES: [FreezeFrame; 2] = [
    FreezeFrame {
        code: "P0118",
        label: "Coolant Temp Sensor High",
        reason: "sensor_plausibility",
        rev_counter: 48_231,
        rpm: 2_840,
        map_kpa_x10: 572,
        tps_pct_x100: 1460,
        coolant_c_x10: -318,
        lambda_x10000: 10_720,
        vbatt_x100: 1418,
        gear: 3,
    },
    FreezeFrame {
        code: "P0193",
        label: "Fuel Pressure Sensor High",
        reason: "pressure_range_high",
        rev_counter: 50_984,
        rpm: 4_125,
        map_kpa_x10: 1_834,
        tps_pct_x100: 6_230,
        coolant_c_x10: 862,
        lambda_x10000: 9_180,
        vbatt_x100: 1394,
        gear: 4,
    },
];
