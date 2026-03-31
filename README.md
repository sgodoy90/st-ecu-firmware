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
- page and table directory definitions
- live-data frame contract
- protection, reset reason, diagnostics, and transport module boundaries

This is not yet MCU bring-up. The next implementation milestone is:

1. board definition
2. startup and clock bring-up
3. config storage
4. ping/version/live frame
5. page read/write/burn

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
- `live_data`: frame layout contract
- `protection`: safety and limp policy boundaries
- `reset_reason`: reset/brownout/watchdog reason model
- `transport`: USB/CAN protocol boundaries

## Immediate Next Tasks
- add STM32H743 target and linker config
- add board definition and pin matrix module
- implement config page header + CRC
- implement version/identity response
- implement read/write page service
