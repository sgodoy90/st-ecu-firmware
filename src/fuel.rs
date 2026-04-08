/// Fuel calculation — speed-density VE model with full correction stack.
///
/// Corrections applied multiplicatively (industry standard):
///   PW = base_pw × correction_iat × correction_clt × correction_baro
///        × correction_flex × stft × ltft × accel_enrich × wall_wetting_factor
///
/// LTFT storage:
///   F407: Flash (page 0, sector dedicated) — limited write cycles
///   H743: Backup SRAM (4KB VBAT-retained) — unlimited cycles, zero flash wear

pub const VE_TABLE_SIZE: usize = 48; // 48×48 RPM×MAP cells — matches protocol.ts TABLE_SIZE=48
pub const INJECTOR_OPEN_TIME_US: f32 = 700.0; // typical injector dead time at 14V

/// Fuel configuration — subset of config page
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuelConfig {
    /// Injector flow rate (cc/min at reference pressure)
    pub injector_flow_cc_min: f32,
    /// Reference fuel pressure (kPa)
    pub injector_ref_pressure_kpa: f32,
    /// Engine displacement (cc)
    pub displacement_cc: f32,
    /// Number of cylinders
    pub cylinders: u8,
    /// Injector dead time at 14V (µs)
    pub injector_dead_time_us: f32,
    /// Stoichiometric AFR (14.7 for gasoline, 9.0 for E85, etc.)
    pub stoich_afr: f32,
    /// Target lambda (1.0 = stoich)
    pub target_lambda: f32,
    /// Enable STFT closed-loop O2 correction
    pub closed_loop_enabled: bool,
    /// STFT authority limit (±%)
    pub stft_limit_pct: f32,
    /// LTFT authority limit (±%)
    pub ltft_limit_pct: f32,
    /// Wall wetting model enabled
    pub wall_wetting_enabled: bool,
    /// Aquino film time constant at 20°C (ms)
    pub wall_wetting_tau_ms: f32,
    /// Aquino X factor (fraction of fuel that doesn't adhere to wall)
    pub wall_wetting_x: f32,
    /// DFCO enabled
    pub dfco_enabled: bool,
    /// DFCO decel threshold (TPS %)
    pub dfco_tps_threshold_pct: f32,
    /// DFCO min CLT (°C)
    pub dfco_min_clt_c: f32,
    /// DFCO min RPM
    pub dfco_min_rpm: u16,
}

impl Default for FuelConfig {
    fn default() -> Self {
        Self {
            injector_flow_cc_min: 440.0,
            injector_ref_pressure_kpa: 300.0,
            displacement_cc: 1998.0,
            cylinders: 4,
            injector_dead_time_us: INJECTOR_OPEN_TIME_US,
            stoich_afr: 14.7,
            target_lambda: 1.0,
            closed_loop_enabled: true,
            stft_limit_pct: 25.0,
            ltft_limit_pct: 25.0,
            wall_wetting_enabled: true,
            wall_wetting_tau_ms: 80.0,
            wall_wetting_x: 0.3,
            dfco_enabled: true,
            dfco_tps_threshold_pct: 1.0,
            dfco_min_clt_c: 60.0,
            dfco_min_rpm: 1500,
        }
    }
}

/// 48×48 VE table (RPM × MAP load) — matches protocol.ts TABLE_SIZE=48
/// RAM: 48×48×4 + 48×4×2 = 9,600 bytes. Fine for H743 (1MB) and F407 (192KB).
#[derive(Debug, Clone, Copy)]
pub struct VeTable {
    pub cells: [[f32; VE_TABLE_SIZE]; VE_TABLE_SIZE],
    pub rpm_axis: [f32; VE_TABLE_SIZE],
    pub load_axis: [f32; VE_TABLE_SIZE],
}

