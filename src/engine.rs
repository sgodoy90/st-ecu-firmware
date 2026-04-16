/// Engine runtime state — master struct aggregating all control loop states.
/// This is the central data structure passed between ISR callbacks and RTOS tasks.

// ─── Sync / Trigger ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncState {
    #[default]
    Unsynced,
    CrankOnly,
    FullSync,
}

// ─── Fuel ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FuelState {
    /// Volumetric efficiency from interpolated VE table (0.0–2.0)
    pub ve: f32,
    /// Final injector pulse-width in ms after all corrections
    pub pulsewidth_ms: f32,
    /// Injector duty cycle (0.0–1.0)
    pub duty: f32,
    /// Short-term fuel trim (multiplicative, 1.0 = no trim)
    pub stft: f32,
    /// Long-term fuel trim (multiplicative, 1.0 = no trim)
    pub ltft: f32,
    /// Acceleration enrichment adder (0.0–1.0)
    pub accel_enrich: f32,
    /// Wall-wetting film mass in mg (Aquino model)
    pub wall_fuel_mg: f32,
    /// Active corrections multiplied together
    pub correction_iat: f32,
    pub correction_clt: f32,
    pub correction_baro: f32,
    pub correction_flex: f32,
    /// DFCO active flag
    pub dfco_active: bool,
    /// Closed-loop O2 active
    pub closed_loop: bool,
}

/// Per-cylinder fuel trim (H743 only — 8 cylinders max)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PerCylinderTrim {
    pub trim: [f32; 8], // multiplicative, 1.0 = no trim
    pub enabled: [bool; 8],
}

// ─── Ignition ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IgnitionState {
    /// Final ignition advance in degrees BTDC
    pub advance_deg: f32,
    /// Dwell time in ms
    pub dwell_ms: f32,
    /// Injection offset in degrees (phasing)
    pub injection_offset_deg: f32,
    /// Knock retard currently applied (degrees)
    pub knock_retard_deg: f32,
    /// Idle ignition correction (degrees added for stability)
    pub idle_ign_correction_deg: f32,
}

// ─── Knock ───────────────────────────────────────────────────────────────────

/// Knock detection level per cylinder (0–100 normalized)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct KnockState {
    /// Normalized knock intensity 0–100
    pub level: u8,
    /// Current retard being applied (degrees)
    pub retard_deg: f32,
    /// Per-cylinder Fine Knock Correction (FKC) — degrees retard accumulated
    pub fkc: [f32; 8],
    /// Damage Accumulation Multiplier (DAM) — 0.0 to 1.0 (1.0 = no damage)
    pub dam: f32,
    /// Background noise level per cell (H743 spectral fingerprint)
    pub noise_floor: f32,
    /// H743 FFT peak frequency bin of last detected knock
    pub fft_peak_bin: u16,
}

// ─── Idle ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct IdleState {
    pub target_rpm: u16,
    /// Idle valve duty (0.0–1.0)
    pub valve_duty: f32,
    pub pid_integral: f32,
    pub pid_error_prev: f32,
    pub in_idle_control: bool,
    /// DSG/dual-clutch step active (extra RPM added)
    pub dsg_step_active: bool,
}

// ─── Boost ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoostState {
    pub target_kpa: f32,
    pub duty_pct: u8,
    pub pid_integral: f32,
    pub pid_error_prev: f32,
    pub scramble_active: bool,
    /// Active map set (0–2)
    pub active_map_set: u8,
}

// ─── VVT ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VvtState {
    /// Bank 1 intake cam angle (degrees)
    pub b1_intake_deg: f32,
    /// Bank 1 exhaust cam angle
    pub b1_exhaust_deg: f32,
    /// Bank 2 intake cam angle
    pub b2_intake_deg: f32,
    /// Bank 2 exhaust cam angle
    pub b2_exhaust_deg: f32,
    /// PID integrals per channel
    pub pid_integral: [f32; 4],
    pub error_flag: bool,
}

// ─── DBW / E-Throttle ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DbwProfile {
    #[default]
    Sport,
    Eco,
    Rain,
    Drag,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DbwState {
    /// Requested throttle position (0.0–1.0)
    pub throttle_request: f32,
    /// Actual throttle position from TPS1 (0.0–1.0)
    pub throttle_actual: f32,
    /// TPS2 for redundancy
    pub throttle_actual2: f32,
    pub pid_integral: f32,
    pub pid_error_prev: f32,
    pub active_profile: DbwProfile,
    pub limp_mode: bool,
    pub tps_plausibility_error: bool,
}

// ─── Launch Control ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LaunchMode {
    #[default]
    Disabled,
    TwoStep,
    ThreeStep,
    Rolling,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LaunchPreset {
    pub enabled: bool,
    pub mode: LaunchMode,
    pub rpm_limit: u16,
    pub spark_cut_pct: u8,     // 0–100
    pub fuel_cut_pct: u8,      // 0–100
    pub retard_deg: f32,
    pub slip_target_pct: f32,  // for rolling launch
    pub name: [u8; 16],        // ASCII label
}

