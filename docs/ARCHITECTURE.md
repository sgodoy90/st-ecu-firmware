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
