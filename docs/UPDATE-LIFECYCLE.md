# ST ECU Firmware Update Lifecycle

Last updated: 2026-04-18

## Goal
Deliver firmware updates over USB/CAN paths while preserving calibration data and guaranteeing ECU recoverability.

## Mandatory Embedded Layout

1. Immutable bootloader partition.
2. Application Bank A.
3. Application Bank B.
4. Dedicated config partition:
   - page headers
   - schema/format versions
   - generation counters
   - payload/image CRC
5. Optional recovery metadata partition:
   - pending image hash/size
   - target bank
   - boot-attempt counter
   - last-known-good bank

## Update State Machine (Bootloader-Owned)

1. `Idle`
2. `SessionStarted` (`EnterBootloader`)
3. `ReceivingBlocks` (`FlashBlock` sequence-checked)
4. `ImageVerified` (`FlashVerify` CRC/signature-checked)
5. `PendingCommit` (write pending metadata + target bank)
6. `RebootToCandidate`
7. `CandidateHealthWindow`
8. `Committed` or `RolledBack`

## Health and Rollback Rules

- Candidate app must emit a positive health marker before timeout.
- Watchdog reset, hard fault, or missing health marker during window triggers rollback.
- Rollback always returns to last-known-good bank.
- Bootloader keeps rollback counter for diagnostics.

## Config/Tune Preservation Rules

- App-bank update must not erase config partition.
- Bootloader may refuse update if config partition health is invalid and recovery policy requires operator confirmation.
- On app startup, config page CRC/image validity is checked before use.
- Invalid pages remain recoverable through explicit reburn workflow.

## Schema Migration Rules

- Migrators are explicit per version step:
  - `v1_to_v2`
  - `v2_to_v3`
- Forward-only migrators run on startup when persisted schema is older.
- If any migration step fails:
  - keep previous committed image untouched
  - expose incompatibility + recovery reason
  - deny unsafe write paths

## Firmware/Desktop Contract Requirements

Firmware identity payload must include:

- `protocol_version`
- `schema_version`
- `firmware_id`
- `firmware_semver`
- `board_id`
- `capabilities[]`

Capability gating:

- `firmware_flash` controls update pathway availability.
- missing `firmware_flash` means desktop must block normal update flow (except explicit legacy recovery workflow).

## Test Matrix (Firmware Side)

- Ordered block enforcement.
- CRC mismatch rejection.
- Blank-image rejection.
- Power loss in each state:
  - before verify
  - after verify, before pending commit
  - after pending commit, before health verdict
- rollback to last-known-good bank.
- config partition retained across app-bank swap.

## Current Status Snapshot (2026-04-18)

Already present:

- protocol commands for firmware flash lifecycle
- ordered-block validation
- optional verify CRC handling
- `FlashComplete` gated by successful verify in the same session
- runtime update state machine with status reporting:
  - `GetUpdateStatus` -> `UpdateStatus`
  - `ConfirmBootHealthy` for commit handshake
  - explicit `Idle/Receiving/Verified/PendingCommit/HealthWindow/Committed/RolledBack` states
- health-window timeout rollback in runtime path with rollback counter tracking
- config page image/header CRC and dirty-state tracking
- config image CRC validation that respects image header schema/format versions
- persisted config import path with explicit schema/format migration executor baseline
  - when migrator steps are missing, firmware fails closed with explicit reason (no silent coercion)

Still required for production anti-brick:

- real target bootloader implementation (STM32H743)
- bank swap + pending metadata persistence
- first-boot health marker + watchdog-managed rollback
- concrete per-version migrators (`vN_to_vN+1`) wired to target startup path
