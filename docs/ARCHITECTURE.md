# SPECTRA Architecture

## Overview

SPECTRA is a headless, deterministic simulation of a contested electromagnetic environment. The architecture enforces strict separation between the simulation engine, the AI decision system, and any future visualization layer.

## Design Principles

1. **Simulation-first**: Every feature works without a UI.
2. **Deterministic**: Same seed + same scenario = same result.
3. **Typed**: Domain models use strong types, not raw primitives.
4. **Testable**: Every subsystem is independently testable.
5. **Extensible**: New EW effects, policies, and sensors are added via traits.

## Dependency Direction

```
spectra-ui (TUI visualization)
    |
    v
spectra-sim (deterministic tick engine)
    |
    +---> spectra-core  (domain models)
    +---> spectra-sensors (observation layer)
    +---> spectra-ew     (EW effects)
    +---> spectra-ai     (decision policies)
    +---> spectra-experiments (batch runs, metrics)
```

No cycle is permitted. `spectra-core` depends on nothing within the workspace.

## Crate Responsibilities

### spectra-core

The foundation. Contains:

- `id` — EntityId, SignalId, EventId (unique identifiers)
- `world` — World state, Position, Environment, WorldConfig
- `entity` — Entity model with kind, faction, capabilities, state
- `emitter` — Emitter model with channel, frequency, power, category
- `receiver` — Receiver model with sensitivity, noise, detection logic
- `observation` — Observation model with quality, confidence, uncertainty
- `interference` — Interference model with spatial effects, duration
- `action` — EwAction enum and PerformedAction record
- `config` — ScenarioFile and YAML loading

### spectra-sensors

The observation layer between ground truth and AI perception.

Contains sensor models that transform ground-truth signals into imperfect observations. The AI never sees raw World State — only sensor-processed observations.

### spectra-ew

Abstract electronic warfare effects.

Contains the `EwEffect` trait and implementations for jamming, deception, channel flooding, etc. All effects operate on simulation state only — no real RF protocols.

### spectra-ai

Decision framework with three autonomy modes.

Contains the `DecisionPolicy` trait and implementations (passive, rule-based, utility-based). Also contains ethics constraints and autonomy mode management.

### spectra-sim

The deterministic simulation engine.

Orchestrates the tick loop: world update → emitters → propagation → environment → interference → observations → AI perception → decisions → actions → metrics → events.

### spectra-experiments

Batch experiment framework.

Runs many simulations with varying parameters, collects metrics, and exports results as JSON or CSV. Never requires a UI.

### spectra-ui (Phase 11)

Visualization layer only. Consumes the simulation through a clean API. Contains zero simulation logic.

## Data Flow

```
Scenario YAML
    │
    ▼
┌──────────────────────────────────────┐
│  Simulation Engine (spectra-sim)     │
│                                      │
│  For each tick:                      │
│    1. Update world state             │
│    2. Update emitters                │
│    3. Calculate propagation          │
│    4. Apply environment effects      │
│    5. Apply interference             │
│    6. Generate observations          │
│    7. AI perception update           │
│    8. Evaluate decisions             │
│    9. Apply simulated actions        │
│   10. Calculate metrics              │
│   11. Record events                  │
│                                      │
└──────────────────────────────────────┘
    │
    ▼
  Metrics / Export / Replay
```

## Determinism

Determinism is achieved by:

1. Using seeded RNG (`rand_chacha::ChaCha8Rng`)
2. No system time in simulation logic
3. No concurrent state mutation
4. Fixed-point arithmetic where precision matters
5. Deterministic iteration order over entities

## Testing Strategy

- **Unit tests**: Every type and function in `spectra-core`
- **Integration tests**: Subsystem interaction tests in `tests/`
- **Determinism tests**: Replay produces identical results
- **Property tests**: `proptest` for invariant checking
- **Benchmarks**: `criterion` for performance regression
