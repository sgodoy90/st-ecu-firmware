# Firmware Architecture Bootstrap

## Guiding Rule
Hardware defines capability.
Firmware defines deterministic behavior.
Desktop defines presentation and workflow.

## Near-Term Build Order
1. `contract`
2. `config`
3. `live_data`
4. `transport`
5. `boot`
6. `engine`
7. `diagnostics`
8. `protection`

## Deliverable For First Real Embedded Milestone
- version response
- schema version response
- capability list
- live-data frame with deterministic layout
- page read/write/burn on a known board target

## Bootstrap Already Landed
- selected `STM32H743ZG` MCU matrix for the current pin subset, separated from
  the board contract so firmware can validate board assumptions against real MCU
  capabilities
- preliminary STM32H743 board definition and pin capability matrix
- ECU-level IO assignment validator with hardware-path and MCU-resource checks
- board-path metadata that distinguishes conditioned trigger inputs, protected
  analog inputs, solenoid drivers, injector drivers, ignition drivers, native
  USB, CAN transceiver pins, WiFi bridge UART, and debug access
- per-pin mux routes that identify the logical function class, mux mode, signal
  name, and exclusive resource key used for conflict detection
- corrected the seeded injector timing channels to independent `TIM1_CH1` and
  `TIM1_CH2` outputs and aligned the seeded `PC3` analog route with a real ADC
  channel
- protocol payloads for exposing board pins and active IO assignments
- protocol payloads for exposing runtime trigger capture and supported trigger decoder presets, including CKP/CMP sensor-kind expectations, edge policy, sync strategy, and expected cycle metadata
- firmware identity, capability, and compatibility structs
- page directory and table directory constants
- versioned RAM/flash config staging model with per-page image headers, CRC, generation, and burn detection
- packet framing for version, capabilities, and page payloads
