# Mermaid Journeys

DocCrate renders Mermaid `journey` blocks natively. They are useful for
software workflow journals, onboarding notes, release days, and incident
response retrospectives where the important signal is the experience of each
step.

```mermaid
journey
title Developer First PR Journal
section Setup
Clone the repo: 5: Developer
Install Rust toolchain: 4: Developer
First release build: 3: Developer
section Change
Find renderer path: 3: Developer
Add journey support: 4: Developer
Capture testsnap: 5: Developer, Reviewer
section Review
Inspect PNG output: 5: Developer, Reviewer
Tune label spacing: 4: Developer
Land the change: 5: Developer
```

An incident response journey:

```mermaid
journey
title Incident Response Journal
section Detect
Alert fires: 3: On-call
Confirm customer impact: 2: On-call, Support
section Mitigate
Find suspect release: 3: On-call, Developer
Rollback service: 4: Developer, SRE
Validate recovery: 5: SRE, Support
section Learn
Write follow-up notes: 4: On-call
Add regression coverage: 5: Developer
```

## Manual Journey Layout

Manual comments can place journey cards, section headers, and the canvas when
the journal needs a presentation-quality shape. Use `@node` for task cards,
`@group` for section headers, and `@graph` for the canvas.

```mermaid
journey
title Incident Response Journal
section Detect
Alert fires: 3: On-call
section Mitigate
Rollback service: 4: Developer, SRE
section Learn
Write follow-up: 4: On-call

%% @group Detect x=72 y=48 w=146 h=38
%% @group Mitigate x=232 y=48 w=146 h=38
%% @group Learn x=392 y=48 w=146 h=38
%% @node "Alert fires" x=86 y=245 w=154 h=106
%% @node "Rollback service" x=246 y=245 w=166 h=108
%% @node "Write follow-up" x=416 y=245 w=154 h=106
%% @graph w=620 h=420
```
