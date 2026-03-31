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
- preliminary STM32H743 board definition and pin capability matrix
- ECU-level IO assignment validator with hardware-path and MCU-resource checks
- board-path metadata that distinguishes conditioned trigger inputs, protected
  analog inputs, solenoid drivers, injector drivers, ignition drivers, native
  USB, CAN transceiver pins, WiFi bridge UART, and debug access
- per-pin mux routes that identify the logical function class, mux mode, signal
  name, and exclusive resource key used for conflict detection
- protocol payloads for exposing board pins and active IO assignments
- firmware identity, capability, and compatibility structs
- page directory and table directory constants
- RAM/flash config staging model with CRC and burn detection
- packet framing for version, capabilities, and page payloads
