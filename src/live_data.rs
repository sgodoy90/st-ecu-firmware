/// 128-byte live-data wire frame — big-endian, 50 Hz.
/// Must match `parseLiveData()` in `src/lib/protocol.ts` byte-for-byte.
///
/// Layout (verified against protocol.ts offset sequence):
///
///  0  u32  timestamp_ms
///  4  u16  rpm
///  6  i16  rpm_accel
///  8  u8   sync_loss_counter
///  9  u8   [pad]
/// 10  u16  map_kpa        (×10)
/// 12  u16  baro_kpa       (×10)
/// 14  u16  oil_pressure_kpa (×10)
/// 16  u16  fuel_pressure_kpa (×10)
/// 18  i16  boost_kpa      (×10)
/// 20  i16  coolant_c      (×10)
/// 22  i16  intake_c       (×10)
/// 24  i16  oil_temp_c     (×10)
/// 26  i16  fuel_temp_c    (×10)
/// 28  i16  aux_temp1_c    (×10)  EGT1
/// 30  i16  aux_temp2_c    (×10)  EGT2
/// 32  i8   mcu_temp_c
/// 33  u8   [pad]
/// 34  i16  tps_pct        (×100)
/// 36  i16  pedal_pct      (×100)
/// 38  i16  fuel_load      (×100)
/// 40  i16  ign_load       (×100)
/// 42  u16  lambda         (×10000)
/// 44  u16  lambda2        (×10000)
/// 46  u16  afr_target     (×100)
/// 48  u8   injector_duty_pct (×2)
/// 49  u8   ve_pct
/// 50  i16  fuel_correction_pct (×100)
/// 52  u8   accel_enrich_pct
/// 53  u8   [pad]
/// 54  u16  actual_pulsewidth_ms (×1000)
/// 56  u16  wall_fuel_mg   (×100)
/// 58  i16  advance_deg    (×10)
/// 60  u16  dwell_ms       (×1000)
/// 62  i16  injection_offset_deg
/// 64  u16  vbatt          (×100)
/// 66  i16  vref_mv        (×1000)
/// 68  u16  vss_kmh
/// 70  u8   gear
/// 71  u8   [pad]
/// 72  i16  vvt_b1_intake_deg  (×10)
/// 74  i16  vvt_b1_exhaust_deg (×10)
/// 76  i16  vvt_b2_intake_deg  (×10)
/// 78  i16  vvt_b2_exhaust_deg (×10)
/// 80  u8   knock_level
/// 81  i8   knock_retard_deg
/// 82  u16  boost_target_kpa   (×10)
/// 84  u8   boost_duty_pct
/// 85  u8   [pad] — protocol.ts reads idle_target_rpm at 85? No: boost_duty is u8 at 84
///     Actually protocol.ts: boost_target_kpa=r16u (82-83), boost_duty_pct=r8u(84)
///     idle_target_rpm=r16u (85-86), idle_valve_pct=r8u(87)
/// 85  u8   [pad — align for idle_target_rpm u16]  NO — protocol.ts doesn't pad here
///     Let's trace exactly: after knock (80,81=2), boost_target(82-83=2), boost_duty(84=1) → 85
///     idle_target_rpm r16u = bytes 85-86
///     idle_valve_pct r8u = byte 87
/// 88  i16  correction_iat (×100)
/// 90  i16  correction_clt (×100)
/// 92  i16  correction_baro (×100)
/// 94  i16  correction_flex (×100)
/// 96  u32  status_flags
/// 100 u8   protect_flags
/// 101 u16  error_flags
/// 103 u8   [pad]
/// 104 u16  revolution_counter
/// 106 u16  loops_per_sec
/// 108 u8   free_heap_pct
/// 109 u8   rotational_idle_cut_pct
/// 110 u16  rotational_idle_timer_cs
/// 112 u8   rotational_idle_active_cylinders
/// 113 u8   rotational_idle_gate_code
/// 114 u8   rotational_idle_sync_guard_events
/// 115 u8   transmission_status_flags
/// 116 u8   transmission_requested_gear
/// 117 u8   transmission_torque_reduction_pct
/// 118 u16  transmission_torque_reduction_timer_cs
/// 120 u8   transmission_shift_result_code
/// 121 u8   transmission_shift_request_counter
/// 122 u8   transmission_shift_timeout_counter
/// 123 u8   transmission_shift_fault_code
/// 124 u8   transmission_state_code
/// 125 u16  transmission_request_age_cs
/// 127 u8   transmission_ack_counter
/// Total: 128

