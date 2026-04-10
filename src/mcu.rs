use crate::board::{PinCapability, ST_ECU_V1_PINS};
use crate::pinmux::PinRoute;

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

const DATASHEET_REFERENCE: &str = "DS12110 Table 9";
const EMPTY_ROUTES: &[PinRoute] = &[];
const EMPTY_MCU_PIN: McuPinCapability = McuPinCapability {
    pin_id: "",
    port: 'A',
    pin_number: 0,
    voltage_tolerance: "",
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
    datasheet_ref: DATASHEET_REFERENCE,
    routes: EMPTY_ROUTES,
};

const fn from_board_pin(pin: PinCapability) -> McuPinCapability {
    McuPinCapability {
        pin_id: pin.pin_id,
        port: pin.port,
        pin_number: pin.pin_number,
        voltage_tolerance: pin.voltage_tolerance,
        supports_adc: pin.supports_adc,
        supports_pwm: pin.supports_pwm,
        supports_capture: pin.supports_capture,
        supports_gpio_in: pin.supports_gpio_in,
        supports_gpio_out: pin.supports_gpio_out,
        supports_can: pin.supports_can,
        supports_uart: pin.supports_uart,
        supports_spi: pin.supports_spi,
        supports_i2c: pin.supports_i2c,
        timer_instance: pin.timer_instance,
        timer_channel: pin.timer_channel,
        adc_instance: pin.adc_instance,
        adc_channel: pin.adc_channel,
        datasheet_ref: DATASHEET_REFERENCE,
        routes: pin.routes,
    }
}

const fn build_selected_pins() -> [McuPinCapability; ST_ECU_V1_PINS.len()] {
    let mut pins = [EMPTY_MCU_PIN; ST_ECU_V1_PINS.len()];
    let mut index = 0;
    while index < ST_ECU_V1_PINS.len() {
        pins[index] = from_board_pin(ST_ECU_V1_PINS[index]);
        index += 1;
    }
    pins
}

pub const STM32H743ZG_SELECTED_PINS: [McuPinCapability; ST_ECU_V1_PINS.len()] =
    build_selected_pins();

pub const STM32H743ZG_SELECTED_MATRIX: McuDefinition = McuDefinition {
    family: "STM32H743ZG",
    package: McuPackage::Lqfp144,
    datasheet: DATASHEET_REFERENCE,
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
