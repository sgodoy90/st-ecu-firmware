use crate::contract::FirmwareIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectricalClass {
    AnalogSensor,
    DigitalInput,
    FrequencyInput,
    LogicOutput,
    PwmOutput,
    PowerLowSide,
    PowerHighSide,
    Communication,
    Reserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinFunctionClass {
    AnalogInput,
    DigitalInput,
    CaptureInput,
    PwmOutput,
    Injector,
    Ignition,
    LowSideOutput,
    HighSideOutput,
    Can,
    Uart,
    Spi,
    I2c,
    Usb,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinCapability {
    pub pin_id: &'static str,
    pub port: char,
    pub pin_number: u8,
    pub label: &'static str,
    pub electrical_class: ElectricalClass,
    pub voltage_tolerance: &'static str,
    pub max_current_ma: u16,
    pub reserved: bool,
    pub supports_adc: bool,
    pub supports_pwm: bool,
    pub supports_capture: bool,
    pub supports_gpio_in: bool,
    pub supports_gpio_out: bool,
    pub supports_can: bool,
    pub supports_uart: bool,
    pub supports_spi: bool,
    pub supports_i2c: bool,
    pub timer_instance: Option<&'static str>,
    pub timer_channel: Option<&'static str>,
    pub adc_instance: Option<&'static str>,
    pub adc_channel: Option<u8>,
    pub notes: &'static str,
    pub valid_function_classes: &'static [PinFunctionClass],
}

impl PinCapability {
    pub fn supports_function(&self, function: PinFunctionClass) -> bool {
        self.valid_function_classes.contains(&function)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardDefinition {
    pub board_id: &'static str,
    pub mcu: &'static str,
    pub pins: &'static [PinCapability],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardValidationError {
    UnknownPin,
    ReservedPin,
    UnsupportedFunction,
}

const USB_FUNCTIONS: &[PinFunctionClass] = &[PinFunctionClass::Usb];
const CAN_FUNCTIONS: &[PinFunctionClass] = &[PinFunctionClass::Can];
const CAPTURE_FUNCTIONS: &[PinFunctionClass] = &[
    PinFunctionClass::DigitalInput,
    PinFunctionClass::CaptureInput,
];
const ANALOG_FUNCTIONS: &[PinFunctionClass] = &[PinFunctionClass::AnalogInput];
const PWM_FUNCTIONS: &[PinFunctionClass] =
    &[PinFunctionClass::PwmOutput, PinFunctionClass::LowSideOutput];
const INJECTOR_FUNCTIONS: &[PinFunctionClass] = &[PinFunctionClass::Injector];
const IGNITION_FUNCTIONS: &[PinFunctionClass] = &[PinFunctionClass::Ignition];
const UART_FUNCTIONS: &[PinFunctionClass] = &[PinFunctionClass::Uart];
const DEBUG_FUNCTIONS: &[PinFunctionClass] = &[PinFunctionClass::Debug];

pub const ST_ECU_V1_PINS: [PinCapability; 19] = [
    PinCapability {
        pin_id: "PA11",
        port: 'A',
        pin_number: 11,
        label: "USB_DM",
        electrical_class: ElectricalClass::Reserved,
        voltage_tolerance: "3.3V",
        max_current_ma: 8,
        reserved: true,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        notes: "Reserved for native USB D-.",
        valid_function_classes: USB_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PA12",
        port: 'A',
        pin_number: 12,
        label: "USB_DP",
        electrical_class: ElectricalClass::Reserved,
        voltage_tolerance: "3.3V",
        max_current_ma: 8,
        reserved: true,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        notes: "Reserved for native USB D+.",
        valid_function_classes: USB_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PD0",
        port: 'D',
        pin_number: 0,
        label: "CAN1_RX",
        electrical_class: ElectricalClass::Communication,
        voltage_tolerance: "5V tolerant",
        max_current_ma: 8,
        reserved: true,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: true,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        notes: "Hard-routed to primary CAN-FD transceiver.",
        valid_function_classes: CAN_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PD1",
        port: 'D',
        pin_number: 1,
        label: "CAN1_TX",
        electrical_class: ElectricalClass::Communication,
        voltage_tolerance: "5V tolerant",
        max_current_ma: 8,
        reserved: true,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: true,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        notes: "Hard-routed to primary CAN-FD transceiver.",
        valid_function_classes: CAN_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PA0",
        port: 'A',
        pin_number: 0,
        label: "CRANK_IN",
        electrical_class: ElectricalClass::FrequencyInput,
        voltage_tolerance: "5V tolerant",
        max_current_ma: 4,
        reserved: false,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: true,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM2"),
        timer_channel: Some("CH1"),
        adc_instance: None,
        adc_channel: None,
        notes: "Primary crank trigger input behind dedicated conditioner.",
        valid_function_classes: CAPTURE_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PA1",
        port: 'A',
        pin_number: 1,
        label: "CAM_IN",
        electrical_class: ElectricalClass::FrequencyInput,
        voltage_tolerance: "5V tolerant",
        max_current_ma: 4,
        reserved: false,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: true,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM2"),
        timer_channel: Some("CH2"),
        adc_instance: None,
        adc_channel: None,
        notes: "Primary cam trigger input behind dedicated conditioner.",
        valid_function_classes: CAPTURE_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PC0",
        port: 'C',
        pin_number: 0,
        label: "MAP",
        electrical_class: ElectricalClass::AnalogSensor,
        voltage_tolerance: "3.3V",
        max_current_ma: 2,
        reserved: false,
        supports_adc: true,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: Some("ADC3"),
        adc_channel: Some(10),
        notes: "Primary MAP sensor path with protected scaling network.",
        valid_function_classes: ANALOG_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PC1",
        port: 'C',
        pin_number: 1,
        label: "TPS",
        electrical_class: ElectricalClass::AnalogSensor,
        voltage_tolerance: "3.3V",
        max_current_ma: 2,
        reserved: false,
        supports_adc: true,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: Some("ADC3"),
        adc_channel: Some(11),
        notes: "Throttle position sensor path.",
        valid_function_classes: ANALOG_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PC2",
        port: 'C',
        pin_number: 2,
        label: "CLT",
        electrical_class: ElectricalClass::AnalogSensor,
        voltage_tolerance: "3.3V",
        max_current_ma: 2,
        reserved: false,
        supports_adc: true,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: Some("ADC3"),
        adc_channel: Some(12),
        notes: "Coolant temperature thermistor input.",
        valid_function_classes: ANALOG_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PC3",
        port: 'C',
        pin_number: 3,
        label: "IAT",
        electrical_class: ElectricalClass::AnalogSensor,
        voltage_tolerance: "3.3V",
        max_current_ma: 2,
        reserved: false,
        supports_adc: true,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: Some("ADC3"),
        adc_channel: Some(13),
        notes: "Intake air temperature thermistor input.",
        valid_function_classes: ANALOG_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PB0",
        port: 'B',
        pin_number: 0,
        label: "BOOST_PWM",
        electrical_class: ElectricalClass::PwmOutput,
        voltage_tolerance: "5V tolerant",
        max_current_ma: 20,
        reserved: false,
        supports_adc: false,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM3"),
        timer_channel: Some("CH3"),
        adc_instance: None,
        adc_channel: None,
        notes: "Boost control solenoid output.",
        valid_function_classes: PWM_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PB1",
        port: 'B',
        pin_number: 1,
        label: "IDLE_PWM",
        electrical_class: ElectricalClass::PwmOutput,
        voltage_tolerance: "5V tolerant",
        max_current_ma: 20,
        reserved: false,
        supports_adc: false,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM3"),
        timer_channel: Some("CH4"),
        adc_instance: None,
        adc_channel: None,
        notes: "Idle valve or DBW fallback PWM output.",
        valid_function_classes: PWM_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PE8",
        port: 'E',
        pin_number: 8,
        label: "INJ1",
        electrical_class: ElectricalClass::PowerLowSide,
        voltage_tolerance: "3.3V gate drive",
        max_current_ma: 1500,
        reserved: false,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM1"),
        timer_channel: Some("CH1"),
        adc_instance: None,
        adc_channel: None,
        notes: "Dedicated injector channel 1 low-side driver.",
        valid_function_classes: INJECTOR_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PE9",
        port: 'E',
        pin_number: 9,
        label: "INJ2",
        electrical_class: ElectricalClass::PowerLowSide,
        voltage_tolerance: "3.3V gate drive",
        max_current_ma: 1500,
        reserved: false,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM1"),
        timer_channel: Some("CH2"),
        adc_instance: None,
        adc_channel: None,
        notes: "Dedicated injector channel 2 low-side driver.",
        valid_function_classes: INJECTOR_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PF8",
        port: 'F',
        pin_number: 8,
        label: "IGN1",
        electrical_class: ElectricalClass::LogicOutput,
        voltage_tolerance: "5V tolerant",
        max_current_ma: 20,
        reserved: false,
        supports_adc: false,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM13"),
        timer_channel: Some("CH1"),
        adc_instance: None,
        adc_channel: None,
        notes: "Ignition channel 1 logic-level output.",
        valid_function_classes: IGNITION_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PF9",
        port: 'F',
        pin_number: 9,
        label: "IGN2",
        electrical_class: ElectricalClass::LogicOutput,
        voltage_tolerance: "5V tolerant",
        max_current_ma: 20,
        reserved: false,
        supports_adc: false,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM14"),
        timer_channel: Some("CH1"),
        adc_instance: None,
        adc_channel: None,
        notes: "Ignition channel 2 logic-level output.",
        valid_function_classes: IGNITION_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PB6",
        port: 'B',
        pin_number: 6,
        label: "WIFI_UART_TX",
        electrical_class: ElectricalClass::Communication,
        voltage_tolerance: "3.3V",
        max_current_ma: 8,
        reserved: true,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: true,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        notes: "Reserved for ESP32-C6 bridge TX.",
        valid_function_classes: UART_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PB7",
        port: 'B',
        pin_number: 7,
        label: "WIFI_UART_RX",
        electrical_class: ElectricalClass::Communication,
        voltage_tolerance: "3.3V",
        max_current_ma: 8,
        reserved: true,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: true,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        notes: "Reserved for ESP32-C6 bridge RX.",
        valid_function_classes: UART_FUNCTIONS,
    },
    PinCapability {
        pin_id: "PA13",
        port: 'A',
        pin_number: 13,
        label: "SWDIO",
        electrical_class: ElectricalClass::Reserved,
        voltage_tolerance: "3.3V",
        max_current_ma: 8,
        reserved: true,
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        notes: "Reserved for production and recovery debug access.",
        valid_function_classes: DEBUG_FUNCTIONS,
    },
];

pub const ST_ECU_V1_BOARD: BoardDefinition = BoardDefinition {
    board_id: "st-ecu-v1",
    mcu: "STM32H743",
    pins: &ST_ECU_V1_PINS,
};

pub fn board_definition() -> &'static BoardDefinition {
    &ST_ECU_V1_BOARD
}

pub fn find_pin(pin_id: &str) -> Option<&'static PinCapability> {
    ST_ECU_V1_BOARD.pins.iter().find(|pin| pin.pin_id == pin_id)
}

pub fn assignable_pins() -> Vec<&'static PinCapability> {
    ST_ECU_V1_BOARD
        .pins
        .iter()
        .filter(|pin| !pin.reserved)
        .collect()
}

pub fn validate_pin_assignment(
    pin_id: &str,
    function: PinFunctionClass,
) -> Result<&'static PinCapability, BoardValidationError> {
    let pin = find_pin(pin_id).ok_or(BoardValidationError::UnknownPin)?;
    if pin.reserved {
        return Err(BoardValidationError::ReservedPin);
    }
    if !pin.supports_function(function) {
        return Err(BoardValidationError::UnsupportedFunction);
    }
    Ok(pin)
}