pub const LIVE_DATA_SIZE: usize = 128;

/// All engine channels packed for the 128-byte wire frame.
/// Floating-point on firmware side; serialized as scaled integers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LiveDataFrame {
    // Time
    pub timestamp_ms: u32,

    // Engine speed / sync
    pub rpm: f32,
    pub rpm_accel: f32,     // RPM/s
    pub sync_loss_counter: u8,

    // Pressures (kPa)
    pub map_kpa: f32,
    pub baro_kpa: f32,
    pub oil_pressure_kpa: f32,
    pub fuel_pressure_kpa: f32,
    pub boost_kpa: f32,     // can be negative

    // Temperatures (°C)
    pub coolant_c: f32,
    pub intake_c: f32,
    pub oil_temp_c: f32,
    pub fuel_temp_c: f32,
    pub aux_temp1_c: f32,   // EGT1
    pub aux_temp2_c: f32,   // EGT2
    pub mcu_temp_c: f32,

    // Throttle / load (%)
    pub tps_pct: f32,
    pub pedal_pct: f32,
    pub fuel_load: f32,
    pub ign_load: f32,

    // Fuel
    pub lambda: f32,        // e.g. 1.0000
    pub lambda2: f32,
    pub afr_target: f32,    // e.g. 14.70
    pub injector_duty_pct: f32,
    pub ve_pct: f32,
    pub fuel_correction_pct: f32,
    pub accel_enrich_pct: f32,
    pub actual_pulsewidth_ms: f32,
    pub wall_fuel_mg: f32,

    // Ignition
    pub advance_deg: f32,
    pub dwell_ms: f32,
    pub injection_offset_deg: f32,

    // Electrical
    pub vbatt: f32,     // V
    pub vref_mv: f32,   // V

    // Motion
    pub vss_kmh: f32,
    pub gear: u8,

    // VVT (°)
    pub vvt_b1_intake_deg: f32,
    pub vvt_b1_exhaust_deg: f32,
    pub vvt_b2_intake_deg: f32,
    pub vvt_b2_exhaust_deg: f32,

    // Knock
    pub knock_level: u8,    // 0..100
    pub knock_retard_deg: f32,

    // Boost
    pub boost_target_kpa: f32,
    pub boost_duty_pct: u8,

    // Idle
    pub idle_target_rpm: u16,
    pub idle_valve_pct: u8,

    // Active corrections (%)
    pub correction_iat: f32,
    pub correction_clt: f32,
    pub correction_baro: f32,
    pub correction_flex: f32,

    // Status / protect / error flags (bitfields)
    pub status_flags: u32,
    pub protect_flags: u8,
    pub error_flags: u16,

    // Health counters
    pub revolution_counter: u16,
    pub loops_per_sec: u16,
    pub free_heap_pct: u8,

    // Rotational idle runtime telemetry
    pub rotational_idle_cut_pct: u8,
    pub rotational_idle_timer_cs: u16,
    pub rotational_idle_active_cylinders: u8,
    pub rotational_idle_gate_code: u8,
    pub rotational_idle_sync_guard_events: u8,

    // Transmission / external TCU runtime telemetry
    pub transmission_status_flags: u8,
    pub transmission_requested_gear: u8,
    pub transmission_torque_reduction_pct: u8,
    pub transmission_torque_reduction_timer_cs: u16,
    pub transmission_shift_result_code: u8,
    pub transmission_shift_request_counter: u8,
    pub transmission_shift_timeout_counter: u8,
    pub transmission_shift_fault_code: u8,
    pub transmission_state_code: u8,
    pub transmission_request_age_cs: u16,
    pub transmission_ack_counter: u8,
}