impl Default for VeTable {
    fn default() -> Self {
        // 48 RPM breakpoints: 500..9000 RPM (dense at low end, sparse at high)
        let rpm_axis: [f32; VE_TABLE_SIZE] = [
            500.,  600.,  700.,  800.,  900., 1000., 1100., 1200.,
           1300., 1400., 1500., 1600., 1700., 1800., 1900., 2000.,
           2200., 2400., 2600., 2800., 3000., 3200., 3400., 3600.,
           3800., 4000., 4200., 4400., 4600., 4800., 5000., 5200.,
           5400., 5600., 5800., 6000., 6200., 6400., 6600., 6800.,
           7000., 7200., 7400., 7600., 7800., 8000., 8500., 9000.,
        ];
        // 48 load breakpoints: 20..310 kPa absolute
        let load_axis: [f32; VE_TABLE_SIZE] = [
            20.,  25.,  30.,  35.,  40.,  45.,  50.,  55.,
            60.,  65.,  70.,  75.,  80.,  85.,  90.,  95.,
           100., 105., 110., 115., 120., 125., 130., 135.,
           140., 145., 150., 155., 160., 165., 170., 175.,
           180., 185., 190., 195., 200., 210., 220., 230.,
           240., 250., 260., 270., 280., 290., 300., 310.,
        ];
        // Realistic VE shape: peak around 70% RPM + 80% load, drops at extremes
        let mut cells = [[80.0f32; VE_TABLE_SIZE]; VE_TABLE_SIZE];
        for r in 0..VE_TABLE_SIZE {
            for l in 0..VE_TABLE_SIZE {
                let rpm_norm  = r as f32 / (VE_TABLE_SIZE - 1) as f32;
                let load_norm = l as f32 / (VE_TABLE_SIZE - 1) as f32;
                let rpm_factor  = 1.0 - (rpm_norm - 0.70).powi(2) * 0.5;
                let load_factor = 0.5 + load_norm * 0.5;
                cells[r][l] = (65.0 + 25.0 * rpm_factor * load_factor).max(40.0).min(100.0);
            }
        }
        Self { cells, rpm_axis, load_axis }
    }
}

impl VeTable {
    /// Bilinear interpolation to get VE% for given RPM and MAP (kPa).
    pub fn interpolate(&self, rpm: f32, map_kpa: f32) -> f32 {
        let ri = axis_index(&self.rpm_axis, rpm);
        let li = axis_index(&self.load_axis, map_kpa);
        bilinear(
            ri.0, ri.1, ri.2,
            li.0, li.1, li.2,
            &self.cells,
        )
    }
}

/// Find surrounding axis indices and interpolation fraction.
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
    cells: &[[f32; VE_TABLE_SIZE]; VE_TABLE_SIZE],
) -> f32 {
    bilinear_4(cells[ri][li], cells[ri1][li], cells[ri][li1], cells[ri1][li1], rf, lf)
}

/// Slice-based bilinear — works for any table size (used by LtftMap 16×16).
fn bilinear_slice(
    cells: &[[f32; LTFT_SIZE]; LTFT_SIZE],
    ri: usize, ri1: usize, rf: f32,
    li: usize, li1: usize, lf: f32,
) -> f32 {
    bilinear_4(cells[ri][li], cells[ri1][li], cells[ri][li1], cells[ri1][li1], rf, lf)
}

#[inline(always)]
fn bilinear_4(c00: f32, c10: f32, c01: f32, c11: f32, rf: f32, lf: f32) -> f32 {
    c00 * (1.0 - rf) * (1.0 - lf)
        + c10 * rf * (1.0 - lf)
        + c01 * (1.0 - rf) * lf
        + c11 * rf * lf
}

// ─── LTFT storage layout in BBSRAM (H743) ────────────────────────────────────
// LTFT uses its OWN 16×16 grid — BBSRAM is only 4KB.
// 16×16×4 = 1024 bytes fits comfortably. VE table (48×48) is NOT stored in BBSRAM.
// Offset 0:    LTFT map    16×16 × 4 bytes = 1,024 bytes
// Offset 1024: Knock map   (KnockLearningMap::serialize) = 2,080 bytes
// Total: 3,104 bytes / 4,096 bytes BBSRAM used.

pub const LTFT_SIZE: usize = 16;          // 16×16 — BBSRAM-limited
pub const LTFT_BBSRAM_OFFSET: usize = 0;
pub const KNOCK_BBSRAM_OFFSET: usize = 1024;

/// 16×16 LTFT table stored in BBSRAM. Separate from VE table (48×48 in DTCM RAM).
#[derive(Debug, Clone, Copy)]
pub struct LtftMap {
    pub cells: [[f32; LTFT_SIZE]; LTFT_SIZE],
    /// Shared 16-bin RPM axis (subset of VE axis breakpoints)
    pub rpm_axis: [f32; LTFT_SIZE],
    /// Shared 16-bin load axis
    pub load_axis: [f32; LTFT_SIZE],
}

impl Default for LtftMap {
    fn default() -> Self {
        Self {
            cells: [[1.0f32; LTFT_SIZE]; LTFT_SIZE],
            rpm_axis: [500., 1000., 1500., 2000., 2500., 3000., 3500., 4000.,
                       4500., 5000., 5500., 6000., 6500., 7000., 7500., 8000.],
            load_axis: [20., 40., 60., 80., 100., 120., 140., 160.,
                        180., 200., 220., 240., 260., 280., 300., 310.],
        }
    }
}

