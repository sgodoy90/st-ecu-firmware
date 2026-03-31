use crate::contract::FirmwareIdentity;
use crate::pinmux::{PinFunctionClass, PinRoute};

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

impl ElectricalClass {
    pub const fn key(self) -> &'static str {
        match self {
            Self::AnalogSensor => "analog_sensor",
            Self::DigitalInput => "digital_input",
            Self::FrequencyInput => "frequency_input",
            Self::LogicOutput => "logic_output",
            Self::PwmOutput => "pwm_output",
            Self::PowerLowSide => "power_low_side",
            Self::PowerHighSide => "power_high_side",
            Self::Communication => "communication",
            Self::Reserved => "reserved",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardPathKind {
    NativeUsb,
    PrimaryCanTransceiver,
    TriggerConditionedInput,
    AnalogSensorInput,
    SolenoidPwmDriver,
    InjectorLowSideDriver,
    IgnitionLogicDriver,
    WifiBridgeUart,
    DebugAccess,
}

impl BoardPathKind {
    pub const fn key(self) -> &'static str {
        match self {
            Self::NativeUsb => "native_usb",
            Self::PrimaryCanTransceiver => "primary_can_transceiver",
            Self::TriggerConditionedInput => "trigger_conditioned_input",
            Self::AnalogSensorInput => "analog_sensor_input",
            Self::SolenoidPwmDriver => "solenoid_pwm_driver",
            Self::InjectorLowSideDriver => "injector_low_side_driver",
            Self::IgnitionLogicDriver => "ignition_logic_driver",
            Self::WifiBridgeUart => "wifi_bridge_uart",
            Self::DebugAccess => "debug_access",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinCapability {
    pub pin_id: &'static str,
    pub port: char,
    pub pin_number: u8,
    pub label: &'static str,
    pub electrical_class: ElectricalClass,
    pub board_path: BoardPathKind,
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
    pub routes: &'static [PinRoute],
}

impl PinCapability {
    pub fn supports_function(&self, function: PinFunctionClass) -> bool {
        self.valid_function_classes.contains(&function) && self.route_for(function).is_some()
    }

    pub fn route_for(&self, function: PinFunctionClass) -> Option<&'static PinRoute> {
        self.routes
            .iter()
            .find(|route| route.function_class == function)
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

const PA11_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Usb,
    mux_mode: "native_usb",
    signal: "USB_OTG_FS_DM",
    exclusive_resource: Some("usb:otg_fs:dm"),
}];
const PA12_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Usb,
    mux_mode: "native_usb",
    signal: "USB_OTG_FS_DP",
    exclusive_resource: Some("usb:otg_fs:dp"),
}];
const PD0_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Can,
    mux_mode: "can_fd",
    signal: "FDCAN1_RX",
    exclusive_resource: Some("can:fdcan1:rx"),
}];
const PD1_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Can,
    mux_mode: "can_fd",
    signal: "FDCAN1_TX",
    exclusive_resource: Some("can:fdcan1:tx"),
}];
const PA0_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::DigitalInput,
        mux_mode: "gpio_input",
        signal: "GPIOA0",
        exclusive_resource: None,
    },
    PinRoute {
        function_class: PinFunctionClass::CaptureInput,
        mux_mode: "timer_capture",
        signal: "TIM2_CH1",
        exclusive_resource: Some("timer:TIM2:CH1"),
    },
];
const PA1_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::DigitalInput,
        mux_mode: "gpio_input",
        signal: "GPIOA1",
        exclusive_resource: None,
    },
    PinRoute {
        function_class: PinFunctionClass::CaptureInput,
        mux_mode: "timer_capture",
        signal: "TIM2_CH2",
        exclusive_resource: Some("timer:TIM2:CH2"),
    },
];
const PC0_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::AnalogInput,
    mux_mode: "analog",
    signal: "ADC1_INP10",
    exclusive_resource: Some("adc:ADC1:ch10"),
}];
const PC1_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::AnalogInput,
    mux_mode: "analog",
    signal: "ADC1_INP11",
    exclusive_resource: Some("adc:ADC1:ch11"),
}];
const PC2_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::AnalogInput,
    mux_mode: "analog",
    signal: "ADC1_INP12",
    exclusive_resource: Some("adc:ADC1:ch12"),
}];
const PC3_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::AnalogInput,
    mux_mode: "analog",
    signal: "ADC1_INP13",
    exclusive_resource: Some("adc:ADC1:ch13"),
}];
const PB0_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::PwmOutput,
        mux_mode: "timer_pwm",
        signal: "TIM3_CH3",
        exclusive_resource: Some("timer:TIM3:CH3"),
    },
    PinRoute {
        function_class: PinFunctionClass::LowSideOutput,
        mux_mode: "timer_pwm",
        signal: "TIM3_CH3",
        exclusive_resource: Some("timer:TIM3:CH3"),
    },
];
const PB1_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::PwmOutput,
        mux_mode: "timer_pwm",
        signal: "TIM3_CH4",
        exclusive_resource: Some("timer:TIM3:CH4"),
    },
    PinRoute {
        function_class: PinFunctionClass::LowSideOutput,
        mux_mode: "timer_pwm",
        signal: "TIM3_CH4",
        exclusive_resource: Some("timer:TIM3:CH4"),
    },
];
const PC8_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::PwmOutput,
        mux_mode: "timer_pwm",
        signal: "TIM3_CH3",
        exclusive_resource: Some("timer:TIM3:CH3"),
    },
    PinRoute {
        function_class: PinFunctionClass::LowSideOutput,
        mux_mode: "timer_pwm",
        signal: "TIM3_CH3",
        exclusive_resource: Some("timer:TIM3:CH3"),
    },
];
const PE9_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Injector,
    mux_mode: "timed_driver",
    signal: "TIM1_CH1",
    exclusive_resource: Some("timer:TIM1:CH1"),
}];
const PE11_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Injector,
    mux_mode: "timed_driver",
    signal: "TIM1_CH2",
    exclusive_resource: Some("timer:TIM1:CH2"),
}];
const PF8_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Ignition,
    mux_mode: "timed_logic",
    signal: "TIM13_CH1",
    exclusive_resource: Some("timer:TIM13:CH1"),
}];
const PF9_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Ignition,
    mux_mode: "timed_logic",
    signal: "TIM14_CH1",
    exclusive_resource: Some("timer:TIM14:CH1"),
}];
const PB6_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Uart,
    mux_mode: "uart",
    signal: "USART1_TX",
    exclusive_resource: Some("uart:USART1:tx"),
}];
const PB7_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Uart,
    mux_mode: "uart",
    signal: "USART1_RX",
    exclusive_resource: Some("uart:USART1:rx"),
}];
const PA13_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Debug,
    mux_mode: "swd",
    signal: "SWDIO",
    exclusive_resource: Some("debug:swdio"),
}];