impl LiveDataFrame {
    /// Serialize into the 128-byte big-endian wire format.
    /// Every offset here must match protocol.ts `parseLiveData()`.
    pub fn encode(&self) -> [u8; LIVE_DATA_SIZE] {
        let mut b = [0u8; LIVE_DATA_SIZE];
        let mut o: usize = 0;

        macro_rules! w32u {
            ($v:expr) => { b[o..o+4].copy_from_slice(&($v as u32).to_be_bytes()); o += 4; };
        }
        macro_rules! w16u {
            ($v:expr) => { b[o..o+2].copy_from_slice(&($v as u16).to_be_bytes()); o += 2; };
        }
        macro_rules! w16i {
            ($v:expr) => { b[o..o+2].copy_from_slice(&($v as i16).to_be_bytes()); o += 2; };
        }
        macro_rules! w8u {
            ($v:expr) => { b[o] = $v as u8; o += 1; };
        }
        macro_rules! w8i {
            ($v:expr) => { b[o] = ($v as i8) as u8; o += 1; };
        }
        macro_rules! pad {
            () => { b[o] = 0; o += 1; };
        }

        // 0: timestamp_ms  u32
        w32u!(self.timestamp_ms);
        // 4: rpm  u16
        w16u!(self.rpm.round().max(0.0).min(65535.0));
        // 6: rpm_accel  i16
        w16i!(self.rpm_accel.round().max(-32768.0).min(32767.0));
        // 8: sync_loss_counter  u8
        w8u!(self.sync_loss_counter);
        // 9: pad
        pad!();
        // 10: map_kpa ×10  u16
        w16u!((self.map_kpa * 10.0).round().max(0.0).min(65535.0));
        // 12: baro_kpa ×10  u16
        w16u!((self.baro_kpa * 10.0).round().max(0.0).min(65535.0));
        // 14: oil_pressure_kpa ×10  u16
        w16u!((self.oil_pressure_kpa * 10.0).round().max(0.0).min(65535.0));
        // 16: fuel_pressure_kpa ×10  u16
        w16u!((self.fuel_pressure_kpa * 10.0).round().max(0.0).min(65535.0));
        // 18: boost_kpa ×10  i16
        w16i!((self.boost_kpa * 10.0).round().max(-32768.0).min(32767.0));
        // 20: coolant_c ×10  i16
        w16i!((self.coolant_c * 10.0).round().max(-32768.0).min(32767.0));
        // 22: intake_c ×10  i16
        w16i!((self.intake_c * 10.0).round().max(-32768.0).min(32767.0));
        // 24: oil_temp_c ×10  i16
        w16i!((self.oil_temp_c * 10.0).round().max(-32768.0).min(32767.0));
        // 26: fuel_temp_c ×10  i16
        w16i!((self.fuel_temp_c * 10.0).round().max(-32768.0).min(32767.0));
        // 28: aux_temp1_c ×10  i16
        w16i!((self.aux_temp1_c * 10.0).round().max(-32768.0).min(32767.0));
        // 30: aux_temp2_c ×10  i16
        w16i!((self.aux_temp2_c * 10.0).round().max(-32768.0).min(32767.0));
        // 32: mcu_temp_c  i8
        w8i!(self.mcu_temp_c.round().max(-128.0).min(127.0));
        // 33: pad
        pad!();
        // 34: tps_pct ×100  i16
        w16i!((self.tps_pct * 100.0).round().max(-32768.0).min(32767.0));
        // 36: pedal_pct ×100  i16
        w16i!((self.pedal_pct * 100.0).round().max(-32768.0).min(32767.0));
        // 38: fuel_load ×100  i16
        w16i!((self.fuel_load * 100.0).round().max(-32768.0).min(32767.0));
        // 40: ign_load ×100  i16
        w16i!((self.ign_load * 100.0).round().max(-32768.0).min(32767.0));
        // 42: lambda ×10000  u16
        w16u!((self.lambda * 10000.0).round().max(0.0).min(65535.0));
        // 44: lambda2 ×10000  u16
        w16u!((self.lambda2 * 10000.0).round().max(0.0).min(65535.0));
        // 46: afr_target ×100  u16
        w16u!((self.afr_target * 100.0).round().max(0.0).min(65535.0));
        // 48: injector_duty_pct ×2  u8
        w8u!((self.injector_duty_pct * 2.0).round().min(255.0));
        // 49: ve_pct  u8
        w8u!(self.ve_pct.round().max(0.0).min(255.0));
        // 50: fuel_correction_pct ×100  i16
        w16i!((self.fuel_correction_pct * 100.0).round().max(-32768.0).min(32767.0));
        // 52: accel_enrich_pct  u8
        w8u!(self.accel_enrich_pct.round().max(0.0).min(255.0));
        // 53: pad
        pad!();
        // 54: actual_pulsewidth_ms ×1000  u16
        w16u!((self.actual_pulsewidth_ms * 1000.0).round().max(0.0).min(65535.0));
        // 56: wall_fuel_mg ×100  u16
        w16u!((self.wall_fuel_mg * 100.0).round().max(0.0).min(65535.0));
        // 58: advance_deg ×10  i16
        w16i!((self.advance_deg * 10.0).round().max(-32768.0).min(32767.0));
        // 60: dwell_ms ×1000  u16
        w16u!((self.dwell_ms * 1000.0).round().max(0.0).min(65535.0));
        // 62: injection_offset_deg  i16
        w16i!(self.injection_offset_deg.round().max(-32768.0).min(32767.0));
        // 64: vbatt ×100  u16
        w16u!((self.vbatt * 100.0).round().max(0.0).min(65535.0));
        // 66: vref_mv ×1000  i16
        w16i!((self.vref_mv * 1000.0).round().max(-32768.0).min(32767.0));
        // 68: vss_kmh  u16
        w16u!(self.vss_kmh.round().max(0.0).min(65535.0));
        // 70: gear  u8
        w8u!(self.gear);
        // 71: pad
        pad!();
        // 72: vvt_b1_intake_deg ×10  i16
        w16i!((self.vvt_b1_intake_deg * 10.0).round().max(-32768.0).min(32767.0));
        // 74: vvt_b1_exhaust_deg ×10  i16
        w16i!((self.vvt_b1_exhaust_deg * 10.0).round().max(-32768.0).min(32767.0));
        // 76: vvt_b2_intake_deg ×10  i16
        w16i!((self.vvt_b2_intake_deg * 10.0).round().max(-32768.0).min(32767.0));
        // 78: vvt_b2_exhaust_deg ×10  i16
        w16i!((self.vvt_b2_exhaust_deg * 10.0).round().max(-32768.0).min(32767.0));
        // 80: knock_level  u8
        w8u!(self.knock_level);
        // 81: knock_retard_deg  i8
        w8i!(self.knock_retard_deg.round().max(-128.0).min(127.0));
        // 82: boost_target_kpa ×10  u16
        w16u!((self.boost_target_kpa * 10.0).round().max(0.0).min(65535.0));
        // 84: boost_duty_pct  u8
        w8u!(self.boost_duty_pct);
        // 85: idle_target_rpm  u16
        w16u!(self.idle_target_rpm);
        // 87: idle_valve_pct  u8
        w8u!(self.idle_valve_pct);
        // 88: correction_iat ×100  i16
        w16i!((self.correction_iat * 100.0).round().max(-32768.0).min(32767.0));
        // 90: correction_clt ×100  i16
        w16i!((self.correction_clt * 100.0).round().max(-32768.0).min(32767.0));
        // 92: correction_baro ×100  i16
        w16i!((self.correction_baro * 100.0).round().max(-32768.0).min(32767.0));
        // 94: correction_flex ×100  i16
        w16i!((self.correction_flex * 100.0).round().max(-32768.0).min(32767.0));
        // 96: status_flags  u32
        w32u!(self.status_flags);
        // 100: protect_flags  u8
        w8u!(self.protect_flags);
        // 101: error_flags  u16
        w16u!(self.error_flags);
        // 103: pad
        pad!();
        // 104: revolution_counter  u16
        w16u!(self.revolution_counter);
        // 106: loops_per_sec  u16
        w16u!(self.loops_per_sec);
        // 108: free_heap_pct  u8
        w8u!(self.free_heap_pct);
        // 109: rotational_idle_cut_pct  u8
        w8u!(self.rotational_idle_cut_pct);
        // 110: rotational_idle_timer_cs  u16
        w16u!(self.rotational_idle_timer_cs);
        // 112: rotational_idle_active_cylinders  u8
        w8u!(self.rotational_idle_active_cylinders);
        // 113: rotational_idle_gate_code  u8
        w8u!(self.rotational_idle_gate_code);
        // 114: rotational_idle_sync_guard_events  u8
        w8u!(self.rotational_idle_sync_guard_events);
        // 115: transmission_status_flags  u8
        w8u!(self.transmission_status_flags);
        // 116: transmission_requested_gear  u8
        w8u!(self.transmission_requested_gear);
        // 117: transmission_torque_reduction_pct  u8
        w8u!(self.transmission_torque_reduction_pct);
        // 118: transmission_torque_reduction_timer_cs  u16
        w16u!(self.transmission_torque_reduction_timer_cs);
        // 120: transmission_shift_result_code  u8
        w8u!(self.transmission_shift_result_code);
        // 121: transmission_shift_request_counter  u8
        w8u!(self.transmission_shift_request_counter);
        // 122: transmission_shift_timeout_counter  u8
        w8u!(self.transmission_shift_timeout_counter);
        // 123: transmission_shift_fault_code  u8
        w8u!(self.transmission_shift_fault_code);
        // 124: transmission_state_code  u8
        w8u!(self.transmission_state_code);
        // 125: transmission_request_age_cs  u16
        w16u!(self.transmission_request_age_cs);
        // 127: transmission_ack_counter  u8
        w8u!(self.transmission_ack_counter);

        debug_assert_eq!(o, 128, "live_data encode offset drift: expected 128, got {o}");
        b
    }

