use crate::pinmux::{PinFunctionClass, PinRoute};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McuPackage {
    Lqfp144,
}

impl McuPackage {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Lqfp144 => "lqfp144",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McuPinCapability {
    pub pin_id: &'static str,
    pub port: char,
    pub pin_number: u8,
    pub voltage_tolerance: &'static str,
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
    pub datasheet_ref: &'static str,
    pub routes: &'static [PinRoute],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McuDefinition {
    pub family: &'static str,
    pub package: McuPackage,
    pub datasheet: &'static str,
    pub pins: &'static [McuPinCapability],
}

const PA0_MCU_ROUTES: &[PinRoute] = &[
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
    PinRoute {
        function_class: PinFunctionClass::CaptureInput,
        mux_mode: "timer_capture",
        signal: "TIM5_CH1",
        exclusive_resource: Some("timer:TIM5:CH1"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "UART4_TX",
        exclusive_resource: Some("uart:UART4:tx"),
    },
];

const PA1_MCU_ROUTES: &[PinRoute] = &[
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
    PinRoute {
        function_class: PinFunctionClass::CaptureInput,
        mux_mode: "timer_capture",
        signal: "TIM5_CH2",
        exclusive_resource: Some("timer:TIM5:CH2"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "UART4_RX",
        exclusive_resource: Some("uart:UART4:rx"),
    },
];

const PA11_MCU_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Usb,
    mux_mode: "native_usb",
    signal: "USB_OTG_FS_DM",
    exclusive_resource: Some("usb:otg_fs:dm"),
}];

const PA12_MCU_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Usb,
    mux_mode: "native_usb",
    signal: "USB_OTG_FS_DP",
    exclusive_resource: Some("usb:otg_fs:dp"),
}];

const PA13_MCU_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Debug,
    mux_mode: "swd",
    signal: "SWDIO",
    exclusive_resource: Some("debug:swdio"),
}];

const PB0_MCU_ROUTES: &[PinRoute] = &[
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
    PinRoute {
        function_class: PinFunctionClass::PwmOutput,
        mux_mode: "timer_pwm",
        signal: "TIM1_CH2N",
        exclusive_resource: Some("timer:TIM1:CH2N"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "UART4_CTS",
        exclusive_resource: Some("uart:UART4:cts"),
    },
];

const PB1_MCU_ROUTES: &[PinRoute] = &[
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
    PinRoute {
        function_class: PinFunctionClass::PwmOutput,
        mux_mode: "timer_pwm",
        signal: "TIM1_CH3N",
        exclusive_resource: Some("timer:TIM1:CH3N"),
    },
];

const PB6_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "USART1_TX",
        exclusive_resource: Some("uart:USART1:tx"),
    },
    PinRoute {
        function_class: PinFunctionClass::I2c,
        mux_mode: "i2c",
        signal: "I2C1_SCL",
        exclusive_resource: Some("i2c:I2C1:scl"),
    },
];

const PB7_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "USART1_RX",
        exclusive_resource: Some("uart:USART1:rx"),
    },
    PinRoute {
        function_class: PinFunctionClass::I2c,
        mux_mode: "i2c",
        signal: "I2C1_SDA",
        exclusive_resource: Some("i2c:I2C1:sda"),
    },
];

const PC0_MCU_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::AnalogInput,
    mux_mode: "analog",
    signal: "ADC1_INP10",
    exclusive_resource: Some("adc:ADC1:ch10"),
}];

const PC1_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::AnalogInput,
        mux_mode: "analog",
        signal: "ADC1_INP11",
        exclusive_resource: Some("adc:ADC1:ch11"),
    },
    PinRoute {
        function_class: PinFunctionClass::Spi,
        mux_mode: "spi",
        signal: "SPI2_MOSI",
        exclusive_resource: Some("spi:SPI2:mosi"),
    },
];

const PC2_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::AnalogInput,
        mux_mode: "analog",
        signal: "ADC1_INP12",
        exclusive_resource: Some("adc:ADC1:ch12"),
    },
    PinRoute {
        function_class: PinFunctionClass::Spi,
        mux_mode: "spi",
        signal: "SPI2_MISO",
        exclusive_resource: Some("spi:SPI2:miso"),
    },
];

const PC3_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::AnalogInput,
        mux_mode: "analog",
        signal: "ADC1_INP13",
        exclusive_resource: Some("adc:ADC1:ch13"),
    },
    PinRoute {
        function_class: PinFunctionClass::Spi,
        mux_mode: "spi",
        signal: "SPI2_MOSI",
        exclusive_resource: Some("spi:SPI2:mosi"),
    },
];

