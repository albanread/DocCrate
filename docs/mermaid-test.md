# Mermaid Test

Simple flowchart to verify the rendering pipeline.

```mermaid
flowchart TD
    A[Start] --> B{Decision}
    B -->|Yes| C[Do the thing]
    B -->|No| D[Skip it]
    C --> E[End]
    D --> E
```

Same diagram with a couple of annotations to exercise the override path:

```mermaid
flowchart LR
    A[Start] --> B[Middle] --> C[End]
%% @node A fill="#264F78" stroke="#9CDCFE"
%% @node C fill="#3E1F47" stroke="#C586C0"
%% @edge A->B line_color="#4EC9B0" line_style="dash"
```

A diagram inside a subgraph:

```mermaid
flowchart TD
    A --> B
    subgraph cluster["Pipeline"]
        B --> C
        C --> D
    end
    D --> E
```

If the build is bad you'll see an italic error line and the source as a code block instead.
