# Contracts

The `contracts` crate holds what every component agrees on: the data that flows between them and the traits each component implements. It hides nothing and depends on nothing but the Geometry utility.

[TOC]

## Units and geometry

- Coordinates are `i32` nanometres, as in KiCad: exact, deterministic and compact (a point is 8 bytes, so a cache line holds 8). Products such as cross products are `i64`.
- Angles of tracks are octilinear (0°, 45°, 90°…) by default; arcs are kept as arcs, never polylines.
- Layers are indices into the stackup, `u8`. A board has at most 64 copper layers.
- Identifiers (`NetId`, `PadId`, `LayerId`, `RouteId`) are dense indices into the design's arrays, never strings or pointers. Names live in side tables for reporting.

## Data contracts

```mermaid
classDiagram
  class Design {
    stackup: Stackup
    outline: Polygon
    nets: Vec~Net~
    pads: Vec~Pad~
    keepouts: Vec~Keepout~
    existing: Vec~Route~
    rules: RuleSource
  }
  class ConstraintSet {
    clearance(class_a, class_b, layer) i32
    width(net, layer) i32
    via_stacks(net) slice
    pairs: Vec~DiffPair~
    length_groups: Vec~LengthGroup~
  }
  class Plan {
    connections: Vec~Connection~
    order: Vec~ConnectionId~
    batches: Vec~Batch~
  }
  class Corridors {
    per_connection: Vec~Corridor~
    congestion: GridMap
  }
  class Routes {
    tracks: Vec~Track~
    vias: Vec~Via~
    status: per connection
  }
  class Report {
    violations: Vec~Violation~
    metrics: Metrics
    timings: StageTimings
  }
  Design --> ConstraintSet : Rules Engine
  Design --> Plan : Planning Engine
  Plan --> Corridors : Global Routing Engine
  Corridors --> Routes : Detailed Routing Engine
  Routes --> Routes : Optimization Engine
  Routes --> Report : Verification Engine
```

| Contract | Produced by | Consumed by |
| --- | --- | --- |
| `Design` | Board Access | Rules, Planning, Layout Access, Verification |
| `ConstraintSet` | Rules Engine | Layout Access and every routing engine |
| `Plan` | Planning Engine | Global and Detailed Routing |
| `Corridors` | Global Routing Engine | Detailed Routing |
| `Routes` | Detailed Routing, Optimization; Board Access for existing copper | Optimization, Verification, Board Access |
| `Report` | Verification Engine, managers (timings) | Run Access, clients |
| `Job`, `Profile` | Clients, Profile Access | Managers |
| `Event` | Managers | Clients (progress, congestion maps, committed batches, finished) |

`Design` is format-neutral. Whatever a DSN file, a `.kicad_pcb` or the KiCad IPC API calls things, Board Access translates them into it. Rules arrive in `Design.rules` as a `RuleSource` close to their origin; only the Rules Engine interprets them.

## Service contracts

Each component implements a trait from `contracts`. Engines are written against the access traits as generic parameters, so the compiler inlines across the boundary (see [Architecture](../Architecture.md#performance-within-the-boundaries)).

```rust
pub trait LayoutAccess: Sync {
    type Txn<'a>: LayoutTxn where Self: 'a;
    fn obstacles_near(&self, layer: LayerId, area: Rect, out: &mut Vec<Obstacle>);
    fn begin(&self, region: Rect) -> Self::Txn<'_>;
}

pub trait LayoutTxn {
    fn add(&mut self, item: RouteItem) -> Result<(), Conflict>;
    fn commit(self) -> Result<(), Conflict>; // optimistic: fails if another worker changed the region
}

pub trait DetailedRouting {
    fn route<L: LayoutAccess, C: ComputeAccess>(
        &self, batch: &Batch, corridors: &Corridors, rules: &ConstraintSet,
        layout: &L, compute: &C, cancel: &CancelToken,
    ) -> BatchOutcome;
}
```

The traits are a sketch; [[FR-4]] fixes them.
