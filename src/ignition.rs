/// Ignition scheduling — advance calculation, dwell, per-cylinder corrections.
///
/// Timer precision:
///   F407: ~1 µs (GP timer at 84 MHz APB2)
///   H743: 217 ps (HRTIM at 4.608 GHz) — 4600× better resolution
///
/// Corrections applied in this order:
///   base_advance + correction_clt + correction_iat + correction_knock_retard
///   + correction_idle_ign + correction_flat_shift

pub const IGN_TABLE_SIZE: usize = 48; // 48×48 — matches protocol.ts TABLE_SIZE=48

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IgnitionMode {
    #[default]
    WastedSpark,    // pairs of cylinders, 360° apart
    CoilOnPlug,     // sequential, one coil per cylinder
    HrtimCop,       // H743 only: COP with 217ps HRTIM scheduling
}

/// 48×48 Ignition timing table (RPM × load) — matches protocol.ts TABLE_SIZE=48
/// RAM: 48×48×4 + 48×4×2 = 9,600 bytes.
#[derive(Debug, Clone, Copy)]
pub struct IgnitionTable {
    pub cells: [[f32; IGN_TABLE_SIZE]; IGN_TABLE_SIZE],
    pub rpm_axis: [f32; IGN_TABLE_SIZE],
    pub load_axis: [f32; IGN_TABLE_SIZE],
}

impl Default for IgnitionTable {
    fn default() -> Self {
        // Same 48-bin axes as VeTable for consistent cursor mapping in MapEditor
        let rpm_axis: [f32; IGN_TABLE_SIZE] = [
            500.,  600.,  700.,  800.,  900., 1000., 1100., 1200.,
           1300., 1400., 1500., 1600., 1700., 1800., 1900., 2000.,
           2200., 2400., 2600., 2800., 3000., 3200., 3400., 3600.,
           3800., 4000., 4200., 4400., 4600., 4800., 5000., 5200.,
           5400., 5600., 5800., 6000., 6200., 6400., 6600., 6800.,
           7000., 7200., 7400., 7600., 7800., 8000., 8500., 9000.,
        ];
        let load_axis: [f32; IGN_TABLE_SIZE] = [
            20.,  25.,  30.,  35.,  40.,  45.,  50.,  55.,
            60.,  65.,  70.,  75.,  80.,  85.,  90.,  95.,
           100., 105., 110., 115., 120., 125., 130., 135.,
           140., 145., 150., 155., 160., 165., 170., 175.,
           180., 185., 190., 195., 200., 210., 220., 230.,
           240., 250., 260., 270., 280., 290., 300., 310.,
        ];
        // Realistic ignition map: peak advance at light load + mid-RPM, drops at high load
        let mut cells = [[20.0f32; IGN_TABLE_SIZE]; IGN_TABLE_SIZE];
        for r in 0..IGN_TABLE_SIZE {
            for l in 0..IGN_TABLE_SIZE {
                let rpm_norm  = r as f32 / (IGN_TABLE_SIZE - 1) as f32;
                let load_norm = l as f32 / (IGN_TABLE_SIZE - 1) as f32;
                // Peak ~32° at 40% RPM light load → 10° at WOT high RPM
                cells[r][l] = (15.0 + 20.0 * (1.0 - load_norm)
                    * (1.0 - (rpm_norm - 0.4).powi(2) * 2.5))
                    .max(5.0).min(40.0);
            }
        }
        Self { cells, rpm_axis, load_axis }
    }
}

impl IgnitionTable {
    pub fn interpolate(&self, rpm: f32, load_kpa: f32) -> f32 {
        let ri = axis_index(&self.rpm_axis, rpm);
        let li = axis_index(&self.load_axis, load_kpa);
        bilinear(ri.0, ri.1, ri.2, li.0, li.1, li.2, &self.cells)
    }
}

fn axis_index(axis: &[f32], value: f32) -> (usize, usize, f32) {
    let n = axis.len();
    if value <= axis[0] { return (0, 0, 0.0); }
    if value >= axis[n-1] { return (n-1, n-1, 0.0); }
    for i in 0..n-1 {
        if value >= axis[i] && value < axis[i+1] {
            let frac = (value - axis[i]) / (axis[i+1] - axis[i]);
            return (i, i+1, frac);
        }
    }
    (n-1, n-1, 0.0)
}