const PC8_MCU_ROUTES: &[PinRoute] = &[
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
    PinRoute {
        function_class: PinFunctionClass::PwmOutput,
        mux_mode: "timer_pwm",
        signal: "TIM8_CH3",
        exclusive_resource: Some("timer:TIM8:CH3"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "UART5_RTS",
        exclusive_resource: Some("uart:UART5:rts"),
    },
];

const PC10_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::Spi,
        mux_mode: "spi",
        signal: "SPI3_SCK",
        exclusive_resource: Some("spi:SPI3:sck"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "USART3_TX",
        exclusive_resource: Some("uart:USART3:tx"),
    },
];

const PC11_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::Spi,
        mux_mode: "spi",
        signal: "SPI3_MISO",
        exclusive_resource: Some("spi:SPI3:miso"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "USART3_RX",
        exclusive_resource: Some("uart:USART3:rx"),
    },
];

const PC12_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::Spi,
        mux_mode: "spi",
        signal: "SPI3_MOSI",
        exclusive_resource: Some("spi:SPI3:mosi"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "UART5_TX",
        exclusive_resource: Some("uart:UART5:tx"),
    },
];

const PD0_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::Can,
        mux_mode: "can_fd",
        signal: "FDCAN1_RX",
        exclusive_resource: Some("can:fdcan1:rx"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "UART4_RX",
        exclusive_resource: Some("uart:UART4:rx"),
    },
];

const PD1_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::Can,
        mux_mode: "can_fd",
        signal: "FDCAN1_TX",
        exclusive_resource: Some("can:fdcan1:tx"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "UART4_TX",
        exclusive_resource: Some("uart:UART4:tx"),
    },
];

const PE8_MCU_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Injector,
    mux_mode: "timed_driver",
    signal: "TIM1_CH1N",
    exclusive_resource: Some("timer:TIM1:CH1N"),
}];

const PE9_MCU_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Injector,
    mux_mode: "timed_driver",
    signal: "TIM1_CH1",
    exclusive_resource: Some("timer:TIM1:CH1"),
}];

const PE10_MCU_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Injector,
    mux_mode: "timed_driver",
    signal: "TIM1_CH2N",
    exclusive_resource: Some("timer:TIM1:CH2N"),
}];

const PE11_MCU_ROUTES: &[PinRoute] = &[PinRoute {
    function_class: PinFunctionClass::Injector,
    mux_mode: "timed_driver",
    signal: "TIM1_CH2",
    exclusive_resource: Some("timer:TIM1:CH2"),
}];

const PF6_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::PwmOutput,
        mux_mode: "timer_pwm",
        signal: "TIM16_CH1",
        exclusive_resource: Some("timer:TIM16:CH1"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "UART7_RX",
        exclusive_resource: Some("uart:UART7:rx"),
    },
    PinRoute {
        function_class: PinFunctionClass::AnalogInput,
        mux_mode: "analog",
        signal: "ADC3_INP8",
        exclusive_resource: Some("adc:ADC3:ch8"),
    },
];

const PF7_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::PwmOutput,
        mux_mode: "timer_pwm",
        signal: "TIM17_CH1",
        exclusive_resource: Some("timer:TIM17:CH1"),
    },
    PinRoute {
        function_class: PinFunctionClass::Uart,
        mux_mode: "uart",
        signal: "UART7_TX",
        exclusive_resource: Some("uart:UART7:tx"),
    },
    PinRoute {
        function_class: PinFunctionClass::AnalogInput,
        mux_mode: "analog",
        signal: "ADC3_INP3",
        exclusive_resource: Some("adc:ADC3:ch3"),
    },
];

const PF8_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::Ignition,
        mux_mode: "timed_logic",
        signal: "TIM13_CH1",
        exclusive_resource: Some("timer:TIM13:CH1"),
    },
    PinRoute {
        function_class: PinFunctionClass::Spi,
        mux_mode: "spi",
        signal: "SPI5_MISO",
        exclusive_resource: Some("spi:SPI5:miso"),
    },
    PinRoute {
        function_class: PinFunctionClass::AnalogInput,
        mux_mode: "analog",
        signal: "ADC3_INP7",
        exclusive_resource: Some("adc:ADC3:ch7"),
    },
];

const PF9_MCU_ROUTES: &[PinRoute] = &[
    PinRoute {
        function_class: PinFunctionClass::Ignition,
        mux_mode: "timed_logic",
        signal: "TIM14_CH1",
        exclusive_resource: Some("timer:TIM14:CH1"),
    },
    PinRoute {
        function_class: PinFunctionClass::Spi,
        mux_mode: "spi",
        signal: "SPI5_MOSI",
        exclusive_resource: Some("spi:SPI5:mosi"),
    },
    PinRoute {
        function_class: PinFunctionClass::AnalogInput,
        mux_mode: "analog",
        signal: "ADC3_INP2",
        exclusive_resource: Some("adc:ADC3:ch2"),
    },
];