    /// Decode a 128-byte buffer back into LiveDataFrame (for testing).
    pub fn decode(b: &[u8; LIVE_DATA_SIZE]) -> Self {
        let mut o: usize = 0;
        macro_rules! r32u { () => {{ let v = u32::from_be_bytes([b[o],b[o+1],b[o+2],b[o+3]]); o+=4; v }} }
        macro_rules! r16u { () => {{ let v = u16::from_be_bytes([b[o],b[o+1]]); o+=2; v }} }
        macro_rules! r16i { () => {{ let v = i16::from_be_bytes([b[o],b[o+1]]); o+=2; v }} }
        macro_rules! r8u  { () => {{ let v = b[o]; o+=1; v }} }
        macro_rules! r8i  { () => {{ let v = b[o] as i8; o+=1; v }} }
        macro_rules! skip { ($n:expr) => { o += $n; } }

        let timestamp_ms        = r32u!();
        let rpm                 = r16u!() as f32;
        let rpm_accel           = r16i!() as f32;
        let sync_loss_counter   = r8u!();
        skip!(1);
        let map_kpa             = r16u!() as f32 / 10.0;
        let baro_kpa            = r16u!() as f32 / 10.0;
        let oil_pressure_kpa    = r16u!() as f32 / 10.0;
        let fuel_pressure_kpa   = r16u!() as f32 / 10.0;
        let boost_kpa           = r16i!() as f32 / 10.0;
        let coolant_c           = r16i!() as f32 / 10.0;
        let intake_c            = r16i!() as f32 / 10.0;
        let oil_temp_c          = r16i!() as f32 / 10.0;
        let fuel_temp_c         = r16i!() as f32 / 10.0;
        let aux_temp1_c         = r16i!() as f32 / 10.0;
        let aux_temp2_c         = r16i!() as f32 / 10.0;
        let mcu_temp_c          = r8i!() as f32;
        skip!(1);
        let tps_pct             = r16i!() as f32 / 100.0;
        let pedal_pct           = r16i!() as f32 / 100.0;
        let fuel_load           = r16i!() as f32 / 100.0;
        let ign_load            = r16i!() as f32 / 100.0;
        let lambda              = r16u!() as f32 / 10000.0;
        let lambda2             = r16u!() as f32 / 10000.0;
        let afr_target          = r16u!() as f32 / 100.0;
        let injector_duty_pct   = r8u!() as f32 / 2.0;
        let ve_pct              = r8u!() as f32;
        let fuel_correction_pct = r16i!() as f32 / 100.0;
        let accel_enrich_pct    = r8u!() as f32;
        skip!(1);
        let actual_pulsewidth_ms = r16u!() as f32 / 1000.0;
        let wall_fuel_mg        = r16u!() as f32 / 100.0;
        let advance_deg         = r16i!() as f32 / 10.0;
        let dwell_ms            = r16u!() as f32 / 1000.0;
        let injection_offset_deg = r16i!() as f32;
        let vbatt               = r16u!() as f32 / 100.0;
        let vref_mv             = r16i!() as f32 / 1000.0;
        let vss_kmh             = r16u!() as f32;
        let gear                = r8u!();
        skip!(1);
        let vvt_b1_intake_deg   = r16i!() as f32 / 10.0;
        let vvt_b1_exhaust_deg  = r16i!() as f32 / 10.0;
        let vvt_b2_intake_deg   = r16i!() as f32 / 10.0;
        let vvt_b2_exhaust_deg  = r16i!() as f32 / 10.0;
        let knock_level         = r8u!();
        let knock_retard_deg    = r8i!() as f32;
        let boost_target_kpa    = r16u!() as f32 / 10.0;
        let boost_duty_pct      = r8u!();
        let idle_target_rpm     = r16u!();
        let idle_valve_pct      = r8u!();
        let correction_iat      = r16i!() as f32 / 100.0;
        let correction_clt      = r16i!() as f32 / 100.0;
        let correction_baro     = r16i!() as f32 / 100.0;
        let correction_flex     = r16i!() as f32 / 100.0;
        let status_flags        = r32u!();
        let protect_flags       = r8u!();
        let error_flags         = r16u!();
        skip!(1);
        let revolution_counter  = r16u!();
        let loops_per_sec       = r16u!();
        let free_heap_pct       = r8u!();
        let rotational_idle_cut_pct = r8u!();
        let rotational_idle_timer_cs = r16u!();
        let rotational_idle_active_cylinders = r8u!();
        let rotational_idle_gate_code = r8u!();
        let rotational_idle_sync_guard_events = r8u!();
        let transmission_status_flags = r8u!();
        let transmission_requested_gear = r8u!();
        let transmission_torque_reduction_pct = r8u!();
        let transmission_torque_reduction_timer_cs = r16u!();
        let transmission_shift_result_code = r8u!();
        let transmission_shift_request_counter = r8u!();
        let transmission_shift_timeout_counter = r8u!();
        let transmission_shift_fault_code = r8u!();
        let transmission_state_code = r8u!();
        let transmission_request_age_cs = r16u!();
        let transmission_ack_counter = r8u!();
        debug_assert_eq!(o, 128, "live_data decode offset drift: expected 128, got {o}");

        Self {
            timestamp_ms,
            rpm, rpm_accel, sync_loss_counter,
            map_kpa, baro_kpa, oil_pressure_kpa, fuel_pressure_kpa, boost_kpa,
            coolant_c, intake_c, oil_temp_c, fuel_temp_c, aux_temp1_c, aux_temp2_c, mcu_temp_c,
            tps_pct, pedal_pct, fuel_load, ign_load,
            lambda, lambda2, afr_target, injector_duty_pct, ve_pct,
            fuel_correction_pct, accel_enrich_pct, actual_pulsewidth_ms, wall_fuel_mg,
            advance_deg, dwell_ms, injection_offset_deg,
            vbatt, vref_mv,
            vss_kmh, gear,
            vvt_b1_intake_deg, vvt_b1_exhaust_deg, vvt_b2_intake_deg, vvt_b2_exhaust_deg,
            knock_level, knock_retard_deg,
            boost_target_kpa, boost_duty_pct,
            idle_target_rpm, idle_valve_pct,
            correction_iat, correction_clt, correction_baro, correction_flex,
            status_flags, protect_flags, error_flags,
            revolution_counter, loops_per_sec, free_heap_pct,
            rotational_idle_cut_pct,
            rotational_idle_timer_cs,
            rotational_idle_active_cylinders,
            rotational_idle_gate_code,
            rotational_idle_sync_guard_events,
            transmission_status_flags,
            transmission_requested_gear,
            transmission_torque_reduction_pct,
            transmission_torque_reduction_timer_cs,
            transmission_shift_result_code,
            transmission_shift_request_counter,
            transmission_shift_timeout_counter,
            transmission_shift_fault_code,
            transmission_state_code,
            transmission_request_age_cs,
            transmission_ack_counter,
        }
    }
}

