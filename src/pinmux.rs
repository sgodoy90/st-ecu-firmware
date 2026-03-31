#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
pub struct PinFunctionClassParseError {
    pub code: u8,
}

impl PinFunctionClass {
    pub const fn code(self) -> u8 {
        match self {
            Self::AnalogInput => 0x01,
            Self::DigitalInput => 0x02,
            Self::CaptureInput => 0x03,
            Self::PwmOutput => 0x04,
            Self::Injector => 0x05,
            Self::Ignition => 0x06,
            Self::LowSideOutput => 0x07,
            Self::HighSideOutput => 0x08,
            Self::Can => 0x09,
            Self::Uart => 0x0A,
            Self::Spi => 0x0B,
            Self::I2c => 0x0C,
            Self::Usb => 0x0D,
            Self::Debug => 0x0E,
        }
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::AnalogInput => "analog_input",
            Self::DigitalInput => "digital_input",
            Self::CaptureInput => "capture_input",
            Self::PwmOutput => "pwm_output",
            Self::Injector => "injector",
            Self::Ignition => "ignition",
            Self::LowSideOutput => "low_side_output",
            Self::HighSideOutput => "high_side_output",
            Self::Can => "can",
            Self::Uart => "uart",
            Self::Spi => "spi",
            Self::I2c => "i2c",
            Self::Usb => "usb",
            Self::Debug => "debug",
        }
    }
}

impl TryFrom<u8> for PinFunctionClass {
    type Error = PinFunctionClassParseError;

    fn try_from(value: u8) -> Result<Self, PinFunctionClassParseError> {
        match value {
            0x01 => Ok(Self::AnalogInput),
            0x02 => Ok(Self::DigitalInput),
            0x03 => Ok(Self::CaptureInput),
            0x04 => Ok(Self::PwmOutput),
            0x05 => Ok(Self::Injector),
            0x06 => Ok(Self::Ignition),
            0x07 => Ok(Self::LowSideOutput),
            0x08 => Ok(Self::HighSideOutput),
            0x09 => Ok(Self::Can),
            0x0A => Ok(Self::Uart),
            0x0B => Ok(Self::Spi),
            0x0C => Ok(Self::I2c),
            0x0D => Ok(Self::Usb),
            0x0E => Ok(Self::Debug),
            _ => Err(PinFunctionClassParseError { code: value }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinRoute {
    pub function_class: PinFunctionClass,
    pub mux_mode: &'static str,
    pub signal: &'static str,
    pub exclusive_resource: Option<&'static str>,
}