fn bilinear(
    ri: usize, ri1: usize, rf: f32,
    li: usize, li1: usize, lf: f32,
    cells: &[[f32; IGN_TABLE_SIZE]; IGN_TABLE_SIZE],
) -> f32 {
    let c00 = cells[ri][li];
    let c10 = cells[ri1][li];
    let c01 = cells[ri][li1];
    let c11 = cells[ri1][li1];
    c00 * (1.0 - rf) * (1.0 - lf)
        + c10 * rf * (1.0 - lf)
        + c01 * (1.0 - rf) * lf
        + c11 * rf * lf
}

/// Dwell table: coil charge time vs battery voltage
#[derive(Debug, Clone, Copy)]
pub struct DwellTable {
    pub voltage: [f32; 8],
    pub dwell_ms: [f32; 8],
}

impl Default for DwellTable {
    fn default() -> Self {
        Self {
            voltage: [8.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0],
            dwell_ms: [5.0, 4.5, 4.0, 3.5, 3.2, 3.0, 2.8, 2.6],
        }
    }
}

impl DwellTable {
    pub fn lookup(&self, vbatt: f32) -> f32 {
        let n = self.voltage.len();
        if vbatt <= self.voltage[0] { return self.dwell_ms[0]; }
        if vbatt >= self.voltage[n-1] { return self.dwell_ms[n-1]; }
        for i in 0..n-1 {
            if vbatt >= self.voltage[i] && vbatt < self.voltage[i+1] {
                let frac = (vbatt - self.voltage[i]) / (self.voltage[i+1] - self.voltage[i]);
                return self.dwell_ms[i] * (1.0 - frac) + self.dwell_ms[i+1] * frac;
            }
        }
        self.dwell_ms[n-1]
    }
}

/// Ignition configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IgnitionConfig {
    pub mode: IgnitionMode,
    pub cylinders: u8,
    pub firing_order: [u8; 8], // 0-indexed
    pub cranking_advance_deg: f32,
    pub cranking_rpm_threshold: u16,
    pub flat_shift_retard_deg: f32,
    /// Gear ignition retard per gear (degrees, during gear change)
    pub gear_retard_deg: [f32; 8],
    pub gear_cut_duration_ms: [u16; 8],
}

impl Default for IgnitionConfig {
    fn default() -> Self {
        Self {
            mode: IgnitionMode::CoilOnPlug,
            cylinders: 4,
            firing_order: [1, 3, 4, 2, 0, 0, 0, 0], // 1-3-4-2 typical inline-4
            cranking_advance_deg: 10.0,
            cranking_rpm_threshold: 400,
            flat_shift_retard_deg: 20.0,
            gear_retard_deg: [0.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0],
            gear_cut_duration_ms: [0, 60, 60, 70, 70, 80, 80, 80],
        }
    }
}

/// Ignition state — updated every tooth event
#[derive(Debug, Clone, Copy, Default)]
pub struct IgnitionState {
    pub advance_deg: f32,
    pub dwell_ms: f32,
    pub injection_offset_deg: f32,
    pub knock_retard_deg: f32,     // total applied across all cylinders
    pub idle_ign_correction_deg: f32,
    pub per_cyl_retard: [f32; 8], // individual per-cylinder knock retard
    pub cranking_mode: bool,
    pub gear_cut_active: bool,
    pub gear_cut_timer_ms: u16,
}

pub struct IgnitionScheduler {
    pub config: IgnitionConfig,
    pub advance_table: IgnitionTable,
    pub dwell_table: DwellTable,
}

impl IgnitionScheduler {
    pub fn new(config: IgnitionConfig) -> Self {
        Self {
            config,
            advance_table: IgnitionTable::default(),
            dwell_table: DwellTable::default(),
        }
    }

    /// Calculate ignition advance and dwell for current conditions.
    pub fn calculate(
        &self,
        state: &mut IgnitionState,
        rpm: f32,
        load_kpa: f32,
        clt_c: f32,
        iat_c: f32,
        vbatt: f32,
        knock_retard: f32,
        idle_ign_correction: f32,
        flat_shift_active: bool,
    ) {
        state.cranking_mode = rpm < self.config.cranking_rpm_threshold as f32;

        if state.cranking_mode {
            state.advance_deg = self.config.cranking_advance_deg;
        } else {
            let base = self.advance_table.interpolate(rpm, load_kpa);
            let corr_clt = ign_clt_correction(clt_c);
            let corr_iat = ign_iat_correction(iat_c);
            let flat_shift_retard = if flat_shift_active { self.config.flat_shift_retard_deg } else { 0.0 };

            state.advance_deg = (base + corr_clt + corr_iat - knock_retard - flat_shift_retard
                + idle_ign_correction).max(0.0).min(60.0);
        }

        state.knock_retard_deg = knock_retard;
        state.idle_ign_correction_deg = idle_ign_correction;
        state.dwell_ms = self.dwell_table.lookup(vbatt);
    }