pub const ST_ECU_V1_PINS: [PinCapability; 20] = [
    PinCapability {
        pin_id: "PA11",
        port: 'A',
        pin_number: 11,
        label: "USB_DM",
        electrical_class: ElectricalClass::Reserved,
        board_path: BoardPathKind::NativeUsb,
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
        routes: PA11_ROUTES,
    },
    PinCapability {
        pin_id: "PA12",
        port: 'A',
        pin_number: 12,
        label: "USB_DP",
        electrical_class: ElectricalClass::Reserved,
        board_path: BoardPathKind::NativeUsb,
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
        routes: PA12_ROUTES,
    },
    PinCapability {
        pin_id: "PD0",
        port: 'D',
        pin_number: 0,
        label: "CAN1_RX",
        electrical_class: ElectricalClass::Communication,
        board_path: BoardPathKind::PrimaryCanTransceiver,
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
        routes: PD0_ROUTES,
    },
    PinCapability {
        pin_id: "PD1",
        port: 'D',
        pin_number: 1,
        label: "CAN1_TX",
        electrical_class: ElectricalClass::Communication,
        board_path: BoardPathKind::PrimaryCanTransceiver,
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
        routes: PD1_ROUTES,
    },
    PinCapability {
        pin_id: "PA0",
        port: 'A',
        pin_number: 0,
        label: "CRANK_IN",
        electrical_class: ElectricalClass::FrequencyInput,
        board_path: BoardPathKind::TriggerConditionedInput,
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
        routes: PA0_ROUTES,
    },
    PinCapability {
        pin_id: "PA1",
        port: 'A',
        pin_number: 1,
        label: "CAM_IN",
        electrical_class: ElectricalClass::FrequencyInput,
        board_path: BoardPathKind::TriggerConditionedInput,
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
        routes: PA1_ROUTES,
    },
    PinCapability {
        pin_id: "PC0",
        port: 'C',
        pin_number: 0,
        label: "MAP",
        electrical_class: ElectricalClass::AnalogSensor,
        board_path: BoardPathKind::AnalogSensorInput,
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
        adc_instance: Some("ADC1"),
        adc_channel: Some(10),
        notes: "Primary MAP sensor path with protected scaling network.",
        valid_function_classes: ANALOG_FUNCTIONS,
        routes: PC0_ROUTES,
    },
    PinCapability {
        pin_id: "PC1",
        port: 'C',
        pin_number: 1,
        label: "TPS",
        electrical_class: ElectricalClass::AnalogSensor,
        board_path: BoardPathKind::AnalogSensorInput,
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
        adc_instance: Some("ADC1"),
        adc_channel: Some(11),
        notes: "Throttle position sensor path.",
        valid_function_classes: ANALOG_FUNCTIONS,
        routes: PC1_ROUTES,
    },
    PinCapability {
        pin_id: "PC2",
        port: 'C',
        pin_number: 2,
        label: "CLT",
        electrical_class: ElectricalClass::AnalogSensor,
        board_path: BoardPathKind::AnalogSensorInput,
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
        adc_instance: Some("ADC1"),
        adc_channel: Some(12),
        notes: "Coolant temperature thermistor input.",
        valid_function_classes: ANALOG_FUNCTIONS,
        routes: PC2_ROUTES,
    },
    PinCapability {
        pin_id: "PC3",
        port: 'C',
        pin_number: 3,
        label: "IAT",
        electrical_class: ElectricalClass::AnalogSensor,
        board_path: BoardPathKind::AnalogSensorInput,
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
        adc_instance: Some("ADC1"),
        adc_channel: Some(13),
        notes: "Intake air temperature thermistor input.",
        valid_function_classes: ANALOG_FUNCTIONS,
        routes: PC3_ROUTES,
    },
    PinCapability {
        pin_id: "PB0",
        port: 'B',
        pin_number: 0,
        label: "BOOST_PWM",
        electrical_class: ElectricalClass::PwmOutput,
        board_path: BoardPathKind::SolenoidPwmDriver,
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
        routes: PB0_ROUTES,
    },
    PinCapability {
        pin_id: "PB1",
        port: 'B',
        pin_number: 1,
        label: "IDLE_PWM",
        electrical_class: ElectricalClass::PwmOutput,
        board_path: BoardPathKind::SolenoidPwmDriver,
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
        routes: PB1_ROUTES,
    },
    PinCapability {
        pin_id: "PC8",
        port: 'C',
        pin_number: 8,
        label: "AUX_PWM_ALT1",
        electrical_class: ElectricalClass::PwmOutput,
        board_path: BoardPathKind::SolenoidPwmDriver,
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
        notes: "Alternate PWM-capable output sharing TIM3 CH3 for resource-conflict validation.",
        valid_function_classes: PWM_FUNCTIONS,
        routes: PC8_ROUTES,
    },
    PinCapability {
        pin_id: "PE9",
        port: 'E',
        pin_number: 9,
        label: "INJ1",
        electrical_class: ElectricalClass::PowerLowSide,
        board_path: BoardPathKind::InjectorLowSideDriver,
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
        routes: PE9_ROUTES,
    },
    PinCapability {
        pin_id: "PE11",
        port: 'E',
        pin_number: 11,
        label: "INJ2",
        electrical_class: ElectricalClass::PowerLowSide,
        board_path: BoardPathKind::InjectorLowSideDriver,
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
        routes: PE11_ROUTES,
    },
    PinCapability {
        pin_id: "PF8",
        port: 'F',
        pin_number: 8,
        label: "IGN1",
        electrical_class: ElectricalClass::LogicOutput,
        board_path: BoardPathKind::IgnitionLogicDriver,
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
        routes: PF8_ROUTES,
    },
    PinCapability {
        pin_id: "PF9",
        port: 'F',
        pin_number: 9,
        label: "IGN2",
        electrical_class: ElectricalClass::LogicOutput,
        board_path: BoardPathKind::IgnitionLogicDriver,
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
        routes: PF9_ROUTES,
    },
    PinCapability {
        pin_id: "PB6",
        port: 'B',
        pin_number: 6,
        label: "WIFI_UART_TX",
        electrical_class: ElectricalClass::Communication,
        board_path: BoardPathKind::WifiBridgeUart,
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
        routes: PB6_ROUTES,
    },
    PinCapability {
        pin_id: "PB7",
        port: 'B',
        pin_number: 7,
        label: "WIFI_UART_RX",
        electrical_class: ElectricalClass::Communication,
        board_path: BoardPathKind::WifiBridgeUart,
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
        routes: PB7_ROUTES,
    },
    PinCapability {
        pin_id: "PA13",
        port: 'A',
        pin_number: 13,
        label: "SWDIO",
        electrical_class: ElectricalClass::Reserved,
        board_path: BoardPathKind::DebugAccess,
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
        routes: PA13_ROUTES,
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
        assignable_pins, board_definition, board_matches_firmware_identity, find_pin,
        validate_pin_assignment, BoardPathKind, BoardValidationError, PinFunctionClass,
    };
    use crate::contract::FirmwareIdentity;
    use crate::mcu::find_mcu_pin;

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

    #[test]
    fn pin_route_metadata_exposes_board_and_resource_truth() {
        let boost = find_pin("PB0").unwrap();
        let route = boost.route_for(PinFunctionClass::PwmOutput).unwrap();

        assert_eq!(boost.board_path, BoardPathKind::SolenoidPwmDriver);
        assert_eq!(route.mux_mode, "timer_pwm");
        assert_eq!(route.signal, "TIM3_CH3");
        assert_eq!(route.exclusive_resource, Some("timer:TIM3:CH3"));
    }

    #[test]
    fn every_board_pin_is_backed_by_selected_mcu_matrix() {
        for pin in board_definition().pins {
            let mcu_pin = find_mcu_pin(pin.pin_id).unwrap_or_else(|| {
                panic!("board pin {} missing from selected MCU matrix", pin.pin_id)
            });

            assert_eq!(pin.port, mcu_pin.port);
            assert_eq!(pin.pin_number, mcu_pin.pin_number);
            for route in pin.routes {
                assert!(
                    mcu_pin.routes.contains(route),
                    "board route {:?} for {} missing from MCU route set",
                    route,
                    pin.pin_id
                );
            }
        }
    }
}