impl LtftMap {
    pub fn serialize(&self) -> [u8; 1024] {
        let mut buf = [0u8; 1024]; // 16×16×4 = 1024
        let mut o = 0;
        for row in &self.cells {
            for &v in row {
                buf[o..o+4].copy_from_slice(&v.to_be_bytes());
                o += 4;
            }
        }
        buf
    }

    pub fn deserialize(buf: &[u8; 1024]) -> Self {
        let mut map = Self::default();
        let mut o = 0;
        for row in &mut map.cells {
            for v in row.iter_mut() {
                *v = f32::from_be_bytes([buf[o], buf[o+1], buf[o+2], buf[o+3]]);
                o += 4;
            }
        }
        map
    }

    pub fn get(&self, rpm: f32, map_kpa: f32) -> f32 {
        let ri = axis_index(&self.rpm_axis, rpm);
        let li = axis_index(&self.load_axis, map_kpa);
        bilinear_slice(&self.cells, ri.0, ri.1, ri.2, li.0, li.1, li.2)
    }

    pub fn update(&mut self, rpm: f32, map_kpa: f32, stft: f32, learn_rate: f32) {
        let ri = axis_index(&self.rpm_axis, rpm);
        let li = axis_index(&self.load_axis, map_kpa);
        let cell = &mut self.cells[ri.0][li.0];
        *cell = *cell * (1.0 - learn_rate) + (*cell * stft) * learn_rate;
        *cell = cell.max(0.75).min(1.25); // clamp ±25%
    }
}

// ─── Wall Wetting (Aquino model) ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
pub struct WallWettingState {
    /// Current film mass on port walls (mg)
    pub film_mass_mg: f32,
}

impl WallWettingState {
    /// Update wall fuel model and return corrected pulse width.
    /// tau_ms: film time constant (CLT-dependent lookup)
    /// x: fraction of injected fuel that doesn't adhere (goes direct to cylinder)
    pub fn update(&mut self, base_pw_ms: f32, tau_ms: f32, x: f32, dt_ms: f32) -> f32 {
        // Aquino model:
        // film_dot = x_evap * film - (1-x) * m_inj
        // Where x_evap = 1/tau
        let m_inj = base_pw_ms; // treat pw as proxy for injected mass
        // SAFETY: tau_ms guaranteed ≥ 0.25 by wall_wetting_tau(). Extra clamp for direct calls.
        let safe_tau = tau_ms.max(0.25);
        let evap = self.film_mass_mg / safe_tau * dt_ms;
        let deposit = (1.0 - x.clamp(0.0, 1.0)) * m_inj;
        let cylinder_fuel = x.clamp(0.0, 1.0) * m_inj + evap; // fuel that reaches cylinder
        self.film_mass_mg = (self.film_mass_mg - evap + deposit).max(0.0);
        // Correction factor: clamp denominator to avoid division by near-zero or NaN
        let denom = cylinder_fuel.max(base_pw_ms * 0.01).max(0.001);
        if base_pw_ms > 0.001 {
            (base_pw_ms * base_pw_ms / denom).clamp(base_pw_ms * 0.5, base_pw_ms * 3.0)
        } else {
            base_pw_ms
        }
    }
}

// ─── Fuel Calculator ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
pub struct FuelState {
    pub ve: f32,
    pub base_pw_ms: f32,
    pub final_pw_ms: f32,
    pub duty_pct: f32,
    pub stft: f32,      // current STFT value (multiplicative)
    pub ltft: f32,      // current LTFT value
    pub accel_enrich: f32,
    pub dfco_active: bool,
    pub correction_iat: f32,
    pub correction_clt: f32,
    pub correction_baro: f32,
    pub correction_flex: f32,
    pub wall_state: WallWettingState,
}

pub struct FuelCalculator {
    pub config: FuelConfig,
    pub ve_table: VeTable,
    pub ltft_map: LtftMap,
}

impl FuelCalculator {
    pub fn new(config: FuelConfig) -> Self {
        Self {
            config,
            ve_table: VeTable::default(),
            ltft_map: LtftMap::default(),
        }
    }