    /// Schedule injection offset (degrees after TDC for sequential injection).
    pub fn injection_offset(&self, cylinder: usize, advance_deg: f32) -> f32 {
        // Injection phased relative to intake valve opening
        // Simplified: 120° BTDC intake, adjusted by advance
        let base_offset = 340.0; // degrees BTDC typical MPFI target
        base_offset - advance_deg
    }

    /// Convert degrees-before-TDC to timer nanoseconds for a given RPM.
    pub fn deg_to_ns(deg: f32, rpm: f32) -> u64 {
        if rpm < 1.0 { return 0; }
        let us_per_deg = 1_000_000.0 / (rpm * 6.0); // 6 deg/ms at 1000 RPM
        (deg * us_per_deg * 1000.0) as u64
    }
}

/// CLT-based ignition correction (retard when hot, slight advance when cold)
fn ign_clt_correction(clt_c: f32) -> f32 {
    if clt_c >= 80.0 { return 0.0; }
    if clt_c < 20.0 { return -3.0; } // warm-up retard for stability
    (80.0 - clt_c) / 60.0 * (-3.0) // linear from -3° at 20°C to 0° at 80°C
}

/// IAT-based ignition correction (retard when intake air is hot)
fn ign_iat_correction(iat_c: f32) -> f32 {
    if iat_c <= 25.0 { return 0.0; }
    // -1° per 10°C above 25°C, max -5°
    ((25.0 - iat_c) / 10.0).max(-5.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignition_table_default_range() {
        let table = IgnitionTable::default();
        for row in &table.cells {
            for &v in row {
                assert!(v >= 5.0 && v <= 40.0, "advance {v} out of range");
            }
        }
    }

    #[test]
    fn dwell_table_higher_voltage_lower_dwell() {
        let table = DwellTable::default();
        let low = table.lookup(10.0);
        let high = table.lookup(14.0);
        assert!(high < low, "higher voltage should mean less dwell");
    }

    #[test]
    fn ign_clt_correction_warm_engine() {
        assert_eq!(ign_clt_correction(90.0), 0.0);
    }

    #[test]
    fn ign_iat_correction_cold_intake() {
        assert_eq!(ign_iat_correction(20.0), 0.0);
    }

    #[test]
    fn ign_iat_correction_hot_intake() {
        let corr = ign_iat_correction(65.0);
        assert!(corr < 0.0, "hot intake should retard timing");
    }

    #[test]
    fn deg_to_ns_at_6000rpm() {
        // At 6000 RPM: 1 rev = 10ms, 1 deg = 27.78µs
        let ns = IgnitionScheduler::deg_to_ns(1.0, 6000.0);
        let expected = 27_778; // ns
        assert!((ns as i64 - expected as i64).abs() < 500, "got {ns}ns, expected ~{expected}ns");
    }

    #[test]
    fn cranking_mode_uses_fixed_advance() {
        let config = IgnitionConfig::default();
        let sched = IgnitionScheduler::new(config);
        let mut state = IgnitionState::default();
        sched.calculate(&mut state, 200.0, 70.0, 20.0, 25.0, 14.0, 0.0, 0.0, false);
        assert!(state.cranking_mode);
        assert!((state.advance_deg - config.cranking_advance_deg).abs() < 0.01);
    }

    #[test]
    fn knock_retard_reduces_advance() {
        let config = IgnitionConfig::default();
        let sched = IgnitionScheduler::new(config);
        let mut state_no_knock = IgnitionState::default();
        let mut state_knock = IgnitionState::default();
        sched.calculate(&mut state_no_knock, 3000.0, 90.0, 85.0, 25.0, 14.0, 0.0, 0.0, false);
        sched.calculate(&mut state_knock, 3000.0, 90.0, 85.0, 25.0, 14.0, 5.0, 0.0, false);
        assert!(state_knock.advance_deg < state_no_knock.advance_deg);
    }

    #[test]
    fn flat_shift_retards_ignition() {
        let config = IgnitionConfig::default();
        let sched = IgnitionScheduler::new(config);
        let mut state = IgnitionState::default();
        let mut state_flat = IgnitionState::default();
        sched.calculate(&mut state, 5000.0, 120.0, 85.0, 30.0, 14.0, 0.0, 0.0, false);
        sched.calculate(&mut state_flat, 5000.0, 120.0, 85.0, 30.0, 14.0, 0.0, 0.0, true);
        assert!(state_flat.advance_deg < state.advance_deg);
    }
}