// ─── Status flag bit positions (match StatusFlags in protocol.ts) ─────────────
pub mod status {
    pub const RUNNING:         u32 = 1 << 0;
    pub const CRANKING:        u32 = 1 << 1;
    pub const WARMUP:          u32 = 1 << 2;
    pub const ASE:             u32 = 1 << 3;
    pub const DFCO:            u32 = 1 << 4;
    pub const CLOSED_LOOP:     u32 = 1 << 5;
    pub const ACCEL_ENRICH:    u32 = 1 << 6;
    pub const LAUNCH_ACTIVE:   u32 = 1 << 7;
    pub const LAUNCH_HARD_CUT: u32 = 1 << 8;
    pub const FLAT_SHIFT:      u32 = 1 << 9;
    pub const BOOST_CUT_FUEL:  u32 = 1 << 10;
    pub const BOOST_CUT_SPARK: u32 = 1 << 11;
    pub const NITROUS_ACTIVE:  u32 = 1 << 12;
    pub const TRACTION_ACTIVE: u32 = 1 << 13;
    pub const VSS_VALID:       u32 = 1 << 14;
    pub const FLEX_VALID:      u32 = 1 << 15;
    pub const SD_PRESENT:      u32 = 1 << 16;
    pub const SD_LOGGING:      u32 = 1 << 17;
    pub const WIFI_CONNECTED:  u32 = 1 << 18;
    pub const CAN_ACTIVE:      u32 = 1 << 19;
    pub const USB_CONNECTED:   u32 = 1 << 20;
    pub const CHECK_ENGINE:    u32 = 1 << 21;
    pub const NEED_BURN:       u32 = 1 << 22;
    pub const OVERREV:         u32 = 1 << 23;
    pub const ROTATIONAL_IDLE_ACTIVE: u32 = 1 << 24;
    pub const ROTATIONAL_IDLE_ARMED: u32 = 1 << 25;
    pub const WIDEBAND_HEATER_READY: u32 = 1 << 26;
    pub const WIDEBAND_INTEGRATED_ACTIVE: u32 = 1 << 27;
    pub const WIDEBAND_ANALOG_FALLBACK: u32 = 1 << 28;
}

