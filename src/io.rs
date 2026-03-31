use crate::board::{
    find_pin, validate_pin_assignment, BoardValidationError, PinCapability, PinFunctionClass,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EcuFunction {
    CrankInput,
    CamInput,
    MapSensor,
    TpsSensor,
    CltSensor,
    IatSensor,
    BoostControl,
    IdleControl,
    Injector1,
    Injector2,
    Ignition1,
    Ignition2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EcuFunctionParseError {
    pub code: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingPolicy {
    FixedHardwarePath,
    FlexibleCompatible,
}

impl EcuFunction {
    pub const fn code(self) -> u8 {
        match self {
            Self::CrankInput => 0x01,
            Self::CamInput => 0x02,
            Self::MapSensor => 0x03,
            Self::TpsSensor => 0x04,
            Self::CltSensor => 0x05,
            Self::IatSensor => 0x06,
            Self::BoostControl => 0x10,
            Self::IdleControl => 0x11,
            Self::Injector1 => 0x20,
            Self::Injector2 => 0x21,
            Self::Ignition1 => 0x30,
            Self::Ignition2 => 0x31,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::CrankInput => "crank_input",
            Self::CamInput => "cam_input",
            Self::MapSensor => "map_sensor",
            Self::TpsSensor => "tps_sensor",
            Self::CltSensor => "clt_sensor",
            Self::IatSensor => "iat_sensor",
            Self::BoostControl => "boost_control",
            Self::IdleControl => "idle_control",
            Self::Injector1 => "injector_1",
            Self::Injector2 => "injector_2",
            Self::Ignition1 => "ignition_1",
            Self::Ignition2 => "ignition_2",
        }
    }

    pub const fn required_pin_function(self) -> PinFunctionClass {
        match self {
            Self::CrankInput | Self::CamInput => PinFunctionClass::CaptureInput,
            Self::MapSensor | Self::TpsSensor | Self::CltSensor | Self::IatSensor => {
                PinFunctionClass::AnalogInput
            }
            Self::BoostControl | Self::IdleControl => PinFunctionClass::PwmOutput,
            Self::Injector1 | Self::Injector2 => PinFunctionClass::Injector,
            Self::Ignition1 | Self::Ignition2 => PinFunctionClass::Ignition,
        }
    }

    pub const fn routing_policy(self) -> RoutingPolicy {
        match self {
            Self::CrankInput
            | Self::CamInput
            | Self::Injector1
            | Self::Injector2
            | Self::Ignition1
            | Self::Ignition2 => RoutingPolicy::FixedHardwarePath,
            Self::MapSensor
            | Self::TpsSensor
            | Self::CltSensor
            | Self::IatSensor
            | Self::BoostControl
            | Self::IdleControl => RoutingPolicy::FlexibleCompatible,
        }
    }

    pub const fn fixed_pin_id(self) -> Option<&'static str> {
        match self {
            Self::CrankInput => Some("PA0"),
            Self::CamInput => Some("PA1"),
            Self::Injector1 => Some("PE8"),
            Self::Injector2 => Some("PE9"),
            Self::Ignition1 => Some("PF8"),
            Self::Ignition2 => Some("PF9"),
            _ => None,
        }
    }
}

impl TryFrom<u8> for EcuFunction {
    type Error = EcuFunctionParseError;

    fn try_from(value: u8) -> Result<Self, EcuFunctionParseError> {
        match value {
            0x01 => Ok(Self::CrankInput),
            0x02 => Ok(Self::CamInput),
            0x03 => Ok(Self::MapSensor),
            0x04 => Ok(Self::TpsSensor),
            0x05 => Ok(Self::CltSensor),
            0x06 => Ok(Self::IatSensor),
            0x10 => Ok(Self::BoostControl),
            0x11 => Ok(Self::IdleControl),
            0x20 => Ok(Self::Injector1),
            0x21 => Ok(Self::Injector2),
            0x30 => Ok(Self::Ignition1),
            0x31 => Ok(Self::Ignition2),
            _ => Err(EcuFunctionParseError { code: value }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinAssignmentRequest<'a> {
    pub function: EcuFunction,
    pub pin_id: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPinAssignment {
    pub function: EcuFunction,
    pub pin_id: &'static str,
    pub pin_label: &'static str,
    pub required_function: PinFunctionClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentError {
    Board(BoardValidationError),
    DuplicateFunction {
        function: EcuFunction,
    },
    DuplicatePin {
        pin_id: String,
        existing: EcuFunction,
        requested: EcuFunction,
    },
    FixedPinRequired {
        function: EcuFunction,
        expected_pin_id: &'static str,
        requested_pin_id: String,
    },
    ResourceConflict {
        resource: String,
        existing: EcuFunction,
        requested: EcuFunction,
    },
}

impl From<BoardValidationError> for AssignmentError {
    fn from(error: BoardValidationError) -> Self {
        Self::Board(error)
    }
}

pub fn default_pin_assignments() -> Vec<PinAssignmentRequest<'static>> {
    vec![
        PinAssignmentRequest {
            function: EcuFunction::CrankInput,
            pin_id: "PA0",
        },
        PinAssignmentRequest {
            function: EcuFunction::CamInput,
            pin_id: "PA1",
        },
        PinAssignmentRequest {
            function: EcuFunction::MapSensor,
            pin_id: "PC0",
        },
        PinAssignmentRequest {
            function: EcuFunction::TpsSensor,
            pin_id: "PC1",
        },
        PinAssignmentRequest {
            function: EcuFunction::CltSensor,
            pin_id: "PC2",
        },
        PinAssignmentRequest {
            function: EcuFunction::IatSensor,
            pin_id: "PC3",
        },
        PinAssignmentRequest {
            function: EcuFunction::BoostControl,
            pin_id: "PB0",
        },
        PinAssignmentRequest {
            function: EcuFunction::IdleControl,
            pin_id: "PB1",
        },
        PinAssignmentRequest {
            function: EcuFunction::Injector1,
            pin_id: "PE8",
        },
        PinAssignmentRequest {
            function: EcuFunction::Injector2,
            pin_id: "PE9",
        },
        PinAssignmentRequest {
            function: EcuFunction::Ignition1,
            pin_id: "PF8",
        },
        PinAssignmentRequest {
            function: EcuFunction::Ignition2,
            pin_id: "PF9",
        },
    ]
}

pub fn validate_assignment_set(
    requests: &[PinAssignmentRequest<'_>],
) -> Result<Vec<ResolvedPinAssignment>, AssignmentError> {
    let mut resolved: Vec<ResolvedPinAssignment> = Vec::with_capacity(requests.len());

    for request in requests {
        if resolved
            .iter()
            .any(|item| item.function == request.function)
        {
            return Err(AssignmentError::DuplicateFunction {
                function: request.function,
            });
        }

        if let Some(expected_pin_id) = request.function.fixed_pin_id() {
            if request.pin_id != expected_pin_id {
                return Err(AssignmentError::FixedPinRequired {
                    function: request.function,
                    expected_pin_id,
                    requested_pin_id: request.pin_id.to_string(),
                });
            }
        }

        let pin =
            validate_pin_assignment(request.pin_id, request.function.required_pin_function())?;

        if let Some(existing) = resolved.iter().find(|item| item.pin_id == pin.pin_id) {
            return Err(AssignmentError::DuplicatePin {
                pin_id: pin.pin_id.to_string(),
                existing: existing.function,
                requested: request.function,
            });
        }

        if let Some(resource) = resource_key(pin, request.function.required_pin_function()) {
            if let Some(existing) = resolved.iter().find(|item| {
                find_pin(item.pin_id)
                    .and_then(|existing_pin| resource_key(existing_pin, item.required_function))
                    .as_deref()
                    == Some(resource.as_str())
            }) {
                return Err(AssignmentError::ResourceConflict {
                    resource,
                    existing: existing.function,
                    requested: request.function,
                });
            }
        }

        resolved.push(ResolvedPinAssignment {
            function: request.function,
            pin_id: pin.pin_id,
            pin_label: pin.label,
            required_function: request.function.required_pin_function(),
        });
    }

    Ok(resolved)
}

pub fn apply_assignment_overrides(
    base: &[ResolvedPinAssignment],
    overrides: &[PinAssignmentRequest<'_>],
) -> Result<Vec<ResolvedPinAssignment>, AssignmentError> {
    let mut merged = base
        .iter()
        .map(|assignment| PinAssignmentRequest {
            function: assignment.function,
            pin_id: assignment.pin_id,
        })
        .collect::<Vec<_>>();

    for override_request in overrides {
        if let Some(existing) = merged
            .iter_mut()
            .find(|assignment| assignment.function == override_request.function)
        {
            *existing = *override_request;
        } else {
            merged.push(*override_request);
        }
    }

    validate_assignment_set(&merged)
}

fn resource_key(pin: &PinCapability, function: PinFunctionClass) -> Option<String> {
    match function {
        PinFunctionClass::AnalogInput => pin
            .adc_instance
            .zip(pin.adc_channel)
            .map(|(instance, channel)| format!("adc:{instance}:ch{channel}")),
        PinFunctionClass::CaptureInput
        | PinFunctionClass::PwmOutput
        | PinFunctionClass::Injector
        | PinFunctionClass::Ignition
        | PinFunctionClass::LowSideOutput
        | PinFunctionClass::HighSideOutput => pin
            .timer_instance
            .zip(pin.timer_channel)
            .map(|(instance, channel)| format!("timer:{instance}:{channel}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_assignment_overrides, default_pin_assignments, validate_assignment_set,
        AssignmentError, EcuFunction, PinAssignmentRequest,
    };

    #[test]
    fn default_assignment_set_is_valid() {
        let resolved = validate_assignment_set(&default_pin_assignments()).unwrap();
        assert_eq!(resolved.len(), 12);
    }

    #[test]
    fn fixed_hardware_paths_cannot_be_moved() {
        let result = validate_assignment_set(&[PinAssignmentRequest {
            function: EcuFunction::CrankInput,
            pin_id: "PA1",
        }]);

        assert_eq!(
            result,
            Err(AssignmentError::FixedPinRequired {
                function: EcuFunction::CrankInput,
                expected_pin_id: "PA0",
                requested_pin_id: "PA1".to_string(),
            })
        );
    }

    #[test]
    fn flexible_pwm_can_move_to_alternate_compatible_pin() {
        let resolved = validate_assignment_set(&[
            PinAssignmentRequest {
                function: EcuFunction::BoostControl,
                pin_id: "PC8",
            },
            PinAssignmentRequest {
                function: EcuFunction::IdleControl,
                pin_id: "PB1",
            },
        ])
        .unwrap();

        assert!(resolved.iter().any(|item| item.pin_id == "PC8"));
    }

    #[test]
    fn duplicate_pin_assignment_is_rejected() {
        let result = validate_assignment_set(&[
            PinAssignmentRequest {
                function: EcuFunction::BoostControl,
                pin_id: "PB0",
            },
            PinAssignmentRequest {
                function: EcuFunction::IdleControl,
                pin_id: "PB0",
            },
        ]);

        assert_eq!(
            result,
            Err(AssignmentError::DuplicatePin {
                pin_id: "PB0".to_string(),
                existing: EcuFunction::BoostControl,
                requested: EcuFunction::IdleControl,
            })
        );
    }

    #[test]
    fn shared_timer_channel_conflict_is_rejected() {
        let result = validate_assignment_set(&[
            PinAssignmentRequest {
                function: EcuFunction::BoostControl,
                pin_id: "PB0",
            },
            PinAssignmentRequest {
                function: EcuFunction::IdleControl,
                pin_id: "PC8",
            },
        ]);

        assert_eq!(
            result,
            Err(AssignmentError::ResourceConflict {
                resource: "timer:TIM3:CH3".to_string(),
                existing: EcuFunction::BoostControl,
                requested: EcuFunction::IdleControl,
            })
        );
    }

    #[test]
    fn override_replaces_only_requested_function() {
        let base = validate_assignment_set(&default_pin_assignments()).unwrap();
        let overridden = apply_assignment_overrides(
            &base,
            &[PinAssignmentRequest {
                function: EcuFunction::BoostControl,
                pin_id: "PC8",
            }],
        )
        .unwrap();

        assert!(overridden
            .iter()
            .any(|item| { item.function == EcuFunction::BoostControl && item.pin_id == "PC8" }));
        assert!(overridden
            .iter()
            .any(|item| { item.function == EcuFunction::IdleControl && item.pin_id == "PB1" }));
    }
}
