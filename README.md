# ST ECU Firmware

Local scaffold created: 2026-03-31

## Purpose
This repository is the start of the real ST ECU firmware project.

It exists to do three things cleanly:

- own the embedded runtime contract
- evolve into the STM32H743 firmware target
- stop `st-manager` from carrying implied firmware logic that does not yet exist

## Current Scope
This first scaffold is intentionally architecture-first:

- firmware identity and compatibility contract
- preliminary board definition and pin capability matrix
- selected STM32H743ZG MCU matrix separated from board wiring so board truth can
  be validated against MCU truth instead of being hand-waved into one layer
- board-path and mux-route modeling so each exposed ECU pin declares both its
  hardware conditioning path and the MCU signal route used to drive it
- IO assignment validation with fixed-path and compatible-reroute rules
- runtime protocol surface for pin directory and active assignments
- page and table directory definitions
- live-data frame contract
- packet framing and page payload encoding
- versioned config RAM/flash staging with per-page image headers, CRC, and burn semantics
- protection, reset reason, diagnostics, and transport module boundaries

This is not yet MCU bring-up. The next implementation milestone is:

1. board definition
2. startup and clock bring-up
3. STM32H743 board abstraction
4. startup and clock bring-up
5. ping/version/live frame on transport
6. page read/write/burn backed by MCU flash

## Source Of Truth Relationship
At the moment, shared contract docs live in:

- `../st-manager/CONFIG-PAGE-LAYOUT.md`
- `../st-manager/PIN-MATRIX.md`
- `../st-manager/PROTOCOL-SCHEMA.md`
- `../st-manager/LIVE-DATA-SCHEMA.md`
- `../st-manager/FIRMWARE-COMPATIBILITY.md`

This repository should eventually own the runtime implementation of those contracts.

## Initial Modules
- `boot`: startup and boot mode policy
- `config`: page IDs and page directory
- `contract`: firmware identity and capability contract
- `diagnostics`: DTC and freeze-frame boundaries
- `engine`: trigger, injection, ignition scheduling boundaries
- `trigger`: OEM/generic decoder presets plus runtime trigger-capture contract, including CKP/CMP sensor expectations, sync strategy, and engine-cycle metadata
- `live_data`: frame layout contract
- `protection`: safety and limp policy boundaries
- `reset_reason`: reset/brownout/watchdog reason model
- `transport`: USB/CAN protocol boundaries

## Immediate Next Tasks
- add STM32H743 target and linker config
- expand board definition from seeded production routes to a broader STM32H743
  package matrix and full harness-facing pinout
- keep reconciling board outputs against real timer/ADC channels as the harness
  and power stages are frozen
- wire protocol packets into transport service
- replace the versioned host flash-image backend with a real STM32H743 sector driver
- implement version/identity response on real target