pub mod transmission_status {
    pub const TCU_LINK_ONLINE: u8 = 1 << 0;
    pub const SHIFT_IN_PROGRESS: u8 = 1 << 1;
    pub const TORQUE_INTERVENTION_ACTIVE: u8 = 1 << 2;
    pub const TORQUE_INTERVENTION_REQUESTED: u8 = 1 << 3;
}

// ─── Protect flag bits ────────────────────────────────────────────────────────
pub mod protect {
    pub const RPM:     u8 = 1 << 0;
    pub const MAP:     u8 = 1 << 1;
    pub const OIL:     u8 = 1 << 2;
    pub const AFR:     u8 = 1 << 3;
    pub const COOLANT: u8 = 1 << 4;
}

// ─── Error flag bits ─────────────────────────────────────────────────────────
pub mod error {
    pub const TPS:            u16 = 1 << 0;
    pub const TPS2:           u16 = 1 << 1;
    pub const CLT:            u16 = 1 << 2;
    pub const IAT:            u16 = 1 << 3;
    pub const MAP:            u16 = 1 << 4;
    pub const O2_PRIMARY:     u16 = 1 << 5;
    pub const O2_SECONDARY:   u16 = 1 << 6;
    pub const TRIGGER:        u16 = 1 << 7;
    pub const PEDAL:          u16 = 1 << 8;
    pub const INJECTOR:       u16 = 1 << 9;
    pub const IGNITION:       u16 = 1 << 10;
    pub const ANALOG_SUPPLY:  u16 = 1 << 11;
    pub const KNOCK:          u16 = 1 << 12;
    pub const VVT:            u16 = 1 << 13;
    pub const BOOST_VALVE:    u16 = 1 << 14;
    pub const CRITICAL:       u16 = 1 << 15;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_produces_exactly_128_bytes() {
        let frame = LiveDataFrame::default();
        let encoded = frame.encode();
        assert_eq!(encoded.len(), LIVE_DATA_SIZE);
    }