pub const STM32H743ZG_SELECTED_PINS: [McuPinCapability; 27] = [
    McuPinCapability {
        pin_id: "PA0",
        port: 'A',
        pin_number: 0,
        voltage_tolerance: "5V tolerant",
        supports_adc: false,
        supports_pwm: false,
        supports_capture: true,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: true,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM2"),
        timer_channel: Some("CH1"),
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PA0",
        routes: PA0_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PA1",
        port: 'A',
        pin_number: 1,
        voltage_tolerance: "5V tolerant",
        supports_adc: false,
        supports_pwm: false,
        supports_capture: true,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: true,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM2"),
        timer_channel: Some("CH2"),
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PA1",
        routes: PA1_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PA11",
        port: 'A',
        pin_number: 11,
        voltage_tolerance: "3.3V",
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
        datasheet_ref: "DS12110 Table 9, PA11",
        routes: PA11_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PA12",
        port: 'A',
        pin_number: 12,
        voltage_tolerance: "3.3V",
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
        datasheet_ref: "DS12110 Table 9, PA12",
        routes: PA12_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PA13",
        port: 'A',
        pin_number: 13,
        voltage_tolerance: "3.3V",
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
        datasheet_ref: "DS12110 Table 9, PA13",
        routes: PA13_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PB0",
        port: 'B',
        pin_number: 0,
        voltage_tolerance: "5V tolerant",
        supports_adc: false,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: true,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM3"),
        timer_channel: Some("CH3"),
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PB0",
        routes: PB0_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PB1",
        port: 'B',
        pin_number: 1,
        voltage_tolerance: "5V tolerant",
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
        datasheet_ref: "DS12110 Table 9, PB1",
        routes: PB1_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PB6",
        port: 'B',
        pin_number: 6,
        voltage_tolerance: "3.3V",
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: true,
        supports_spi: false,
        supports_i2c: true,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PB6",
        routes: PB6_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PB7",
        port: 'B',
        pin_number: 7,
        voltage_tolerance: "3.3V",
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: true,
        supports_spi: false,
        supports_i2c: true,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PB7",
        routes: PB7_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PC0",
        port: 'C',
        pin_number: 0,
        voltage_tolerance: "3.3V",
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
        datasheet_ref: "DS12110 Table 9, PC0",
        routes: PC0_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PC1",
        port: 'C',
        pin_number: 1,
        voltage_tolerance: "3.3V",
        supports_adc: true,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: true,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: Some("ADC1"),
        adc_channel: Some(11),
        datasheet_ref: "DS12110 Table 9, PC1",
        routes: PC1_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PC2",
        port: 'C',
        pin_number: 2,
        voltage_tolerance: "3.3V",
        supports_adc: true,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: true,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: Some("ADC1"),
        adc_channel: Some(12),
        datasheet_ref: "DS12110 Table 9, PC2",
        routes: PC2_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PC3",
        port: 'C',
        pin_number: 3,
        voltage_tolerance: "3.3V",
        supports_adc: true,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: false,
        supports_spi: true,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: Some("ADC1"),
        adc_channel: Some(13),
        datasheet_ref: "DS12110 Table 9, PC3",
        routes: PC3_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PC8",
        port: 'C',
        pin_number: 8,
        voltage_tolerance: "5V tolerant",
        supports_adc: false,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: true,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM3"),
        timer_channel: Some("CH3"),
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PC8",
        routes: PC8_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PC10",
        port: 'C',
        pin_number: 10,
        voltage_tolerance: "5V tolerant",
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: true,
        supports_spi: true,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PC10",
        routes: PC10_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PC11",
        port: 'C',
        pin_number: 11,
        voltage_tolerance: "5V tolerant",
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: false,
        supports_uart: true,
        supports_spi: true,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PC11",
        routes: PC11_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PC12",
        port: 'C',
        pin_number: 12,
        voltage_tolerance: "5V tolerant",
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: true,
        supports_spi: true,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PC12",
        routes: PC12_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PD0",
        port: 'D',
        pin_number: 0,
        voltage_tolerance: "5V tolerant",
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: true,
        supports_uart: true,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PD0",
        routes: PD0_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PD1",
        port: 'D',
        pin_number: 1,
        voltage_tolerance: "5V tolerant",
        supports_adc: false,
        supports_pwm: false,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: false,
        supports_can: true,
        supports_uart: true,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: None,
        timer_channel: None,
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PD1",
        routes: PD1_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PE8",
        port: 'E',
        pin_number: 8,
        voltage_tolerance: "3.3V gate drive",
        supports_adc: false,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM1"),
        timer_channel: Some("CH1N"),
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PE8",
        routes: PE8_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PE9",
        port: 'E',
        pin_number: 9,
        voltage_tolerance: "3.3V gate drive",
        supports_adc: false,
        supports_pwm: true,
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
        datasheet_ref: "DS12110 Table 9, PE9",
        routes: PE9_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PE10",
        port: 'E',
        pin_number: 10,
        voltage_tolerance: "3.3V gate drive",
        supports_adc: false,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: false,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: false,
        supports_spi: false,
        supports_i2c: false,
        timer_instance: Some("TIM1"),
        timer_channel: Some("CH2N"),
        adc_instance: None,
        adc_channel: None,
        datasheet_ref: "DS12110 Table 9, PE10",
        routes: PE10_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PE11",
        port: 'E',
        pin_number: 11,
        voltage_tolerance: "3.3V gate drive",
        supports_adc: false,
        supports_pwm: true,
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
        datasheet_ref: "DS12110 Table 9, PE11",
        routes: PE11_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PF6",
        port: 'F',
        pin_number: 6,
        voltage_tolerance: "5V tolerant",
        supports_adc: true,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: true,
        supports_spi: true,
        supports_i2c: false,
        timer_instance: Some("TIM16"),
        timer_channel: Some("CH1"),
        adc_instance: Some("ADC3"),
        adc_channel: Some(8),
        datasheet_ref: "DS12110 Table 9, PF6",
        routes: PF6_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PF7",
        port: 'F',
        pin_number: 7,
        voltage_tolerance: "5V tolerant",
        supports_adc: true,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: true,
        supports_spi: true,
        supports_i2c: false,
        timer_instance: Some("TIM17"),
        timer_channel: Some("CH1"),
        adc_instance: Some("ADC3"),
        adc_channel: Some(3),
        datasheet_ref: "DS12110 Table 9, PF7",
        routes: PF7_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PF8",
        port: 'F',
        pin_number: 8,
        voltage_tolerance: "5V tolerant",
        supports_adc: true,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: false,
        supports_spi: true,
        supports_i2c: false,
        timer_instance: Some("TIM13"),
        timer_channel: Some("CH1"),
        adc_instance: Some("ADC3"),
        adc_channel: Some(7),
        datasheet_ref: "DS12110 Table 9, PF8",
        routes: PF8_MCU_ROUTES,
    },
    McuPinCapability {
        pin_id: "PF9",
        port: 'F',
        pin_number: 9,
        voltage_tolerance: "5V tolerant",
        supports_adc: true,
        supports_pwm: true,
        supports_capture: false,
        supports_gpio_in: true,
        supports_gpio_out: true,
        supports_can: false,
        supports_uart: false,
        supports_spi: true,
        supports_i2c: false,
        timer_instance: Some("TIM14"),
        timer_channel: Some("CH1"),
        adc_instance: Some("ADC3"),
        adc_channel: Some(2),
        datasheet_ref: "DS12110 Table 9, PF9",
        routes: PF9_MCU_ROUTES,
    },
];

