---
name: new-adr
description: Scaffold a new Architecture Decision Record with the correct sequential number and standard template. Usage: /new-adr "title of the decision"
---

1. Find the highest existing ADR number in `docs/adr/` by listing the files.
2. Increment it by 1 and zero-pad to 4 digits (e.g. 0015).
3. Convert the title argument to kebab-case for the filename.
4. Create `docs/adr/ADR-{NNNN}-{kebab-case-title}.md` with this exact template:

```
<!-- SPDX-License-Identifier: MPL-2.0 -->

# ADR-{NNNN} — {Title}

**Date:** {today's date as YYYY-MM-DD}
**Status:** Proposed

## Context

[Why this decision is needed — what problem or constraint drives it]

## Decision

[What was decided, stated clearly and concisely]

## Alternatives considered

**[Alternative 1 name]**
[Description and reason it was not chosen]

**[Alternative 2 name]**
[Description and reason it was not chosen]

## Consequences

**Positive:**
-

**Negative / risks:**
-

## Notes

[Optional additional context, links, or follow-up items]
```

5. Add the new ADR to the ADR Index table in `docs/ARCHITECTURE.md` §13 (columns: ADR link, Decision summary, Status).
6. Report the created filename and the line added to the index.