impl LaunchPreset {
    pub const fn default_preset(idx: u8) -> Self {
        let mut p = Self {
            enabled: false,
            mode: LaunchMode::Disabled,
            rpm_limit: 4500,
            spark_cut_pct: 50,
            fuel_cut_pct: 0,
            retard_deg: 10.0,
            slip_target_pct: 10.0,
            name: [0u8; 16],
        };
        p.rpm_limit = 4000 + (idx as u16) * 500;
        p
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LaunchPhase {
    #[default]
    Idle,
    Armed,
    Active,
    Releasing,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LaunchState {
    pub active_preset: u8, // 0–4
    pub phase: LaunchPhase,
    pub launch_rpm: u16,
    pub spark_cut_active: bool,
    pub fuel_cut_active: bool,
    pub slip_error: f32,
}

// ─── Anti-Lag ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlsMode {
    #[default]
    Disabled,
    IgnitionRetard,
    FuelEnrich,
    Combined,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AntiLagState {
    pub mode: AlsMode,
    /// Current intensity 0.0–1.0 (progressive)
    pub intensity: f32,
    pub active: bool,
    /// Rolling Anti-Lag: active during corners at low throttle
    pub rolling_als_active: bool,
    pub retard_applied_deg: f32,
    pub fuel_enrich_pct: f32,
}

// ─── Traction Control ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TractionPreset {
    pub slip_target_pct: f32,
    pub pid_p: f32,
    pub pid_i: f32,
    pub pid_d: f32,
    pub spark_retard_max_deg: f32,
    pub etb_cut_pct: f32,
    pub min_speed_kmh: f32,
    pub enabled: bool,
    pub name: [u8; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TractionState {
    pub active_preset: u8, // 0–2
    pub active: bool,
    pub slip_actual_pct: f32,
    pub slip_error: f32,
    pub pid_integral: f32,
    pub spark_retard_deg: f32,
    pub etb_cut: f32,
}

// ─── Protections ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtectionAction {
    #[default]
    None,
    IgnRetard,
    SparkCut,
    FuelCut,
    SparkAndFuelCut,
    LimpMode,
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ProtectionState {
    pub rpm_protect: bool,
    pub map_protect: bool,
    pub oil_protect: bool,
    pub afr_protect: bool,
    pub coolant_protect: bool,
    pub egt_protect: bool,
    pub active_action: ProtectionAction,
    pub fuel_enrich_egt_pct: f32, // EGT-based protective enrichment
}

// ─── Master Engine State ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EngineRuntime {
    pub rpm: f32,
    pub rpm_accel: f32, // RPM/s
    pub sync_state: SyncState,
    pub sync_loss_counter: u8,
    pub revolution_counter: u16,

    pub fuel: FuelState,
    pub ignition: IgnitionState,
    pub knock: KnockState,
    pub idle: IdleState,
    pub boost: BoostState,
    pub vvt: VvtState,
    pub dbw: DbwState,
    pub launch: LaunchState,
    pub antilag: AntiLagState,
    pub traction: TractionState,
    pub protection: ProtectionState,

    /// H743-only per-cylinder fuel trim
    pub per_cyl_trim: PerCylinderTrim,

    // Raw sensor values
    pub map_kpa: f32,
    pub baro_kpa: f32,
    pub oil_pressure_kpa: f32,
    pub fuel_pressure_kpa: f32,
    pub coolant_c: f32,
    pub intake_c: f32,
    pub oil_temp_c: f32,
    pub fuel_temp_c: f32,
    pub egt1_c: f32,
    pub egt2_c: f32,
    pub mcu_temp_c: f32,
    pub vbatt: f32,
    pub vss_kmh: f32,
    pub gear: u8,
    pub tps_pct: f32,
    pub pedal_pct: f32,
    pub lambda: f32,
    pub lambda2: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_runtime_default_sync_unsynced() {
        let e = EngineRuntime::default();
        assert_eq!(e.sync_state, SyncState::Unsynced);
        assert_eq!(e.rpm, 0.0);
    }

    #[test]
    fn launch_preset_default_rpm_increases_with_index() {
        let p0 = LaunchPreset::default_preset(0);
        let p1 = LaunchPreset::default_preset(1);
        assert!(p1.rpm_limit > p0.rpm_limit);
    }

    #[test]
    fn knock_state_dam_starts_at_zero() {
        let k = KnockState::default();
        assert_eq!(k.dam, 0.0);
        assert_eq!(k.level, 0);
    }

    #[test]
    fn per_cylinder_trim_all_ones_default() {
        let t = PerCylinderTrim::default();
        // Default trim is 0.0 — firmware initializes to 1.0 at startup
        for &v in t.trim.iter() {
            assert_eq!(v, 0.0);
        }
    }

    #[test]
    fn dbw_profile_default_is_sport() {
        let d = DbwState::default();
        assert_eq!(d.active_profile, DbwProfile::Sport);
    }

    #[test]
    fn traction_preset_default_fields() {
        let t = TractionPreset::default();
        assert!(!t.enabled);
        assert_eq!(t.slip_target_pct, 0.0);
    }
}