    #[test]
    fn encode_decode_roundtrip_rpm() {
        let mut frame = LiveDataFrame::default();
        frame.rpm = 4500.0;
        frame.coolant_c = 85.5;
        frame.advance_deg = 32.5;
        frame.lambda = 1.0;
        frame.afr_target = 14.70;
        let encoded = frame.encode();
        let decoded = LiveDataFrame::decode(&encoded);
        assert_eq!(decoded.rpm, 4500.0);
        assert!((decoded.coolant_c - 85.5).abs() < 0.1);
        assert!((decoded.advance_deg - 32.5).abs() < 0.1);
        assert!((decoded.lambda - 1.0).abs() < 0.0001);
        assert!((decoded.afr_target - 14.70).abs() < 0.01);
    }

    #[test]
    fn status_flags_roundtrip() {
        let mut frame = LiveDataFrame::default();
        frame.status_flags = status::RUNNING
            | status::CLOSED_LOOP
            | status::CAN_ACTIVE
            | status::ROTATIONAL_IDLE_ACTIVE
            | status::ROTATIONAL_IDLE_ARMED;
        let enc = frame.encode();
        let dec = LiveDataFrame::decode(&enc);
        assert_eq!(
            dec.status_flags,
            status::RUNNING
                | status::CLOSED_LOOP
                | status::CAN_ACTIVE
                | status::ROTATIONAL_IDLE_ACTIVE
                | status::ROTATIONAL_IDLE_ARMED
        );
    }