pub const STM32H743ZG_SELECTED_MATRIX: McuDefinition = McuDefinition {
    family: "STM32H743ZG",
    package: McuPackage::Lqfp144,
    datasheet: "DS12110 Table 9",
    pins: &STM32H743ZG_SELECTED_PINS,
};

pub fn mcu_definition() -> &'static McuDefinition {
    &STM32H743ZG_SELECTED_MATRIX
}

pub fn find_mcu_pin(pin_id: &str) -> Option<&'static McuPinCapability> {
    STM32H743ZG_SELECTED_MATRIX
        .pins
        .iter()
        .find(|pin| pin.pin_id == pin_id)
}

#[cfg(test)]
mod tests {
    use super::{find_mcu_pin, mcu_definition, McuPackage};

    #[test]
    fn selected_matrix_uses_lqfp144_package() {
        assert_eq!(mcu_definition().family, "STM32H743ZG");
        assert_eq!(mcu_definition().package, McuPackage::Lqfp144);
        assert!(mcu_definition().pins.len() >= 20);
    }

    #[test]
    fn selected_matrix_contains_primary_ecu_timing_pins() {
        let injector_1 = find_mcu_pin("PE9").unwrap();
        let injector_2 = find_mcu_pin("PE11").unwrap();

        assert_eq!(injector_1.timer_channel, Some("CH1"));
        assert_eq!(injector_2.timer_channel, Some("CH2"));
    }
}
