# Visual Review Outputs

This folder is for local SVG and PNG render artifacts generated from Mermaid
fixtures while reviewing Selkie rendering behavior.

The image outputs are intentionally ignored by git so we can regenerate them
freely during development.

Suggested commands:

```powershell
cargo run --bin selkie -- tests/render_fixtures/annotated_visual_review.mmd -o visuals/annotated_visual_review.svg
cargo run --bin selkie -- tests/render_fixtures/annotated_visual_review.mmd -o visuals/annotated_visual_review.png
```

The main review fixture for annotation-aware flowchart rendering is:

- `tests/render_fixtures/annotated_visual_review.mmd`
