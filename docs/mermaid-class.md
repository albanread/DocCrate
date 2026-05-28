# Mermaid Class Diagrams

DocCrate renders Mermaid `classDiagram` blocks natively. They are useful for
API contracts, trait/interface sketches, domain models, and implementation
relationships that should stay close to the documentation.

```mermaid
classDiagram
    direction TB

    class DocumentStore {
        <<interface>>
        +load(path) LoadedDoc
        +scan(root) Vec~DocFile~
    }

    class MarkdownParser {
        -cache HashMap
        +parse(doc) Vec~Block~
        +parse_mermaid(source) Graph
    }

    class Renderer {
        +paint()
        +relayout()
        +save_screen_png()
    }

    class RopeBuffer {
        +from_string(text)
        +to_string() String
    }

    DocumentStore <|.. MarkdownParser : consumes
    MarkdownParser --> Renderer : layout
    RopeBuffer o-- DocumentStore : large files
```

## Manual Class Layout

Manual comments can place classes, namespace groups, attached notes, and
relationship routes when the automatic layout needs author control. Attached
notes can be targeted as `note:ClassName`.

```mermaid
classDiagram
    direction LR

    namespace Core {
      class DocumentStore {
        <<interface>>
        +scan(root) Vec~DocFile~
        +load(path) LoadedDoc
      }

      class MarkdownParser {
        +parse(doc) Vec~Block~
        +parse_mermaid(source) Graph
      }

      class RopeBuffer {
        +line(index) str
        +slice(range) str
      }
    }

    namespace Render {
      class LayoutEngine {
        +layout(blocks) Layout
        +hit_test(point) HitRegion
      }

      class Direct2DPainter {
        +draw(cmds)
        +save_png(path)
      }
    }

    DocumentStore <|.. MarkdownParser : consumes
    RopeBuffer o-- DocumentStore : large files
    MarkdownParser --> LayoutEngine : blocks
    LayoutEngine --> Direct2DPainter : draw list
    note for MarkdownParser "Hot path: avoid extra copies"
    %% @node DocumentStore x=70 y=85 w=210 h=150
    %% @node MarkdownParser x=310 y=85 w=210 h=150
    %% @node RopeBuffer x=70 y=285 w=210 h=120
    %% @node LayoutEngine x=620 y=95 w=190 h=145
    %% @node Direct2DPainter x=620 y=305 w=190 h=130
    %% @node note:MarkdownParser x=310 y=285 w=210 h=70
    %% @group Core x=35 y=40 w=520 h=430
    %% @group Render x=590 y=40 w=250 h=430
    %% @edge DocumentStore->MarkdownParser points="280,160 310,160" label_offset="0,-15"
    %% @edge RopeBuffer->DocumentStore points="175,285 175,235" label_pos="230,260"
    %% @edge MarkdownParser->LayoutEngine points="520,160 570,160 570,168 620,168" label_offset="0,-14"
    %% @edge LayoutEngine->Direct2DPainter points="715,240 715,305" label_offset="34,0"
    %% @graph w=880 h=510
```

Cardinality and relation markers are supported too:

```mermaid
classDiagram
    class Repository
    class Commit
    class Branch
    class PullRequest

    Repository "1" o-- "*" Branch : contains
    Branch "1" *-- "*" Commit : history
    PullRequest ..> Branch : targets
    PullRequest --> Commit : compares
```
