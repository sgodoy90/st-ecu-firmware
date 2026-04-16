/// ST ECU Hardware Abstraction Layer
///
/// All engine algorithms and protocol modules use these traits.
/// No control code touches registers directly.
///
/// Feature flags:
///   cfg(feature = "f407")  → STM32F407 @ 168 MHz
///   cfg(feature = "h743")  → STM32H743 @ 480 MHz (superset)

pub mod common;

#[cfg(feature = "f407")]
pub mod f407;

#[cfg(feature = "h743")]
pub mod h743;

pub use common::*;
