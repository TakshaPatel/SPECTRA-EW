# SPECTRA

**Spectrum-based Electronic Warfare Tactical Reasoning & Analysis**

A research-oriented simulation of contested electromagnetic environments. Models abstract emitters, receivers, signal propagation, environmental effects, interference, simulated jamming, simulated deception, sensor uncertainty, and AI-driven decision-making within a fictional, non-operational simulation.

## Purpose

SPECTRA is intended for cybersecurity, systems engineering, and simulation research. It provides a framework for studying how autonomous agents make decisions under imperfect information in contested spectrum environments.


**This project does not:**

- Control real RF hardware
- Transmit RF signals
- Configure real jammers
- Interface with SDR (Software Defined Radio) hardware
- Provide operational instructions for disrupting real systems

All parameters are abstract simulation values operating entirely inside a virtual environment.

## Quick Start

```bash
cargo test --workspace          # Run all 145 tests
cargo run -p spectra-ui -- scenarios/combat.yaml 42   # Launch TUI
```

## Architecture

```
spectra-ui (TUI visualization)
    |
    v
spectra-sim (deterministic tick engine)
    |
    +---> spectra-core  (domain models, world state)
    +---> spectra-sensors (observation layer, noise, degradation)
    +---> spectra-ew     (EW effects: jamming, deception, flooding)
    +---> spectra-ai     (tactical AI, fog of war, decision errors)
    +---> spectra-experiments (batch runs, metrics, export)
```

`spectra-core` depends on nothing within the workspace. No cycles are permitted.

## Crates

| Crate | Responsibility | Tests |
|-------|---------------|-------|
| `spectra-core` | Domain models, IDs, world state, propagation, events, metrics | 49 |
| `spectra-sensors` | Sensor models, noise injection, observation processing, degradation | 27 |
| `spectra-ew` | `EwEffect` trait, 5 jamming/deception effects, `EwManager` | 17 |
| `spectra-ai` | `TacticalAI`, fog of war, decision errors, configurable error rates | 23 |
| `spectra-sim` | Deterministic simulation engine, scenario loader, tick processing | 11 |
| `spectra-experiments` | `ExperimentRunner`, batch runs, metrics aggregation, JSON export | 18 |
| `spectra-ui` | TUI with ratatui: world map, observation/AI/EW panels, keyboard controls | - |

## Simulation Model

### Entities and Factions

Entities belong to factions (Blue/Red/Neutral) and carry emitters and/or receivers. Each entity occupies a position on a 2D map.


### Sensor Processing

Ground-truth signals pass through a sensor layer that injects realistic imperfections:
- Gaussian noise on signal strength, position, and frequency estimates
- Probabilistic detection (false negatives based on SNR and sensor health)
- False positive generation at low SNR
- Progressive sensor degradation over time

The AI never sees raw world state — only sensor-processed observations.

### AI Decision-Making

The `TacticalAI` evaluates observations and produces EW actions:

| Action | EW Effect Deployed |
|--------|-------------------|
| `SuppressSignal` | Noise jamming toward hostile emitter |
| `Monitor` | Lightweight noise jamming |
| `ProtectChannel` | Channel flooding across monitored channels |
| `ChangeChannel` | Sensor confusion (raises local noise floor) |
| `DeployDecoy` | Deception jamming (false emitter injection) |
| `Observe` | No action |
| `Disengage` | No action |

The AI includes configurable error simulation:
- Missed signals (fail to detect)
- Misclassification (wrong source attribution)
- False detections (phantom signals)
- Response delay (actions queued and delayed)
- Fog of war (staleness tracking per observer)

### EW Effects

Five concrete effects implement the `EwEffect` trait:

| Effect | Description |
|--------|-------------|
| `NoiseJamming` | Broadband noise投放 in radius around target |
| `DeceptionJamming` | Injects false emitter with spoofed parameters |
| `ChannelFlooding` | Floods specific channels with interference |
| `CrossChannelInterference` | Cross-talk interference between channels |
| `SensorConfusion` | Raises noise floor degrading nearby sensors |

### Determinism

Same scenario + same seed = identical simulation output. IDs are generated sequentially via `IdGenerator`, not global atomics. All constructors take explicit `EntityId` parameters.

## Scenarios

YAML-based scenario definitions in `scenarios/`:

| Scenario | Entities | Description |
|----------|----------|-------------|
| `combat.yaml` | 4 (2 Blue, 2 Red) | Aggressive EW engagement with radar and comms |
| `basic.yaml` | 2 (1 Blue, 1 Red) | Minimal two-faction encounter |
| `contested_spectrum.yaml` | Multi-entity | Larger contested spectrum environment |
| `deception_test.yaml` | - | Tests deception jamming effects |

### Scenario Format

```yaml
name: Scenario Name
description: What happens

world:
  map_width: 400.0
  map_height: 400.0
  max_ticks: 200
  environment:
    propagation_loss_exponent: 2.0
    ambient_noise_db: -100.0
    weather_attenuation: 0.0
    terrain_masking: false

entities:
  - kind: Aircraft
    faction: Blue
    position: { x: 100.0, y: 200.0 }
    label: Blue-1
    emits:
      - channel: 1
        frequency_mhz: 100.0
        category: Communication
        power_db: 60.0
    receives: true
```

## TUI Controls

| Key | Action |
|-----|--------|
| `Space` / `N` | Step one tick |
| `R` | Toggle auto-run |
| `+` / `-` | Adjust simulation speed |
| `?` / `H` | Toggle help overlay |
| `Q` / `Esc` | Quit |

## Development

Built incrementally across 7 phases:

1. **Phase 0** — Workspace, crate structure, architecture docs
2. **Phase 1** — Domain models (World, Entity, Emitter, Receiver, Observation, Interference, EwAction)
3. **Phase 2** — Deterministic simulation engine (tick processing, scenario loading, events)
4. **Phase 3** — Sensor subsystem (noise, false positives/negatives, degradation, observation processor)
5. **Phase 4** — EW subsystem (`EwEffect` trait, 5 effects, `EwManager`)
6. **Phase 5** — AI decision-making (`TacticalAI`, fog of war, decision errors)
7. **Phase 6** — Experiments (`ExperimentRunner`, metrics, batch runs, JSON export)
8. **Phase 7** — TUI visualization (ratatui: world map, observation/AI/EW panels)

### Running Tests

```bash
cargo test --workspace                    # All 145 tests
cargo test -p spectra-core                # Core domain tests
cargo test -p spectra-sensors             # Sensor layer tests
cargo test -p spectra-ew                  # EW effect tests
cargo test -p spectra-ai                  # AI decision tests
cargo test -p spectra-experiments         # Experiment runner tests
```

### Linting

```bash
cargo fmt --all
cargo clippy --workspace --all-targets
```

## Dependencies

| Dependency | Purpose |
|-----------|---------|
| `serde` / `serde_yaml` / `serde_json` | Serialization |
| `rand` / `rand_chacha` | Deterministic RNG |
| `ratatui` / `crossterm` | TUI framework |
| `thiserror` / `anyhow` | Error handling |
| `tracing` | Structured logging |


## Demo Screenshot


<img width="1772" height="1116" alt="demo_screenshot" src="https://github.com/user-attachments/assets/48e7abca-ed34-4c3d-8b1b-dffce3acd70a" />


## License

MIT