    /// Calculate injector pulse width for current conditions.
    pub fn calculate(
        &mut self,
        state: &mut FuelState,
        rpm: f32,
        map_kpa: f32,
        baro_kpa: f32,
        clt_c: f32,
        iat_c: f32,
        flex_pct: f32,
        vbatt: f32,
        tps_pct: f32,
        lambda_measured: f32,
        dt_ms: f32,
    ) {
        // DFCO check
        if self.config.dfco_enabled
            && tps_pct < self.config.dfco_tps_threshold_pct
            && rpm as u16 > self.config.dfco_min_rpm
            && clt_c > self.config.dfco_min_clt_c
        {
            state.dfco_active = true;
            state.final_pw_ms = 0.0;
            state.duty_pct = 0.0;
            return;
        }
        state.dfco_active = false;

        // VE interpolation
        state.ve = self.ve_table.interpolate(rpm, map_kpa);

        // Base pulse width (speed-density)
        // PW = (VE × MAP × displacement) / (rpm × cylinders × stoich × injector_flow)
        let air_charge_g = (state.ve / 100.0) * map_kpa * self.config.displacement_cc * 1e-6
            / (8.314 * (iat_c + 273.15)) * 28.97 * 1000.0; // air mass in grams per event
        let fuel_mass_g = air_charge_g / (self.config.stoich_afr * self.config.target_lambda);
        // Injector flow in g/ms at reference pressure
        let injector_flow_g_ms = (self.config.injector_flow_cc_min * 0.745) / 60000.0; // density ~0.745 g/cc
        state.base_pw_ms = fuel_mass_g / injector_flow_g_ms.max(0.001);

        // Corrections
        state.correction_iat = iat_correction(iat_c);
        state.correction_clt = clt_correction(clt_c);
        state.correction_baro = baro_correction(baro_kpa, map_kpa);
        state.correction_flex = flex_correction(flex_pct);

        // STFT (closed-loop O2)
        if self.config.closed_loop_enabled && lambda_measured > 0.5 && lambda_measured < 1.5 {
            let lambda_error = self.config.target_lambda - lambda_measured;
            let stft_delta = lambda_error * 0.1; // proportional gain
            state.stft = (state.stft + stft_delta)
                .max(1.0 - self.config.stft_limit_pct / 100.0)
                .min(1.0 + self.config.stft_limit_pct / 100.0);
            // Update LTFT slowly
            self.ltft_map.update(rpm, map_kpa, state.stft, 0.001);
        }
        state.ltft = self.ltft_map.get(rpm, map_kpa);

        // Wall wetting correction
        let tau = wall_wetting_tau(clt_c, self.config.wall_wetting_tau_ms);
        let corrected_pw = if self.config.wall_wetting_enabled {
            state.wall_state.update(state.base_pw_ms, tau, self.config.wall_wetting_x, dt_ms)
        } else {
            state.base_pw_ms
        };

        // Injector dead time compensation (voltage-dependent)
        let dead_time_ms = injector_dead_time_ms(vbatt, self.config.injector_dead_time_us);

        // Final pulse width
        state.final_pw_ms = (corrected_pw
            * state.correction_iat
            * state.correction_clt
            * state.correction_baro
            * state.correction_flex
            * state.stft
            * state.ltft
            * (1.0 + state.accel_enrich))
            + dead_time_ms;
        state.final_pw_ms = state.final_pw_ms.max(0.0);

        // Duty cycle
        let cycle_ms = if rpm > 0.0 { 60000.0 / (rpm * self.config.cylinders as f32) } else { 100.0 };
        state.duty_pct = (state.final_pw_ms / cycle_ms * 100.0).min(100.0);
    }
}

// ─── Correction Tables ────────────────────────────────────────────────────────

/// IAT correction: denser air = more fuel at cold intake
fn iat_correction(iat_c: f32) -> f32 {
    // Reference: 25°C → 1.0 multiplier
    let ref_temp_k = 298.15;
    let actual_k = iat_c + 273.15;
    (ref_temp_k / actual_k).sqrt().max(0.7).min(1.3)
}

/// CLT warmup correction (additive enrichment during warmup)
fn clt_correction(clt_c: f32) -> f32 {
    if clt_c >= 80.0 { return 1.0; }
    // Linear enrichment: 40% extra at -40°C, 0% extra at 80°C
    let enrich = ((80.0 - clt_c) / 120.0 * 0.4).max(0.0);
    1.0 + enrich
}

/// Barometric correction
fn baro_correction(baro_kpa: f32, map_kpa: f32) -> f32 {
    let ref_baro = 101.325;
    if map_kpa > 80.0 {
        baro_kpa / ref_baro
    } else {
        1.0 // at light load, MAP is already the primary pressure signal
    }
}

/// Flex fuel correction (E0=1.0, E100≈1.36)
fn flex_correction(flex_pct: f32) -> f32 {
    1.0 + (flex_pct / 100.0) * 0.36
}