pub fn board_matches_firmware_identity(identity: &FirmwareIdentity) -> bool {
    identity.board_id == ST_ECU_V1_BOARD.board_id
}

#[cfg(test)]
mod tests {
    use super::{
        assignable_pins, board_definition, board_matches_firmware_identity,
        validate_pin_assignment, BoardValidationError, PinFunctionClass,
    };
    use crate::contract::FirmwareIdentity;

    #[test]
    fn board_id_matches_runtime_identity() {
        assert!(board_matches_firmware_identity(&FirmwareIdentity::ecu_v1()));
        assert_eq!(board_definition().mcu, "STM32H743");
    }

    #[test]
    fn reserved_pin_cannot_be_assigned() {
        let result = validate_pin_assignment("PA11", PinFunctionClass::Usb);
        assert_eq!(result, Err(BoardValidationError::ReservedPin));
    }

    #[test]
    fn analog_pin_accepts_only_analog_function() {
        let pin = validate_pin_assignment("PC0", PinFunctionClass::AnalogInput).unwrap();
        assert_eq!(pin.label, "MAP");
        assert_eq!(
            validate_pin_assignment("PC0", PinFunctionClass::PwmOutput),
            Err(BoardValidationError::UnsupportedFunction)
        );
    }

    #[test]
    fn assignable_pins_exclude_reserved_resources() {
        let pins = assignable_pins();
        assert!(pins.iter().all(|pin| !pin.reserved));
        assert!(pins.iter().any(|pin| pin.label == "INJ1"));
    }
}
