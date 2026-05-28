# Mermaid ER Diagrams

DocCrate renders Mermaid `erDiagram` blocks natively. They are useful for
data models, event schemas, persistence boundaries, and domain documentation.

```mermaid
erDiagram
    DOC_FILE ||--o{ HEADING : contains
    DOC_FILE ||--o{ LINK : references
    DOC_FILE ||--o{ MERMAID_BLOCK : embeds
    HEADING ||--o{ SEARCH_HIT : indexes

    DOC_FILE {
        string path PK
        string title
        int byte_count
    }

    HEADING {
        string id PK
        string doc_path FK
        int source_line
        string text
    }

    LINK {
        string id PK
        string doc_path FK
        string href
    }

    MERMAID_BLOCK {
        string id PK
        string doc_path FK
        string diagram_type
    }

    SEARCH_HIT {
        string heading_id FK
        string query
    }
```

## Manual ER Layout

Manual comments can place entities and route relationships when an ER diagram
needs a stable documentation-friendly shape. Use `@node` for entities,
`@edge` for relationship paths and labels, and `@graph` for the canvas.

```mermaid
erDiagram
    WORKSPACE ||--o{ DOCUMENT : owns
    DOCUMENT ||--o{ SNAPSHOT : captures
    DOCUMENT ||--o{ SEARCH_HIT : indexes
    SNAPSHOT ||--|| PNG_FILE : writes
    DOCUMENT }o--o{ TAG : tagged

    WORKSPACE {
        string root PK
        string branch
    }

    DOCUMENT {
        string path PK
        string title
        int byte_count
    }

    SNAPSHOT {
        string id PK
        string document_path FK
        int scroll_y
    }

    PNG_FILE {
        string path PK
        int width
        int height
    }

    SEARCH_HIT {
        string document_path FK
        int source_line
        string query
    }

    TAG {
        string name PK
    }

    %% @node WORKSPACE x=48 y=72 w=190 h=140
    %% @node DOCUMENT x=330 y=72 w=220 h=180
    %% @node SNAPSHOT x=646 y=72 w=220 h=180
    %% @node PNG_FILE x=646 y=312 w=220 h=180
    %% @node SEARCH_HIT x=330 y=322 w=220 h=180
    %% @node TAG x=48 y=342 w=220 h=100
    %% @edge WORKSPACE->DOCUMENT points="238,142 330,142" label_offset="0,-14"
    %% @edge DOCUMENT->SNAPSHOT points="550,162 646,162" label_offset="0,-14"
    %% @edge SNAPSHOT->PNG_FILE points="756,252 756,312" label_offset="38,0"
    %% @edge DOCUMENT->SEARCH_HIT points="440,252 440,322" label_offset="42,0"
    %% @edge DOCUMENT->TAG points="330,192 285,192 285,392 268,392" label_pos="302,292"
    %% @graph w=920 h=540
```

Cardinality and optional relationships are drawn with crow's foot markers:

```mermaid
erDiagram
    WORKSPACE ||--o{ DOCUMENT : owns
    DOCUMENT ||--o{ SNAPSHOT : captures
    DOCUMENT |o--o{ TAG : may_have
    SNAPSHOT ||--|| PNG_FILE : writes

    WORKSPACE {
        string root PK
        string branch
    }

    DOCUMENT {
        string path PK
        string workspace_root FK
    }

    SNAPSHOT {
        string id PK
        string document_path FK
        int scroll_y
    }

    PNG_FILE {
        string path PK
        int width
        int height
    }

    TAG {
        string name PK
    }
```
