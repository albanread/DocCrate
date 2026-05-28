# Mermaid Gantt Charts

DocCrate renders Mermaid `gantt` blocks natively. They are useful for release
plans, migration windows, incident remediation, and dependency-heavy workflow
docs.

```mermaid
gantt
title DocCrate Mermaid Rendering Plan
dateFormat YYYY-MM-DD
section Parser
Gantt grammar wired :done, p1, 2026-05-25, 2d
Native IR :done, p2, after p1, 2d
section Renderer
Bars and axis :active, r1, after p2, 3d
Snapshot review :crit, r2, after r1, 2d
section Release
Ship Gantt support :milestone, ship, after r2, 0d
```

A longer migration view with multiple sections:

```mermaid
gantt
title Markdown Platform Migration
dateFormat YYYY-MM-DD
section Discovery
Inventory docs :done, inv, 2026-06-01, 3d
Map ownership :done, own, after inv, 2d
section Implementation
Rope buffer rollout :active, rope, after own, 5d
Mermaid coverage :crit, mer, after rope, 6d
Search tuning :search, after rope, 4d
section Validation
Large-file test pass :test, after mer, 3d
Release candidate :milestone, rc, after test, 0d
```
