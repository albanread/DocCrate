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

## Manual State Layout

Manual comments can place states, composite groups, relationship routes, edge
labels, and the canvas when a workflow needs a stable runbook shape. Use
`@node` for states, `@group` for composite states, `@edge` for transition
routes and labels, and `@graph` for the canvas.

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Validating : open
    Validating --> Rendering : ok
    Validating --> Error : failed
    Rendering --> Idle : close
    Error --> Idle : retry

    %% @node Idle x=80 y=105 w=120 h=56
    %% @node Validating x=285 y=105 w=140 h=56
    %% @node Rendering x=510 y=62 w=140 h=56
    %% @node Error x=510 y=168 w=120 h=56
    %% @edge Idle->Validating points="200,133 285,133" label_offset="0,-12"
    %% @edge Validating->Rendering points="425,133 468,133 468,90 510,90" label_offset="0,-12"
    %% @edge Validating->Error points="425,133 468,133 468,196 510,196" label_pos="470,166"
    %% @edge Rendering->Idle points="580,118 580,260 140,260 140,161" label_pos="360,260"
    %% @edge Error->Idle points="510,196 240,196 240,133 200,133" label_offset="0,14"
    %% @graph w=700 h=320
```
