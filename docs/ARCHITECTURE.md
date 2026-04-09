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
9. `tcu_bridge`
10. `wideband_controller`

## Deliverable For First Real Embedded Milestone
- version response
- schema version response
- capability list
- live-data frame with deterministic layout
- page read/write/burn on a known board target

## New Contract Tracks (2026-04-09)

### External TCU profile (TurboLamik-inspired reference)
- firmware capability key for external TCU integration profile
- explicit RX/TX schema with rate checks, timeout handling, and safety fallbacks
- torque-intervention arbitration layer so transmission requests are handled deterministically
- runtime diagnostics for stream freshness, request validity, and shift/adaptation states

### Integrated wideband controller (`L9780TR`)
- dedicated wideband-controller ownership in firmware (SPI/service lifecycle)
- heater-state and controller-fault diagnostics surfaced into transport/diagnostics
- mode negotiation for integrated-controller vs external-CAN wideband vs analog fallback
- calibration schema tied to declared controller mode so desktop does not guess behavior

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
- protocol payloads for exposing runtime trigger capture, tooth logging, and supported trigger decoder presets, including CKP/CMP sensor-kind expectations, primary/secondary pattern hints, reference descriptions, edge policy, sync strategy, expected cycle metadata, reference-event tagging, and phase-event markers
- firmware identity, capability, and compatibility structs
- page directory and table directory constants
- versioned RAM/flash config staging model with per-page image headers, CRC, generation, and burn detection
- packet framing for version, capabilities, and page payloads