/// Injector dead time vs battery voltage (simplified linear model)
fn injector_dead_time_ms(vbatt: f32, base_dead_us: f32) -> f32 {
    let ref_voltage = 14.0;
    let compensation = (ref_voltage - vbatt) * 50.0; // 50 µs per volt deviation
    ((base_dead_us + compensation) / 1000.0).max(0.0)
}

/// Wall wetting tau vs CLT (higher tau = more film at cold temps)
fn wall_wetting_tau(clt_c: f32, base_tau_ms: f32) -> f32 {
    // Tau doubles at 0°C, halves at 100°C relative to base (at 20°C)
    let factor = if clt_c < 20.0 {
        1.0 + (20.0 - clt_c) / 20.0
    } else {
        (1.0 - (clt_c - 20.0) / 80.0).max(0.5)
    };
    // SAFETY: clamp tau ≥ 0.25 ms — prevents division-by-zero in WallWettingState::update()
    // At extreme hot (>100°C) factor could theoretically reach 0; clamp ensures safe evap calc.
    (base_tau_ms * factor).max(0.25)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ve_table_default_interpolate_midrange() {
        let table = VeTable::default();
        let ve = table.interpolate(3500.0, 90.0);
        assert!(ve > 40.0 && ve <= 100.0, "VE {ve} out of expected range");
    }

    #[test]
    fn iat_correction_at_reference_temp() {
        let corr = iat_correction(25.0);
        assert!((corr - 1.0).abs() < 0.02, "IAT correction at 25°C should be ~1.0, got {corr}");
    }

    #[test]
    fn clt_correction_fully_warm() {
        let corr = clt_correction(80.0);
        assert_eq!(corr, 1.0);
    }

    #[test]
    fn clt_correction_cold_enriched() {
        let corr = clt_correction(-20.0);
        assert!(corr > 1.0, "cold engine should have enrichment > 1.0");
    }

    #[test]
    fn flex_correction_e100() {
        let corr = flex_correction(100.0);
        assert!((corr - 1.36).abs() < 0.01);
    }

    #[test]
    fn flex_correction_e0_is_unity() {
        let corr = flex_correction(0.0);
        assert_eq!(corr, 1.0);
    }

    #[test]
    fn ltft_map_serialize_roundtrip() {
        let mut map = LtftMap::default();
        map.cells[2][4] = 1.08;
        let buf = map.serialize();
        let restored = LtftMap::deserialize(&buf);
        assert!((restored.cells[2][4] - 1.08).abs() < 1e-5);
    }

    #[test]
    fn ltft_clamped_to_25pct() {
        let mut map = LtftMap::default();
        for _ in 0..1000 {
            map.update(3000.0, 90.0, 1.5, 0.1); // extreme STFT
        }
        let v = map.cells[4][4];
        assert!(v <= 1.26, "LTFT exceeded +25%: {v}");
    }

    #[test]
    fn wall_wetting_film_accumulates() {
        let mut state = WallWettingState::default();
        for _ in 0..10 {
            state.update(3.0, 80.0, 0.3, 10.0);
        }
        assert!(state.film_mass_mg > 0.0);
    }

    #[test]
    fn wall_wetting_tau_extreme_cold_is_safe() {
        // At -40°C, factor = 1 + (20-(-40))/20 = 4.0 → tau = 80*4 = 320ms — no issue
        let tau = wall_wetting_tau(-40.0, 80.0);
        assert!(tau >= 0.25, "tau should never go below 0.25ms, got {tau}");
        assert!(tau > 100.0, "at -40°C tau should be large (>100ms), got {tau}");
    }

    #[test]
    fn wall_wetting_tau_extreme_hot_clamped() {
        // At 200°C (beyond engine range), factor could go very low — must be clamped
        let tau = wall_wetting_tau(200.0, 80.0);
        assert!(tau >= 0.25, "tau must be >= 0.25ms even at extreme temp, got {tau}");
    }

    #[test]
    fn wall_wetting_update_no_nan_or_inf() {
        let mut state = WallWettingState::default();
        // Edge case: tiny base_pw
        let result = state.update(0.0001, 0.3, 0.5, 5.0);
        assert!(result.is_finite(), "result should be finite, got {result}");
    }

    #[test]
    fn dfco_cuts_fuel() {
        let config = FuelConfig::default();
        let mut calc = FuelCalculator::new(config);
        let mut state = FuelState::default();
        state.stft = 1.0;
        state.ltft = 1.0;
        calc.calculate(&mut state, 3000.0, 30.0, 101.0, 85.0, 25.0, 0.0, 14.0, 0.5, 1.0, 10.0);
        // TPS < threshold AND RPM > dfco_min → DFCO active
        assert!(state.dfco_active);
        assert_eq!(state.final_pw_ms, 0.0);
    }
}
