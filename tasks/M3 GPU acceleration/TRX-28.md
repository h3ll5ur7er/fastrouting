---
id: TRX-28
title: Run kernels on the GPU through wgpu in Compute Access
type: feature
status: backlog
priority: medium
labels: [gpu, compute]
estimate: 8
layer: access
---

The GPU backend, behind the `gpu` feature and `--gpu auto|on|off`.

- [ ] Device selection, adapter limits, buffers, fallback to the CPU on any failure
- [ ] WGSL versions of the kernels with 32-bit packed atomics; 64-bit where available
- [ ] Every kernel tested against its CPU twin for identical results

**Depends on:** [[TRX-25]], [[TRX-16]]
