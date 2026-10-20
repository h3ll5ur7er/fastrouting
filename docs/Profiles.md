# Profiles

A profile trades routing speed against quality. `traxide route --profile NAME` picks one; `default` is built in. A `traxide.toml`, passed with `--config`, changes fields of the built-in profiles or adds profiles of its own; a new profile starts from `default`.

```toml
# traxide.toml: lengths and costs in millimetres
[profiles.default]
grid_pitch = 0.05            # a finer grid for a dense board

[profiles.careful]
heuristic_weight = 1.0       # always the cheapest path, however long the search
via_cost = 4.0               # avoid vias harder
```

Misspelled keys are errors, so a typo never silently does nothing.

## Fields

| Field | Default | Meaning |
| --- | --- | --- |
| `grid_pitch` | 0.1 | Spacing of the routing grid, in mm. Finer finds more ways between pads and costs time |
| `via_cost` | 2.0 | What a via costs, as the length of track (mm) worth avoiding it |
| `bend_cost` | 0.1 | What each 45° change of direction costs, in mm of track |
| `detour_margin` | 5.0 | How far (mm) a connection may stray beyond the box around its two pads |
| `rip_up_cost` | 5.0 | What crossing a routed connection costs (mm of track) when a blocked connection considers ripping it up; it grows with every earlier rip-up of that connection |
| `max_rip_ups` | 10 | How often one connection may be ripped up before it stays unrouted; `0` turns rip-up off |
| `heuristic_weight` | 1.2 | How boldly a search heads for its target. `1.0` finds the cheapest path; more finds a path within that factor of the cheapest, much faster. On a 102-key matrix, 1.2 routed 15 times faster than 1.0 for 0.3 % more track (see [[Benchmarks]]) |

Profiles hold trade-offs, never design rules: widths and clearances come from the board.
