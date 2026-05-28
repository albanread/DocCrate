# Sequence Diagrams

Actors with lifelines, time-ordered messages, self-messages, and notes.
Activation boxes and fragments (loop / alt / opt / par) come in a follow-up.

A small request/response chain with a couple of return arrows:

```mermaid
sequenceDiagram
    Client->>Gateway: GET /users/42
    Gateway->>UserService: fetch(42)
    UserService->>Cache: lookup(42)
    Cache-->>UserService: miss
    UserService->>DB: SELECT * FROM users WHERE id=42
    DB-->>UserService: row
    UserService-->>Gateway: User{42}
    Gateway-->>Client: 200 OK + JSON
```

Open arrowheads (async / fire-and-forget — use `-)` not `->)`) and a
self-message:

```mermaid
sequenceDiagram
    Producer->>Queue: enqueue(msg)
    Queue-)Worker: deliver(msg)
    Worker->>Worker: process(msg)
    Worker-->>Queue: ack
```

Notes can sit beside one participant or span two participants:

```mermaid
sequenceDiagram
    Client->>Gateway: GET /docs/index.md
    Note right of Gateway: Resolve relative paths before loading
    Gateway->>Renderer: render(markdown)
    Note over Gateway,Renderer: synchronous layout pass
    Renderer-->>Client: native frame
```

## Manual Sequence Layout

Manual comments can place participant boxes, notes, and the canvas when a
sequence diagram needs a stable runbook shape. Notes can be targeted by their
zero-based order as `note:0`, `note:1`, and so on.

```mermaid
sequenceDiagram
    participant Reader
    participant Search
    participant Parser
    participant Renderer
    Reader->>Search: find("rope")
    Note right of Search: heading-only index keeps search fast
    Search->>Parser: load selected doc
    Note over Parser,Renderer: large docs use RopeBuffer
    Parser->>Renderer: blocks
    Renderer-->>Reader: highlighted page
    %% @node Reader x=40 y=18 w=110 h=38
    %% @node Search x=230 y=18 w=120 h=38
    %% @node Parser x=430 y=18 w=120 h=38
    %% @node Renderer x=640 y=18 w=130 h=38
    %% @note note:0 x=250 y=86 w=220 h=48
    %% @note note:1 x=416 y=166 w=280 h=48
    %% @graph w=820 h=290
```

Cross-mark for destroyed / lost messages (`-x` solid, `--x` dotted):

```mermaid
sequenceDiagram
    Client->>Server: connect
    Server-xClient: disconnect (forced)
```

A short ping/pong:

```mermaid
sequenceDiagram
    A->>B: ping
    B-->>A: pong
```

Bidirectional (arrowheads on both ends — `<<->>` solid, `<<-->>` dotted):

```mermaid
sequenceDiagram
    Alice<<->>Bob: handshake
    Bob<<-->>Carol: negotiation
```

## Arrow Reference

The full set of arrow tokens supported by the sequence parser. Anything else
will surface a parse error rather than render incorrect geometry.

| Token  | Line   | Arrowhead       | Use                              |
|:------:|:------:|:----------------|:---------------------------------|
| `->`   | solid  | none            | bare line, no head               |
| `-->`  | dotted | none            | bare dotted line                 |
| `->>`  | solid  | filled triangle | synchronous request              |
| `-->>` | dotted | filled triangle | response / return value          |
| `-x`   | solid  | cross           | destroyed / lost message         |
| `--x`  | dotted | cross           | dotted lost message              |
| `-)`   | solid  | open `<`        | asynchronous fire-and-forget     |
| `--)`  | dotted | open `<`        | dotted async                     |
| `<<->>`  | solid  | filled, both ends | bidirectional sync             |
| `<<-->>` | dotted | filled, both ends | bidirectional dotted           |

Any token may be suffixed with `+` or `-` to activate/deactivate the target
actor as a single operation (e.g. `Client->>+Server: req`).

If a diagram fails to parse you'll see an italic `mermaid error: ...` line
followed by the source as a code block.
