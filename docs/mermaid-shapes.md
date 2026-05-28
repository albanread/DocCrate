# Mermaid Shapes

Every shape the renderer now supports, in order. Connected to a `Hub` node so
selkie actually keeps them in the layout — orphan nodes get dropped.

```mermaid
flowchart LR
    Hub((Hub)) --> Rect[Rectangle]
    Hub --> RR(Rounded)
    Hub --> Stad([Stadium])
    Hub --> Circ((Circle))
    Hub --> DCirc(((DoubleCircle)))
    Hub --> Diam{Decision}
    Hub --> Hex{{Hexagon}}
    Hub --> Cyl[(Database)]
    Hub --> Sub[[Subroutine]]
    Hub --> Trap[/Trapezoid\]
    Hub --> ITrap[\InvTrap/]
    Hub --> LR[/LeanRight/]
    Hub --> LL[\LeanLeft\]
    Hub --> Odd>Odd flag]
```

Cylinders with annotations — overriding fill and stroke per-node:

```mermaid
flowchart TD
    App[Application] --> Cache[(Redis Cache)]
    App --> DB[(Primary DB)]
    DB --> Replica[(Read Replica)]
%% @node Cache  fill="#3E1F47" stroke="#C586C0"
%% @node DB     fill="#264F78" stroke="#9CDCFE"
%% @node Replica fill="#1F3A1F" stroke="#608B4E"
```

If any shape doesn't render or looks wrong, tell me which name and what you
see.
