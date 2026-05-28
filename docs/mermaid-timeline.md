# Mermaid Timelines

DocCrate renders Mermaid `timeline` blocks natively. They are useful for
incident reports, release history, architecture evolution, and decision logs.

```mermaid
timeline
title Production Incident Timeline
section Detection
09h14 Alert fired : API error budget burn exceeded
09h18 Triage : On-call paged : Impact confirmed
section Mitigation
09h27 Rollback started : Previous release selected
09h36 Recovery : Error rate returned to baseline
section Follow-up
10h10 Review opened : Add regression test : Update runbook
```

A system evolution timeline:

```mermaid
timeline
title DocCrate Architecture Evolution
section Foundations
Initial viewer : Win32 window : Direct2D text rendering
Markdown parser : pulldown-cmark blocks
section Scale
Large files : Rope buffer added : Heading search kept fast
Mermaid renderer : Flowcharts : Sequence diagrams : C4 and Gantt
section Automation
Test snapshots : screen.png capture : Scroll-to-line review
```

## Manual Timeline Layout

Manual comments can place timeline tasks, event boxes, section bands, and the
canvas when an incident report or release log needs stable spacing. Use
`@node` for task and event boxes, `@group` for sections, and `@graph` for the
canvas.

```mermaid
timeline
title Release Readiness Timeline
section Design
API freeze : ADR accepted : Review complete
section Build
Release candidate : Smoke test : Security scan
section Launch
Publish notes : Rollout starts : Support handoff

%% @group Design x=58 y=50 w=222 h=44
%% @group Build x=308 y=50 w=222 h=44
%% @group Launch x=558 y=50 w=222 h=44
%% @node "API freeze" x=82 y=128 w=170 h=58
%% @node "ADR accepted" x=82 y=252 w=170 h=54
%% @node "Review complete" x=82 y=316 w=170 h=54
%% @node "Release candidate" x=332 y=128 w=178 h=58
%% @node "Smoke test" x=332 y=252 w=178 h=54
%% @node "Security scan" x=332 y=316 w=178 h=54
%% @node "Publish notes" x=582 y=128 w=170 h=58
%% @node "Rollout starts" x=582 y=252 w=170 h=54
%% @node "Support handoff" x=582 y=316 w=170 h=54
%% @graph w=820 h=430
```
