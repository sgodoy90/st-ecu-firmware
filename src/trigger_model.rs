#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStroke {
    TwoStroke,
    FourStroke,
}

impl EngineStroke {
    pub const fn cycle_degrees(self) -> u16 {
        match self {
            Self::TwoStroke => 360,
            Self::FourStroke => 720,
        }
    }

    pub const fn revolutions_per_cycle(self) -> u16 {
        match self {
            Self::TwoStroke => 1,
            Self::FourStroke => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerEdgePolicy {
    RisingOnly,
    RisingAndFalling,
}

impl TriggerEdgePolicy {
    pub const fn edge_multiplier(self) -> u16 {
        match self {
            Self::RisingOnly => 1,
            Self::RisingAndFalling => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerModelError {
    InvalidCylinderCount { cylinders: u8 },
    InvalidToothLayout { total_teeth: u16, missing_teeth: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriggerSyncCorrelation {
    pub strategy: &'static str,
    pub engine_cycle_deg: u16,
    pub primary_events_per_cycle: u16,
    pub primary_edges_per_cycle: u16,
    pub secondary_events_per_cycle: u16,
    pub reference_spacing_deg: f32,
    pub dominant_gap_ratio: Option<f32>,
    pub total_teeth: Option<u16>,
    pub missing_teeth: Option<u16>,
    pub requires_secondary_for_phase: bool,
    pub supports_full_sequential: bool,
}

pub fn distributor_even(
    cylinders: u8,
    stroke: EngineStroke,
    edge_policy: TriggerEdgePolicy,
) -> Result<TriggerSyncCorrelation, TriggerModelError> {
    if cylinders == 0 || cylinders > 16 {
        return Err(TriggerModelError::InvalidCylinderCount { cylinders });
    }

    let cycle_deg = stroke.cycle_degrees();
    let primary_events = cylinders as u16;
    let edge_multiplier = edge_policy.edge_multiplier();
    let primary_edges = primary_events.saturating_mul(edge_multiplier);
    let secondary_events = if stroke == EngineStroke::FourStroke { 1 } else { 0 };
    let spacing = cycle_deg as f32 / primary_events as f32;

    Ok(TriggerSyncCorrelation {
        strategy: "distributor_even",
        engine_cycle_deg: cycle_deg,
        primary_events_per_cycle: primary_events,
        primary_edges_per_cycle: primary_edges,
        secondary_events_per_cycle: secondary_events,
        reference_spacing_deg: spacing,
        dominant_gap_ratio: None,
        total_teeth: None,
        missing_teeth: None,
        requires_secondary_for_phase: stroke == EngineStroke::FourStroke,
        supports_full_sequential: stroke == EngineStroke::FourStroke,
    })
}

pub fn missing_tooth(
    total_teeth: u16,
    missing_teeth: u16,
    stroke: EngineStroke,
    edge_policy: TriggerEdgePolicy,
) -> Result<TriggerSyncCorrelation, TriggerModelError> {
    if total_teeth < 4 || total_teeth > 512 || missing_teeth == 0 || missing_teeth >= total_teeth {
        return Err(TriggerModelError::InvalidToothLayout {
            total_teeth,
            missing_teeth,
        });
    }

    let cycle_revs = stroke.revolutions_per_cycle();
    let visible_teeth_per_rev = total_teeth - missing_teeth;
    let primary_events = visible_teeth_per_rev.saturating_mul(cycle_revs);
    let primary_edges = primary_events.saturating_mul(edge_policy.edge_multiplier());
    let secondary_events = if stroke == EngineStroke::FourStroke { 1 } else { 0 };
    let tooth_pitch_deg = 360.0 / total_teeth as f32;

    Ok(TriggerSyncCorrelation {
        strategy: "missing_tooth",
        engine_cycle_deg: stroke.cycle_degrees(),
        primary_events_per_cycle: primary_events,
        primary_edges_per_cycle: primary_edges,
        secondary_events_per_cycle: secondary_events,
        reference_spacing_deg: tooth_pitch_deg,
        dominant_gap_ratio: Some((missing_teeth + 1) as f32),
        total_teeth: Some(total_teeth),
        missing_teeth: Some(missing_teeth),
        requires_secondary_for_phase: stroke == EngineStroke::FourStroke,
        supports_full_sequential: stroke == EngineStroke::FourStroke,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        distributor_even, missing_tooth, EngineStroke, TriggerEdgePolicy, TriggerModelError,
    };

    fn nearly_equal(left: f32, right: f32) -> bool {
        (left - right).abs() < 0.0001
    }

    #[test]
    fn distributor_even_four_six_eight_cover_common_layouts() {
        let c4 = distributor_even(4, EngineStroke::FourStroke, TriggerEdgePolicy::RisingOnly)
            .expect("4cyl distributor should be valid");
        let c6 = distributor_even(6, EngineStroke::FourStroke, TriggerEdgePolicy::RisingOnly)
            .expect("6cyl distributor should be valid");
        let c8 = distributor_even(8, EngineStroke::FourStroke, TriggerEdgePolicy::RisingOnly)
            .expect("8cyl distributor should be valid");

        assert_eq!(c4.primary_events_per_cycle, 4);
        assert_eq!(c6.primary_events_per_cycle, 6);
        assert_eq!(c8.primary_events_per_cycle, 8);
        assert_eq!(c4.secondary_events_per_cycle, 1);
        assert_eq!(c6.secondary_events_per_cycle, 1);
        assert_eq!(c8.secondary_events_per_cycle, 1);
        assert!(nearly_equal(c4.reference_spacing_deg, 180.0));
        assert!(nearly_equal(c6.reference_spacing_deg, 120.0));
        assert!(nearly_equal(c8.reference_spacing_deg, 90.0));
    }

    #[test]
    fn distributor_two_stroke_does_not_require_phase_input() {
        let corr = distributor_even(2, EngineStroke::TwoStroke, TriggerEdgePolicy::RisingOnly)
            .expect("2-stroke distributor should be valid");
        assert_eq!(corr.engine_cycle_deg, 360);
        assert_eq!(corr.secondary_events_per_cycle, 0);
        assert!(!corr.requires_secondary_for_phase);
        assert!(!corr.supports_full_sequential);
    }

    #[test]
    fn missing_tooth_sixty_two_matches_expected_math() {
        let corr = missing_tooth(
            60,
            2,
            EngineStroke::FourStroke,
            TriggerEdgePolicy::RisingOnly,
        )
        .expect("60-2 should be valid");
        assert_eq!(corr.primary_events_per_cycle, 116);
        assert_eq!(corr.primary_edges_per_cycle, 116);
        assert_eq!(corr.secondary_events_per_cycle, 1);
        assert_eq!(corr.total_teeth, Some(60));
        assert_eq!(corr.missing_teeth, Some(2));
        assert!(nearly_equal(corr.reference_spacing_deg, 6.0));
        assert_eq!(corr.dominant_gap_ratio, Some(3.0));
        assert!(corr.requires_secondary_for_phase);
        assert!(corr.supports_full_sequential);
    }

    #[test]
    fn edge_policy_doubles_count_when_using_both_edges() {
        let rising = missing_tooth(
            36,
            1,
            EngineStroke::FourStroke,
            TriggerEdgePolicy::RisingOnly,
        )
        .expect("36-1 rising should be valid");
        let both = missing_tooth(
            36,
            1,
            EngineStroke::FourStroke,
            TriggerEdgePolicy::RisingAndFalling,
        )
        .expect("36-1 both edges should be valid");
        assert_eq!(both.primary_edges_per_cycle, rising.primary_edges_per_cycle * 2);
    }

    #[test]
    fn rejects_invalid_missing_tooth_layouts() {
        let invalid_total = missing_tooth(
            2,
            1,
            EngineStroke::FourStroke,
            TriggerEdgePolicy::RisingOnly,
        );
        assert_eq!(
            invalid_total,
            Err(TriggerModelError::InvalidToothLayout {
                total_teeth: 2,
                missing_teeth: 1,
            })
        );

        let invalid_missing = missing_tooth(
            36,
            36,
            EngineStroke::FourStroke,
            TriggerEdgePolicy::RisingOnly,
        );
        assert_eq!(
            invalid_missing,
            Err(TriggerModelError::InvalidToothLayout {
                total_teeth: 36,
                missing_teeth: 36,
            })
        );
    }

    #[test]
    fn rejects_invalid_distributor_cylinder_count() {
        let invalid = distributor_even(0, EngineStroke::FourStroke, TriggerEdgePolicy::RisingOnly);
        assert_eq!(
            invalid,
            Err(TriggerModelError::InvalidCylinderCount { cylinders: 0 })
        );
    }
}
