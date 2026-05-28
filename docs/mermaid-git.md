# Mermaid Git Graphs

DocCrate renders Mermaid `gitGraph` blocks natively. They are useful for
release strategy, branching policy, hotfix flow, and migration history.

```mermaid
gitGraph
    commit id:"base"
    branch feature
    checkout feature
    commit id:"parser"
    commit id:"layout"
    checkout main
    commit id:"docs"
    merge feature id:"merge" tag:"v1.4"
    commit id:"release" tag:"stable"
```

A hotfix flow with a cherry-pick:

```mermaid
gitGraph
    commit id:"v1.3"
    branch hotfix
    checkout hotfix
    commit id:"fix" tag:"urgent"
    checkout main
    commit id:"mainline"
    cherry-pick id:"fix"
    merge hotfix id:"ship" tag:"v1.3.1"
```
