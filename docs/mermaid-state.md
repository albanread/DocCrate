# State Diagrams

DocCrate renders mermaid `stateDiagram-v2` blocks through the same renderer
as flowcharts. Selkie maps each state type to the appropriate shape (Start
→ filled circle, End → double circle, Choice → diamond, Fork/Join →
horizontal bar, Default → rounded rect), so everything you've seen in the
flowchart catalog applies here too.

## A simple lifecycle

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Loading : fetch()
    Loading --> Success : 200
    Loading --> Error : 4xx / 5xx
    Success --> Idle : reset
    Error --> Idle : retry
    Error --> [*] : give up
```

## Choice + fork / join

```mermaid
stateDiagram-v2
    [*] --> Validating
    Validating --> Decide
    state Decide <<choice>>
    Decide --> Accepted : valid
    Decide --> Rejected : invalid

    Accepted --> Fork
    state Fork <<fork>>
    Fork --> SendEmail
    Fork --> WriteLog

    SendEmail --> Join
    WriteLog --> Join
    state Join <<join>>
    Join --> [*]

    Rejected --> [*]
```

## Composite states

A `state Name { ... }` block becomes a group with nested children — the
renderer paints the bounding box first and the children on top.

```mermaid
stateDiagram-v2
    [*] --> Active

    state Active {
        [*] --> NumLockOff
        NumLockOff --> NumLockOn : EvNumLockPressed
        NumLockOn --> NumLockOff : EvNumLockPressed
    }

    Active --> [*]
```

State diagrams don't yet honour `@annotation` overrides (selkie's
annotation database is flowchart-only at the moment), so they render with
DocCrate's theme defaults.