    #[test]
    fn boost_negative_kpa_roundtrip() {
        let mut frame = LiveDataFrame::default();
        frame.boost_kpa = -15.3; // vacuum
        let enc = frame.encode();
        let dec = LiveDataFrame::decode(&enc);
        assert!((dec.boost_kpa - (-15.3)).abs() < 0.1);
    }

    #[test]
    fn vbatt_precision() {
        let mut frame = LiveDataFrame::default();
        frame.vbatt = 13.85;
        let enc = frame.encode();
        let dec = LiveDataFrame::decode(&enc);
        assert!((dec.vbatt - 13.85).abs() < 0.01);
    }

    #[test]
    fn rotational_idle_runtime_roundtrip() {
        let mut frame = LiveDataFrame::default();
        frame.rotational_idle_cut_pct = 42;
        frame.rotational_idle_timer_cs = 315;
        frame.rotational_idle_active_cylinders = 3;
        frame.rotational_idle_gate_code = 8;
        frame.rotational_idle_sync_guard_events = 2;
        let enc = frame.encode();
        let dec = LiveDataFrame::decode(&enc);
        assert_eq!(dec.rotational_idle_cut_pct, 42);
        assert_eq!(dec.rotational_idle_timer_cs, 315);
        assert_eq!(dec.rotational_idle_active_cylinders, 3);
        assert_eq!(dec.rotational_idle_gate_code, 8);
        assert_eq!(dec.rotational_idle_sync_guard_events, 2);
    }

    #[test]
    fn transmission_runtime_roundtrip() {
        let mut frame = LiveDataFrame::default();
        frame.transmission_status_flags = transmission_status::TCU_LINK_ONLINE
            | transmission_status::SHIFT_IN_PROGRESS
            | transmission_status::TORQUE_INTERVENTION_ACTIVE;
        frame.transmission_requested_gear = 4;
        frame.transmission_torque_reduction_pct = 28;
        frame.transmission_torque_reduction_timer_cs = 145;
        frame.transmission_shift_result_code = 1;
        frame.transmission_shift_request_counter = 17;
        frame.transmission_shift_timeout_counter = 2;
        frame.transmission_shift_fault_code = 3;
        frame.transmission_state_code = 4;
        frame.transmission_request_age_cs = 188;
        frame.transmission_ack_counter = 15;
        let enc = frame.encode();
        let dec = LiveDataFrame::decode(&enc);
        assert_eq!(dec.transmission_status_flags, frame.transmission_status_flags);
        assert_eq!(dec.transmission_requested_gear, 4);
        assert_eq!(dec.transmission_torque_reduction_pct, 28);
        assert_eq!(dec.transmission_torque_reduction_timer_cs, 145);
        assert_eq!(dec.transmission_shift_result_code, 1);
        assert_eq!(dec.transmission_shift_request_counter, 17);
        assert_eq!(dec.transmission_shift_timeout_counter, 2);
        assert_eq!(dec.transmission_shift_fault_code, 3);
        assert_eq!(dec.transmission_state_code, 4);
        assert_eq!(dec.transmission_request_age_cs, 188);
        assert_eq!(dec.transmission_ack_counter, 15);
    }

    #[test]
    fn transmission_extension_defaults_zero() {
        let frame = LiveDataFrame::default();
        let enc = frame.encode();
        for i in 121..128 {
            assert_eq!(enc[i], 0, "transmission extension byte {i} is not zero");
        }
    }
}
