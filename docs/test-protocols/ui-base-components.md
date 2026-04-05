# Test Protocol — UI Base Components

> **Version:** 1.0.0
> **Date:** 2026-04-05
> **Status:** Draft
> **Scope:** Frontend base components — Button (4 variants), TextInput, Toggle, Dropdown, Tooltip, Dialog/Modal
> **Input documents:** FS.md §3.14 (FS-A11Y), UXD.md §7.9/7.10/7.14/7.15/7.16 §11.3/11.5, app.css (design tokens)
> **Compiled by:** test-engineer

---

## 1. Purpose & Scope

This protocol covers test scenarios for TauTerm's reusable UI base components. These components underpin all interactive surfaces of the application (preferences panel, SSH connection manager, confirmation dialogs, tooltips). Their correctness, accessibility compliance, and token fidelity are prerequisites for all higher-level UI test protocols.

**Components in scope:**
- `Button` — primary, secondary, ghost, destructive variants
- `TextInput` — text field with label, placeholder, helper text, error state
- `Toggle` — binary switch with checked/unchecked/disabled states
- `Dropdown` — select field with option list
- `Tooltip` — hover/focus-triggered informational overlay
- `Dialog` / `Modal` — confirmation and informational overlay with backdrop

**Out of scope:**
- Terminal rendering components (covered by `ui-terminal-components.md`)
- Keyboard shortcut recorder (covered by functional PTY/VT/SSH protocol)
- Theme editor form fields (depend on base components but have dedicated scenarios)

**Test framework:** Vitest + `@testing-library/svelte` for unit/component tests. WebdriverIO for E2E visual verification.

**Design token references:** All token values are sourced from `src/app.css` `@theme` block. The expected resolved values are listed for precision; implementations must use the token variables, not hardcoded values.

---

## 2. Functional Scenarios — Button (UIBC-FN-BTN)

> UXD §7.14. All button variants share: `--radius-sm` (2px), `--font-size-ui-base` (13px), `--font-weight-medium` (500), height `--size-target-min` (44px), horizontal padding `--space-4` (16px).

| ID | Scenario | Steps | Expected Result | FS Ref | Priority |
|----|----------|-------|-----------------|--------|----------|
| UIBC-FN-BTN-001 | Primary button renders with correct default styles | Render `<Button variant="primary">Save</Button>` | Background `var(--color-accent)` (#4a92bf), text `var(--color-text-inverted)` (#0e0d0b), no border, height 44px, padding 16px horizontal, border-radius 2px | FS-A11Y-002 | P1 |
| UIBC-FN-BTN-002 | Primary button hover state applies darker background | Render primary button, simulate `mouseenter` | Background transitions to `var(--umbra-blue-500)` (#2e6f9c) | UXD §7.14 | P1 |
| UIBC-FN-BTN-003 | Primary button active/pressed state | Simulate `mousedown` on primary button | Background `var(--umbra-blue-600)` (#1e4d6e) | UXD §7.14 | P1 |
| UIBC-FN-BTN-004 | Primary button disabled state | Render `<Button variant="primary" disabled>Save</Button>` | Background `var(--umbra-neutral-700)` (#35312a), text `var(--color-text-tertiary)` (#6b6660), `pointer-events: none` or `cursor: not-allowed`, `aria-disabled="true"` | FS-A11Y-002 | P1 |
| UIBC-FN-BTN-005 | Secondary button renders with transparent background and accent border | Render `<Button variant="secondary">Cancel</Button>` | Background transparent, text `var(--color-accent-text)` (#7ab3d3), border 1px solid `var(--color-accent)` (#4a92bf) | UXD §7.14 | P1 |
| UIBC-FN-BTN-006 | Secondary button disabled state | Render `<Button variant="secondary" disabled>Cancel</Button>` | Background transparent, text `var(--color-text-tertiary)`, border 1px solid `var(--umbra-neutral-700)` | UXD §7.14 | P1 |
| UIBC-FN-BTN-007 | Ghost button renders with no background and no border | Render `<Button variant="ghost">Close</Button>` | Background transparent, text `var(--color-text-primary)` (#ccc7bc), no border visible | UXD §7.14 | P1 |
| UIBC-FN-BTN-008 | Ghost button hover state shows subtle background | Simulate `mouseenter` on ghost button | Background `var(--color-hover-bg)` (#2c2921) | UXD §7.14 | P2 |
| UIBC-FN-BTN-009 | Ghost button active state shows stronger background | Simulate `mousedown` on ghost button | Background `var(--color-active-bg)` (#35312a) | UXD §7.14 | P2 |
| UIBC-FN-BTN-010 | Destructive button renders with error background | Render `<Button variant="destructive">Delete</Button>` | Background `var(--color-error)` (#c44444), text `var(--umbra-neutral-100)` (#f5f2ea), no border | UXD §7.14 | P1 |
| UIBC-FN-BTN-011 | Destructive button hover state | Simulate `mouseenter` on destructive button | Background `var(--umbra-red-500)` (#9c2c2c) | UXD §7.14 | P1 |
| UIBC-FN-BTN-012 | Destructive button disabled state | Render `<Button variant="destructive" disabled>Delete</Button>` | Background `var(--umbra-neutral-700)`, text `var(--color-text-tertiary)` — same as primary disabled | UXD §7.14 | P1 |
| UIBC-FN-BTN-013 | Button with icon renders icon at correct size with gap | Render `<Button variant="primary"><Icon /> Save</Button>` | Icon size 14px (`--size-icon-sm`), gap 4px (`--space-1`) between icon and label text | UXD §7.14 | P2 |
| UIBC-FN-BTN-014 | Click handler fires on enabled button | Render button with `onClick` handler, simulate click | Handler called exactly once | — | P1 |
| UIBC-FN-BTN-015 | Click handler does not fire on disabled button | Render `disabled` button with `onClick`, simulate click | Handler not called | FS-A11Y-003 | P1 |
| UIBC-FN-BTN-016 | Button dispatches click on Enter key | Focus button, press `Enter` | Handler called once | FS-A11Y-003 | P1 |
| UIBC-FN-BTN-017 | Button dispatches click on Space key | Focus button, press `Space` | Handler called once | FS-A11Y-003 | P1 |

---

## 3. Functional Scenarios — TextInput (UIBC-FN-INP)

> UXD §7.15. Height `--size-target-min` (44px), background `--term-bg` (#16140f), border 1px solid `--color-border`.

| ID | Scenario | Steps | Expected Result | FS Ref | Priority |
|----|----------|-------|-----------------|--------|----------|
| UIBC-FN-INP-001 | TextInput renders label, field, and helper text | Render `<TextInput label="Host" helper="Enter hostname or IP" />` | Label rendered above field in 12px `--color-text-secondary`; field visible; helper text rendered below in 12px `--color-text-secondary` | UXD §7.15.1 | P1 |
| UIBC-FN-INP-002 | Placeholder renders when field is empty | Render `<TextInput placeholder="e.g. 192.168.1.1" />` | Placeholder text visible in `--color-text-tertiary` (#6b6660) | UXD §7.15.1 | P1 |
| UIBC-FN-INP-003 | Placeholder disappears when value is present | Render with `value="myhost"` or type into field | Placeholder no longer visible; typed value visible in `--color-text-primary` (#ccc7bc) | UXD §7.15.1 | P1 |
| UIBC-FN-INP-004 | Default border color | Render field in default state (no focus, no error) | Border 1px solid `var(--color-border)` (#35312a) | UXD §7.15.1 | P1 |
| UIBC-FN-INP-005 | Hover state changes border color | Simulate `mouseenter` on field | Border changes to 1px solid `var(--umbra-neutral-600)` (#4a4640) | UXD §7.15.1 | P2 |
| UIBC-FN-INP-006 | Focus state applies focus ring | Click or tab into field | Border becomes 2px solid `var(--color-focus-ring)` (#4a92bf), focus ring replaces border | UXD §7.15.1, FS-A11Y-003 | P1 |
| UIBC-FN-INP-007 | Error state applies error border and error text | Render `<TextInput error="Required field" />` | Border 1px solid `var(--color-error)` (#c44444); error text rendered below field in 12px `var(--color-error-text)` (#d97878) | UXD §7.15.1 | P1 |
| UIBC-FN-INP-008 | Disabled state prevents interaction and applies muted styles | Render `<TextInput disabled value="locked" />` | Background `var(--color-bg-surface)` (#242118), text `var(--color-text-tertiary)`, border `var(--color-border-subtle)`, `cursor: not-allowed`, not focusable via Tab | UXD §7.15.1, FS-A11Y-003 | P1 |
| UIBC-FN-INP-009 | Input value binding updates on user input | Render with reactive binding, type "hello" | Bound value equals "hello" after typing | — | P1 |
| UIBC-FN-INP-010 | Field height meets minimum touch target | Inspect computed height | Height is 44px (`--size-target-min`) | FS-A11Y-002 | P1 |

---

## 4. Functional Scenarios — Toggle (UIBC-FN-TOG)

> UXD §7.16. Track 36×20px, thumb 16px diameter. Hit area 44×44px.

| ID | Scenario | Steps | Expected Result | FS Ref | Priority |
|----|----------|-------|-----------------|--------|----------|
| UIBC-FN-TOG-001 | Unchecked state renders with neutral track and thumb | Render `<Toggle checked={false} />` | Track `var(--umbra-neutral-700)` (#35312a), thumb `var(--umbra-neutral-400)` (#9c9890) | UXD §7.16 | P1 |
| UIBC-FN-TOG-002 | Checked state renders with accent track and light thumb | Render `<Toggle checked={true} />` | Track `var(--color-accent)` (#4a92bf), thumb `var(--umbra-neutral-100)` (#f5f2ea) | UXD §7.16 | P1 |
| UIBC-FN-TOG-003 | Clicking toggle switches from unchecked to checked | Render unchecked toggle, click | State changes to checked; track and thumb colors update accordingly | UXD §7.16 | P1 |
| UIBC-FN-TOG-004 | Clicking toggle switches from checked to unchecked | Render checked toggle, click | State changes to unchecked | UXD §7.16 | P1 |
| UIBC-FN-TOG-005 | Thumb slides with CSS transition on state change | Toggle from unchecked to checked | Thumb translates 16px rightward over `--duration-base` (100ms) with `--ease-out` | UXD §7.16 | P2 |
| UIBC-FN-TOG-006 | Hover unchecked state changes track and thumb | Simulate `mouseenter` on unchecked toggle | Track `var(--umbra-neutral-600)`, thumb `var(--umbra-neutral-300)` | UXD §7.16 | P2 |
| UIBC-FN-TOG-007 | Hover checked state darkens track | Simulate `mouseenter` on checked toggle | Track `var(--umbra-blue-500)` | UXD §7.16 | P2 |
| UIBC-FN-TOG-008 | Disabled unchecked state applies muted colors | Render `<Toggle checked={false} disabled />` | Track `var(--umbra-neutral-750)`, thumb `var(--umbra-neutral-600)`, not clickable | UXD §7.16 | P1 |
| UIBC-FN-TOG-009 | Disabled checked state applies muted accent colors | Render `<Toggle checked={true} disabled />` | Track `var(--umbra-blue-700)`, thumb `var(--umbra-neutral-500)`, not clickable | UXD §7.16 | P1 |
| UIBC-FN-TOG-010 | onChange callback fires with new value on click | Render with `onChange` handler, click | Handler called with `true` when toggling on, `false` when toggling off | — | P1 |
| UIBC-FN-TOG-011 | onChange does not fire when disabled | Click disabled toggle | Handler not called | FS-A11Y-003 | P1 |

---

## 5. Functional Scenarios — Dropdown (UIBC-FN-DRP)

> UXD §7.16. Closed state identical to TextInput. Open state: menu below trigger, max-height 240px, `--z-dropdown` (30).

| ID | Scenario | Steps | Expected Result | FS Ref | Priority |
|----|----------|-------|-----------------|--------|----------|
| UIBC-FN-DRP-001 | Closed state renders as text input with ChevronDown icon | Render `<Dropdown options={[...]} />` | Field looks identical to TextInput, `ChevronDown` icon right-aligned at `--size-icon-sm` (14px) in `--color-icon-default` (#9c9890) | UXD §7.16 | P1 |
| UIBC-FN-DRP-002 | Clicking trigger opens the option list | Render closed dropdown, click trigger | Option list panel appears below trigger, z-index `--z-dropdown` (30) | UXD §7.16 | P1 |
| UIBC-FN-DRP-003 | Open state menu has correct background and border | After clicking trigger | Menu background `var(--color-bg-raised)` (#2c2921), border 1px solid `var(--color-border)`, `--radius-md` (4px), `--shadow-raised` | UXD §7.16 | P1 |
| UIBC-FN-DRP-004 | Option list is scrollable when more than max-height | Render dropdown with enough options to exceed 240px | Option list has `overflow-y: auto` (or scroll) and does not exceed 240px visible height | UXD §7.16 | P2 |
| UIBC-FN-DRP-005 | Clicking an option selects it and closes the list | Open dropdown, click an option | Selected option value reflects in trigger field; option list closes | UXD §7.16 | P1 |
| UIBC-FN-DRP-006 | Selected option is visually highlighted | Open dropdown with a pre-selected value | Active/selected option has left border 2px solid `var(--color-accent)` and background `var(--color-accent-subtle)` (#1a3a52) | UXD §7.16 | P2 |
| UIBC-FN-DRP-007 | Clicking outside the open dropdown closes it | Open dropdown, click outside trigger and menu | Option list disappears | UXD §7.16 | P1 |
| UIBC-FN-DRP-008 | Escape key closes the open dropdown | Open dropdown, press `Escape` | Option list disappears, focus returns to trigger | UXD §7.16, FS-A11Y-003 | P1 |
| UIBC-FN-DRP-009 | Arrow Down key navigates to next option | Open dropdown, press `ArrowDown` | Next option receives visual focus (keyboard highlight) | FS-A11Y-003 | P1 |
| UIBC-FN-DRP-010 | Arrow Up key navigates to previous option | Press `ArrowUp` in open dropdown | Previous option receives visual focus | FS-A11Y-003 | P1 |
| UIBC-FN-DRP-011 | Enter key confirms the keyboard-focused option | Navigate to option with arrows, press `Enter` | Option selected, list closes | FS-A11Y-003 | P1 |
| UIBC-FN-DRP-012 | onChange callback fires with new selected value | Select a different option | Handler called with new option value | — | P1 |

---

## 6. Functional Scenarios — Tooltip (UIBC-FN-TIP)

> UXD §7.10. Delay 300ms, disappears immediately on mouse leave, z-index `--z-tooltip` (60), max-width 240px.

| ID | Scenario | Steps | Expected Result | FS Ref | Priority |
|----|----------|-------|-----------------|--------|----------|
| UIBC-FN-TIP-001 | Tooltip does not render on initial mount | Mount trigger element with tooltip | Tooltip panel absent from DOM | UXD §7.10 | P1 |
| UIBC-FN-TIP-002 | Tooltip appears after 300ms hover delay | Hover over trigger, wait 300ms (`--duration-slow`) | Tooltip panel visible; text matches provided label | UXD §7.10 | P1 |
| UIBC-FN-TIP-003 | Tooltip does not appear before 300ms | Hover over trigger, check at 150ms | Tooltip panel absent from DOM | UXD §7.10 | P2 |
| UIBC-FN-TIP-004 | Tooltip disappears immediately on mouse leave | Hover until visible, then `mouseleave` | Tooltip panel immediately absent (no delay) | UXD §7.10 | P1 |
| UIBC-FN-TIP-005 | Tooltip renders with correct visual tokens | Tooltip visible | Background `var(--color-bg-raised)` (#2c2921), border 1px solid `var(--color-border)`, `--radius-md` (4px), padding 4px/8px, font 11px `--color-text-primary`, `--shadow-raised`, z-index 60 | UXD §7.10 | P1 |
| UIBC-FN-TIP-006 | Tooltip text wraps at max-width 240px | Render tooltip with content longer than 240px | Text wraps within the 240px constraint; tooltip does not overflow | UXD §7.10 | P2 |
| UIBC-FN-TIP-007 | Tooltip appears below trigger by default | Render trigger with space below | Tooltip positioned below trigger, horizontally centered | UXD §7.10 | P2 |
| UIBC-FN-TIP-008 | Tooltip flips above when insufficient space below | Render trigger near viewport bottom edge | Tooltip repositions above trigger | UXD §7.10 | P2 |
| UIBC-FN-TIP-009 | Tooltip appears on keyboard focus | Tab to trigger element | Tooltip appears (same content as hover) | FS-A11Y-003, FS-A11Y-005 | P1 |
| UIBC-FN-TIP-010 | Tooltip disappears when trigger loses focus | Tab away from trigger | Tooltip disappears | FS-A11Y-003 | P1 |

---

## 7. Functional Scenarios — Dialog/Modal (UIBC-FN-DLG)

> UXD §7.9. Backdrop: `--color-bg-overlay` at 60% opacity, z-index 49. Panel: `--color-bg-raised`, z-index 50.

| ID | Scenario | Steps | Expected Result | FS Ref | Priority |
|----|----------|-------|-----------------|--------|----------|
| UIBC-FN-DLG-001 | Dialog does not render on initial mount | Mount page without opening dialog | Dialog panel and backdrop absent from DOM | UXD §7.9 | P1 |
| UIBC-FN-DLG-002 | Opening dialog renders backdrop and panel | Invoke open action | Backdrop visible at 60% opacity; panel visible centered on screen | UXD §7.9 | P1 |
| UIBC-FN-DLG-003 | Dialog panel renders with correct visual tokens | Dialog open | Background `var(--color-bg-raised)` (#2c2921), border 1px solid `var(--color-border)`, `--radius-md` (4px), `--shadow-overlay`, padding 24px (`--space-6`), z-index 50 | UXD §7.9.2 | P1 |
| UIBC-FN-DLG-004 | Small dialog width is 420px (max-width 90vw) | Open small dialog | Panel width 420px, constrained to 90vw on narrow viewports | UXD §7.9.2 | P2 |
| UIBC-FN-DLG-005 | Medium dialog (SSH key verify) width is 560px | Open medium dialog variant | Panel width 560px | UXD §7.9.2 | P2 |
| UIBC-FN-DLG-006 | Dialog heading uses correct typography | Dialog open with heading | Heading font-size 16px (`--font-size-ui-lg`), font-weight semibold (600), color `--color-text-primary` | UXD §7.9.2 | P1 |
| UIBC-FN-DLG-007 | Action buttons are right-aligned with 8px gap | Dialog open | Buttons in a flex row, right-aligned, 8px (`--space-2`) gap, primary action rightmost | UXD §7.9.2 | P2 |
| UIBC-FN-DLG-008 | Clicking backdrop does NOT close confirmation dialog | Dialog open, click backdrop | Dialog remains open (UXD explicit requirement: requires deliberate action) | UXD §7.9.1 | P1 |
| UIBC-FN-DLG-009 | Escape key closes dialog | Dialog open, press `Escape` | Dialog closes | UXD §11.2, FS-A11Y-003 | P1 |
| UIBC-FN-DLG-010 | Primary action button closes dialog and fires callback | Click primary action button | Handler called once; dialog closes | UXD §7.9.2 | P1 |
| UIBC-FN-DLG-011 | Secondary action (cancel) closes dialog without calling primary handler | Click secondary button | Dialog closes; primary handler not called | UXD §7.9.2 | P1 |
| UIBC-FN-DLG-012 | Destructive dialog: Cancel is default focused action | Open destructive confirmation dialog | Cancel/secondary button receives initial focus; it is the safe default | UXD §7.9.3 | P1 |
| UIBC-FN-DLG-013 | Destructive dialog heading and body text render correctly | Open destructive confirmation (e.g., close tab) | Heading "Close tab?", body "{N} process(es) still running. Closing will terminate them." | UXD §7.9.3 | P2 |
| UIBC-FN-DLG-014 | SSH key verification dialog: Reject is default focused action | Open SSH host key dialog | Reject button receives initial focus | UXD §7.9.4 | P1 |
| UIBC-FN-DLG-015 | SSH MITM warning dialog heading renders in error color | Open key-changed dialog variant | Heading text color `var(--color-error)` (#c44444); `ShieldAlert` icon present | UXD §7.9.4 | P1 |
| UIBC-FN-DLG-016 | Dialog appears with opacity transition | Open dialog | Panel transitions from opacity 0 to 1 over 100ms (`--duration-base`, `--ease-out`) | UXD §9 animation table | P2 |

---

## 8. Accessibility Scenarios (UIBC-A11Y)

> FS-A11Y-001 (WCAG 2.1 AA: 4.5:1 text, 3:1 UI), FS-A11Y-002 (44×44px targets), FS-A11Y-003 (keyboard navigation), FS-A11Y-004 (non-color indicators), FS-A11Y-005 (dual modality).

### 8.1 Color Contrast

| ID | Scenario | Component | Measurement | Expected Result | FS Ref | Priority |
|----|----------|-----------|-------------|-----------------|--------|----------|
| UIBC-A11Y-001 | Primary button text contrast | Button primary | `--color-text-inverted` (#0e0d0b) on `--color-accent` (#4a92bf) | Contrast ratio ≥ 4.5:1 (text) | FS-A11Y-001 | P1 |
| UIBC-A11Y-002 | Primary button disabled text contrast | Button primary disabled | `--color-text-tertiary` (#6b6660) on `--umbra-neutral-700` (#35312a) | Note: WCAG 1.4.3 exempts disabled controls — document ratio but no hard fail | FS-A11Y-001 | P2 |
| UIBC-A11Y-003 | Secondary button text contrast | Button secondary | `--color-accent-text` (#7ab3d3) on transparent over `--color-bg-base` (#0e0d0b) | Contrast ratio ≥ 4.5:1 | FS-A11Y-001 | P1 |
| UIBC-A11Y-004 | Ghost button text contrast | Button ghost | `--color-text-primary` (#ccc7bc) on transparent over `--color-bg-base` (#0e0d0b) | Contrast ratio ≥ 4.5:1 (calculated: ~8.4:1 — pass) | FS-A11Y-001 | P1 |
| UIBC-A11Y-005 | Destructive button text contrast | Button destructive | `--umbra-neutral-100` (#f5f2ea) on `--color-error` (#c44444) | Contrast ratio ≥ 4.5:1 | FS-A11Y-001 | P1 |
| UIBC-A11Y-006 | TextInput label contrast | TextInput | `--color-text-secondary` (#9c9890) on `--color-bg-base` (#0e0d0b) | Contrast ratio ≥ 4.5:1 | FS-A11Y-001 | P1 |
| UIBC-A11Y-007 | TextInput placeholder contrast | TextInput | `--color-text-tertiary` (#6b6660) on `--term-bg` (#16140f) | Note: WCAG exempts placeholder — document ratio | FS-A11Y-001 | P2 |
| UIBC-A11Y-008 | TextInput error text contrast | TextInput error | `--color-error-text` (#d97878) on `--color-bg-base` (#0e0d0b) | Contrast ratio ≥ 4.5:1 | FS-A11Y-001 | P1 |
| UIBC-A11Y-009 | Tooltip text contrast | Tooltip | `--color-text-primary` (#ccc7bc) on `--color-bg-raised` (#2c2921) | Contrast ratio ≥ 4.5:1 | FS-A11Y-001 | P1 |
| UIBC-A11Y-010 | Dialog heading contrast | Dialog | `--color-text-primary` (#ccc7bc) on `--color-bg-raised` (#2c2921) | Contrast ratio ≥ 4.5:1 | FS-A11Y-001 | P1 |
| UIBC-A11Y-011 | Dialog body text contrast | Dialog | `--color-text-primary` (#ccc7bc) on `--color-bg-raised` (#2c2921) | Contrast ratio ≥ 4.5:1 | FS-A11Y-001 | P1 |
| UIBC-A11Y-012 | Toggle focus ring contrast against backdrop | Toggle | `--color-focus-ring` (#4a92bf) on `--color-bg-base` (#0e0d0b) | Contrast ratio ≥ 3:1 (UI component) | FS-A11Y-001 | P1 |
| UIBC-A11Y-013 | Dropdown option text contrast | Dropdown | `--color-text-primary` on `--color-bg-raised` | Contrast ratio ≥ 4.5:1 | FS-A11Y-001 | P1 |

### 8.2 Keyboard Navigation

| ID | Scenario | Component | Steps | Expected Result | FS Ref | Priority |
|----|----------|-----------|-------|-----------------|--------|----------|
| UIBC-A11Y-020 | Tab reaches button | All buttons | Start before button, press `Tab` | Button receives visible focus ring (2px solid `--color-focus-ring`, offset 2px `--color-focus-ring-offset`) | FS-A11Y-003 | P1 |
| UIBC-A11Y-021 | Disabled button is skipped by Tab | Button (disabled) | Tab through a form with a disabled button | Disabled button not in tab order (not reachable via Tab) | FS-A11Y-003 | P1 |
| UIBC-A11Y-022 | Tab reaches TextInput | TextInput | Press `Tab` into input | Input receives focus; focus ring visible (2px `--color-focus-ring`) | FS-A11Y-003 | P1 |
| UIBC-A11Y-023 | Disabled TextInput is excluded from tab order | TextInput (disabled) | Tab through form | Disabled input skipped | FS-A11Y-003 | P1 |
| UIBC-A11Y-024 | Tab reaches Toggle | Toggle | Press `Tab` to toggle | Toggle receives focus; focus ring visible (+2px `--color-focus-ring`) | FS-A11Y-003 | P1 |
| UIBC-A11Y-025 | Space key activates Toggle | Toggle (focused) | Press `Space` | Toggle state changes | FS-A11Y-003 | P1 |
| UIBC-A11Y-026 | Tab reaches Dropdown trigger | Dropdown | Press `Tab` | Dropdown trigger focused; focus ring visible | FS-A11Y-003 | P1 |
| UIBC-A11Y-027 | Enter or Space opens Dropdown from keyboard | Dropdown (focused) | Press `Enter` or `Space` | Option list opens | FS-A11Y-003 | P1 |
| UIBC-A11Y-028 | Dialog focus trap — Tab does not exit dialog | Dialog (open) | Tab repeatedly from within dialog | Focus cycles among interactive elements inside dialog only; never leaves dialog | UXD §11.2, FS-A11Y-003 | P1 |
| UIBC-A11Y-029 | Dialog focus trap — Shift+Tab cycles backward | Dialog (open) | Shift+Tab from first focusable element | Focus wraps to last focusable element inside dialog | UXD §11.2 | P1 |
| UIBC-A11Y-030 | Dialog initial focus on safe action | Destructive dialog (open) | Dialog opens | Cancel/Reject (safe) button is focused, not the destructive action | UXD §7.9.3/7.9.4 | P1 |
| UIBC-A11Y-031 | Shift+Tab reverses tab order through form | Form with Button + TextInput + Toggle | Shift+Tab | Focus moves in reverse order | FS-A11Y-003 | P1 |
| UIBC-A11Y-032 | Focus visible on all interactive states | All components | Tab through each component | Every focused component shows a visible outline meeting WCAG 2.4.11 (non-zero size focus indicator) | FS-A11Y-003 | P1 |

### 8.3 ARIA Attributes

| ID | Scenario | Component | Check | Expected Result | FS Ref | Priority |
|----|----------|-----------|-------|-----------------|--------|----------|
| UIBC-A11Y-040 | Button has accessible role | All buttons | Inspect `role` attribute | `role="button"` present (or native `<button>` element) | FS-A11Y-003 | P1 |
| UIBC-A11Y-041 | Disabled button has aria-disabled | Button (disabled) | Inspect attributes | `aria-disabled="true"` present | FS-A11Y-003 | P1 |
| UIBC-A11Y-042 | TextInput has associated label | TextInput | Inspect `for`/`id` linkage or `aria-labelledby` | Input element is programmatically associated with its label text | FS-A11Y-003 | P1 |
| UIBC-A11Y-043 | TextInput in error state has aria-describedby | TextInput (error) | Inspect `aria-describedby` | Input's `aria-describedby` references the error message element ID | FS-A11Y-003 | P1 |
| UIBC-A11Y-044 | TextInput in error state has aria-invalid | TextInput (error) | Inspect `aria-invalid` | `aria-invalid="true"` present | FS-A11Y-003 | P1 |
| UIBC-A11Y-045 | Toggle has role="switch" and aria-checked | Toggle | Inspect attributes | `role="switch"`, `aria-checked="true"` when on, `aria-checked="false"` when off | FS-A11Y-003 | P1 |
| UIBC-A11Y-046 | Disabled Toggle has aria-disabled | Toggle (disabled) | Inspect attributes | `aria-disabled="true"` present | FS-A11Y-003 | P1 |
| UIBC-A11Y-047 | Dropdown trigger has aria-haspopup and aria-expanded | Dropdown | Inspect attributes | `aria-haspopup="listbox"` (or `"true"`), `aria-expanded="false"` when closed, `aria-expanded="true"` when open | FS-A11Y-003 | P1 |
| UIBC-A11Y-048 | Dropdown option list has role="listbox" | Dropdown (open) | Inspect popup element | `role="listbox"` on the option list container | FS-A11Y-003 | P1 |
| UIBC-A11Y-049 | Dropdown options have role="option" | Dropdown (open) | Inspect option elements | Each option has `role="option"`, `aria-selected="true"` for the selected value | FS-A11Y-003 | P1 |
| UIBC-A11Y-050 | Tooltip has role="tooltip" | Tooltip | Inspect tooltip element | `role="tooltip"` present | UXD §7.10, FS-A11Y-003 | P1 |
| UIBC-A11Y-051 | Tooltip trigger has aria-describedby | Tooltip trigger | Inspect trigger element | `aria-describedby` references the tooltip element ID | UXD §7.10, FS-A11Y-003 | P1 |
| UIBC-A11Y-052 | Dialog has role="dialog" or "alertdialog" | Dialog | Inspect dialog panel | `role="dialog"` for informational; `role="alertdialog"` for confirmations | UXD §7.9.2 | P1 |
| UIBC-A11Y-053 | Dialog has aria-modal="true" | Dialog (open) | Inspect dialog panel | `aria-modal="true"` present | UXD §11.3 | P1 |
| UIBC-A11Y-054 | Dialog has aria-labelledby pointing to heading | Dialog (open) | Inspect `aria-labelledby` | `aria-labelledby` references the dialog heading element ID | FS-A11Y-003 | P1 |

### 8.4 Touch Target Size

| ID | Scenario | Component | Measurement | Expected Result | FS Ref | Priority |
|----|----------|-----------|-------------|-----------------|--------|----------|
| UIBC-A11Y-060 | Button height meets 44px minimum | All buttons | Computed height | Height = 44px (`--size-target-min`) | FS-A11Y-002 | P1 |
| UIBC-A11Y-061 | Toggle hit area is 44×44px | Toggle | Computed hit area (wrapper/label size) | Hit area ≥ 44×44px even though visual track is 36×20px | FS-A11Y-002, UXD §7.16 | P1 |
| UIBC-A11Y-062 | Dialog action buttons meet 44px height | Dialog | Computed height of buttons inside dialog | Height = 44px | FS-A11Y-002 | P1 |
| UIBC-A11Y-063 | Dropdown trigger height meets 44px | Dropdown | Computed height | Height = 44px | FS-A11Y-002 | P1 |
| UIBC-A11Y-064 | TextInput field height meets 44px | TextInput | Computed height of input element | Height = 44px | FS-A11Y-002 | P1 |

---

## 9. i18n Scenarios (UIBC-I18N)

> FS-I18N-001/002 (EN/FR support), FS-I18N-004 (all UI strings from message catalogue — no hardcoded strings).

| ID | Scenario | Component | Steps | Expected Result | FS Ref | Priority |
|----|----------|-----------|-------|-----------------|--------|----------|
| UIBC-I18N-001 | Button label switches language | Button | Set locale to FR, render button with translated label prop | Button displays FR label | FS-I18N-001 | P1 |
| UIBC-I18N-002 | Dialog content renders translated strings | Dialog | Set locale to FR, open dialog | Heading, body, and button labels display FR text | FS-I18N-001 | P1 |
| UIBC-I18N-003 | TextInput label and helper text switch to FR | TextInput | Set locale to FR | Label and helper text display FR text | FS-I18N-001 | P2 |
| UIBC-I18N-004 | Toggle label switches to FR | Toggle | Set locale to FR, render toggle with associated label | Label displays FR text | FS-I18N-001 | P2 |
| UIBC-I18N-005 | Dropdown option labels are translated | Dropdown | Set locale to FR | All option labels render in FR | FS-I18N-001 | P2 |
| UIBC-I18N-006 | No hardcoded string in Button component | Button | Inspect component source | No string literals in component markup — all labels received via props or message accessors | FS-I18N-004 | P1 |
| UIBC-I18N-007 | No hardcoded string in Dialog component | Dialog | Inspect component source | Heading and body text come from props or message accessors, not string literals in the component | FS-I18N-004 | P1 |
| UIBC-I18N-008 | Long FR label does not break Button layout | Button | Set locale to FR with a known long label (>30 chars) | Button width expands to accommodate label; text does not overflow or truncate (unless explicitly truncated by design) | FS-I18N-002 | P2 |
| UIBC-I18N-009 | Long FR label does not break Dialog layout | Dialog | FR locale, open dialog with verbose translations | Dialog panel maintains padding and alignment; buttons remain right-aligned and reachable | FS-I18N-002 | P2 |
| UIBC-I18N-010 | TextInput placeholder accepts UTF-8 content | TextInput | Type multi-byte UTF-8 characters (e.g., accented, CJK) into input | Characters accepted and displayed correctly; no encoding errors | — | P2 |

---

## 10. Security Scenarios (UIBC-SEC)

**Version:** 1.0 — 2026-04-05
**Author role:** security-expert
**Scope:** Base UI components — Button, TextInput, Toggle, Dropdown, Tooltip, Dialog/Modal

### Threat Context

The base UI components constitute the primary interface between user-controlled data and the WebView renderer. Although Svelte's template engine escapes text interpolation by default, three attack vectors remain relevant at this layer:

- **XSS via props** — a prop value containing HTML or script markup passed to a component must be treated as text, never as markup. The risk materialises if any component uses `{@html}` on a prop value, or if Bits UI primitives render slot content via `innerHTML`.
- **Focus management bypass** — a Dialog that fails to trap focus allows keyboard navigation to reach background content, enabling UI redressing attacks where a background Tauri command trigger receives accidental or malicious input.
- **Event handler bypass** — disabled controls that still fire JavaScript handlers represent a logic bypass: a script injected via a compromised dependency could invoke a `disabled` Button's `onclick` callback directly through the DOM.

The CSP in `tauri.conf.json` (`script-src 'self'`) blocks external script injection and inline `<script>` tags. It does **not** prevent DOM-based XSS executed by an already-running script, nor does it block Svelte's own `{@html}` rendering path. The scenarios below address the residual risks that CSP does not cover.

### Summary Table

| ID | Scenario | Test Method | Expected Result | Severity |
|---|---|---|---|---|
| UIBC-SEC-001 | XSS via Button label prop | Pass `<script>alert(1)</script>` as `label` prop; inspect DOM text node | Text rendered as literal string; script not in DOM and not executed | Critical |
| UIBC-SEC-002 | XSS via TextInput placeholder | Pass `<img src=x onerror=alert(1)>` as `placeholder` prop; inspect rendered attribute | Attribute value is literal string; no `onerror` handler fires | Critical |
| UIBC-SEC-003 | XSS via Dropdown option labels | Pass `[{ value: 'x', label: '<script>alert(1)</script>' }]` as `options` prop; open dropdown; inspect DOM | Each option renders as a text node; no script tag in DOM | Critical |
| UIBC-SEC-004 | XSS via Tooltip content | Pass `<b onmouseover=alert(1)>hover</b>` as `content` prop; trigger tooltip; inspect DOM | Content rendered as literal string inside tooltip element | Critical |
| UIBC-SEC-005 | XSS via Dialog title and body | Pass `<img src=x onerror=alert(1)>` as `title` and `description` props; open dialog; inspect DOM | Both rendered as text nodes; no `onerror` handler fires | Critical |
| UIBC-SEC-006 | No `{@html}` on prop-derived content | Static scan: search all base component `.svelte` files for `{@html}` | Zero occurrences of `{@html}` in Button, TextInput, Toggle, Dropdown, Tooltip, Dialog | Critical |
| UIBC-SEC-007 | Dialog focus trap — Tab stays inside | Open Dialog; Tab through all focusable elements repeatedly | Focus cycles inside dialog only; no element outside dialog receives focus | High |
| UIBC-SEC-008 | Dialog closes on Escape and restores focus | Open Dialog from a Button; press Escape | Dialog is removed from DOM; focus returns to the trigger Button | High |
| UIBC-SEC-009 | Dropdown closes on Escape without side effects | Open Dropdown; press Escape | Dropdown popover is dismissed; no `change` or `select` event is emitted; focus returns to trigger | Medium |
| UIBC-SEC-010 | TextInput respects `maxlength` attribute | Render TextInput with `maxlength={10}`; type 20 characters | Input value length does not exceed 10 | Medium |
| UIBC-SEC-011 | TextInput transmits special characters verbatim | Set value to null byte and RTL override; read back | Value matches exactly — no stripping, no normalization | Medium |
| UIBC-SEC-012 | Disabled Button does not fire `onclick` handler | Render disabled Button with spy; call `.click()` on DOM | Spy not invoked | High |
| UIBC-SEC-013 | Disabled Toggle does not fire `onchange` handler | Render disabled Toggle with spy; dispatch click | Spy not invoked; state unchanged | High |
| UIBC-SEC-014 | No `{@html}` in any base component — automated static check | Vitest static test reads each file; asserts zero `{@html}` outside comments | All six files pass | Critical |
| UIBC-SEC-015 | No inline event handlers in generated HTML | Mount each component; check `innerHTML` for `onclick=`, `onerror=`, etc. | Zero matches | High |

### Detailed Scenarios

#### UIBC-SEC-001 — XSS via Button label prop

| Field | Value |
|---|---|
| **STRIDE** | Tampering / Elevation of Privilege |
| **Threat** | Label prop containing `<script>alert(1)</script>` renders as raw HTML if `{@html}` is used. |
| **Test method** | Mount `<Button label="<script>alert(1)</script>" />`; assert `textContent` equals literal string; assert no `<script>` in DOM; assert `window.alert` not called. |
| **Expected mitigation** | Svelte text interpolation (`{label}`) escapes HTML entities automatically. |
| **Priority** | Critical |

#### UIBC-SEC-002 — XSS via TextInput placeholder

| Field | Value |
|---|---|
| **STRIDE** | Tampering |
| **Threat** | `placeholder` prop with `<img src=x onerror=alert(1)>` fires `onerror` if bound unsafely. |
| **Test method** | Mount `<TextInput placeholder="<img src=x onerror=alert(1)>" />`; assert `getAttribute('placeholder')` is the literal string; assert `window.alert` not called. |
| **Expected mitigation** | Svelte attribute binding always escapes the value. |
| **Priority** | Critical |

#### UIBC-SEC-003 — XSS via Dropdown option labels

| Field | Value |
|---|---|
| **STRIDE** | Tampering / Elevation of Privilege |
| **Threat** | Option labels rendered via Bits UI `Select.Item` could reach `innerHTML` if the primitive uses unsafe DOM insertion. |
| **Test method** | Mount Dropdown with `options=[{ value: 'x', label: '<script>alert(1)</script>' }]`; open; assert options contain text nodes only; assert `window.alert` not called. |
| **Expected mitigation** | Labels passed as Svelte slot text interpolation — Bits UI renders slot content as provided, not via `innerHTML`. |
| **Priority** | Critical |

#### UIBC-SEC-004 — XSS via Tooltip content

| Field | Value |
|---|---|
| **STRIDE** | Tampering |
| **Threat** | Tooltip `content` prop with markup renders if `{@html content}` is used. |
| **Test method** | Mount Tooltip with `content="<b onmouseover=alert(1)>hover</b>"`; trigger; assert `textContent` is the literal string; assert no `onmouseover` attribute. |
| **Expected mitigation** | Content rendered via `{content}` text interpolation only. |
| **Priority** | Critical |

#### UIBC-SEC-005 — XSS via Dialog title and body

| Field | Value |
|---|---|
| **STRIDE** | Tampering / Elevation of Privilege |
| **Threat** | Dialog `title` and `description` props with XSS payloads execute if rendered via `{@html}`. Particularly sensitive as dialogs confirm privileged actions. |
| **Test method** | Mount Dialog with XSS payloads in `title` and `description`; open; assert both appear as literal text; assert `window.alert` not called. |
| **Expected mitigation** | Both props rendered via Svelte text interpolation. |
| **Priority** | Critical |

#### UIBC-SEC-006 — No `{@html}` on prop-derived content

| Field | Value |
|---|---|
| **STRIDE** | Tampering |
| **Threat** | Future modification introduces `{@html}` on a prop expression, bypassing Svelte's automatic escaping for all call sites. |
| **Test method** | Static analysis: read each of the six component files; assert `{@html` does not appear outside comment nodes. Automated in Vitest (see UIBC-SEC-014 code). |
| **Expected mitigation** | `{@html}` permanently forbidden in base UI components. |
| **Priority** | Critical |

#### UIBC-SEC-007 — Dialog focus trap

| Field | Value |
|---|---|
| **STRIDE** | Elevation of Privilege |
| **Threat** | Without focus trap, Tab can reach background controls including PTY-triggering elements. |
| **Test method** | Mount layout with background Button + open Dialog; Tab repeatedly; assert `document.activeElement` always within dialog subtree. |
| **Expected mitigation** | Bits UI `Dialog.Root` implements focus trap natively. Component must not disable it. |
| **Priority** | High |

#### UIBC-SEC-008 — Dialog Escape restores focus

| Field | Value |
|---|---|
| **STRIDE** | Elevation of Privilege |
| **Threat** | Undefined focus location after close leaves keyboard input reaching unintended targets, including PTY input. |
| **Test method** | Focus trigger Button; open Dialog; press Escape; assert `document.activeElement === triggerButton`. |
| **Expected mitigation** | Bits UI `Dialog.Root` restores focus to previously focused element on close. |
| **Priority** | High |

#### UIBC-SEC-009 — Dropdown Escape without side effects

| Field | Value |
|---|---|
| **STRIDE** | Tampering |
| **Threat** | Escape on open Dropdown fires a `change` event with the highlighted (uncommitted) value, triggering IPC calls. |
| **Test method** | Open Dropdown; highlight option with Arrow; press Escape; assert `onchange` not called; assert `value` unchanged. |
| **Expected mitigation** | Bits UI `Select` commits only on click or Enter. |
| **Priority** | Medium |

#### UIBC-SEC-010 — TextInput maxlength enforcement

| Field | Value |
|---|---|
| **STRIDE** | Denial of Service |
| **Threat** | No maxlength allows large pastes that overflow IPC payload limits (Rust `MAX_INPUT_SIZE` = 64 KiB). |
| **Test method** | Mount `<TextInput maxlength={256} />`; simulate paste of 1024-char string; assert `value.length <= 256`. |
| **Expected mitigation** | `maxlength` prop forwarded to native `<input>` HTML attribute; browser enforces natively. |
| **Priority** | Medium |

#### UIBC-SEC-011 — TextInput verbatim special characters

| Field | Value |
|---|---|
| **STRIDE** | Tampering |
| **Threat** | Silent stripping of control characters (null bytes, RTL overrides) causes mismatch between visible value and IPC payload. |
| **Test method** | Set input value to `"test\u0000injection"` and `"safe\u202enoitcejni"`; read back bound state; assert exact match. |
| **Expected mitigation** | TextInput performs no character filtering — validation is the consumer's responsibility. |
| **Priority** | Medium |

#### UIBC-SEC-012 — Disabled Button event bypass

| Field | Value |
|---|---|
| **STRIDE** | Elevation of Privilege |
| **Threat** | Disabled Button fires handler when `.click()` called programmatically, allowing a compromised dependency to invoke IPC commands the user disabled. |
| **Test method** | Mount `<Button disabled={true} onclick={spy} />`; call `button.click()`; assert spy not called. |
| **Expected mitigation** | Native `<button disabled>` prevents click event dispatch. Svelte `disabled={true}` must produce a DOM `disabled` attribute, not just a CSS class. |
| **Priority** | High |

#### UIBC-SEC-013 — Disabled Toggle event bypass

| Field | Value |
|---|---|
| **STRIDE** | Elevation of Privilege |
| **Threat** | Toggle implemented as a styled `<div>` does not inherit native disabled semantics. |
| **Test method** | Mount `<Toggle disabled={true} onchange={spy} />`; dispatch click; assert spy not called; assert state unchanged. |
| **Expected mitigation** | Toggle click handler explicitly checks `disabled` before invoking callback (or uses native `<button role="switch">` which respects disabled). |
| **Priority** | High |

#### UIBC-SEC-014 — Automated static `{@html}` scanner

| Field | Value |
|---|---|
| **STRIDE** | Tampering / Elevation of Privilege |
| **Threat** | `{@html}` introduced silently in a future commit renders attacker-controlled props as raw HTML. |
| **Test method** | Vitest static test: read each component file; strip comment nodes; assert `{@html` absent. Code: see Automation Note below. |
| **Expected mitigation** | `{@html}` absent from all six component files. Mirrors the approach in `security_static_checks.rs`. |
| **Priority** | Critical |

#### UIBC-SEC-015 — No inline event handlers in rendered HTML

| Field | Value |
|---|---|
| **STRIDE** | Tampering |
| **Threat** | String-templated HTML attributes like `onclick=` bypass Svelte's event system and could be exploited via prototype pollution. |
| **Test method** | Mount each component; query `document.body.innerHTML`; assert no match for `/\s(onclick\|onmouseover\|onerror\|onload\|oninput\|onchange)=/i`. |
| **Expected mitigation** | All handlers wired via Svelte event directives — compiled to `addEventListener`, not HTML attributes. |
| **Priority** | High |

### Automation Note — UIBC-SEC-014 and UIBC-SEC-015

**UIBC-SEC-014 static scanner** (Vitest, no jsdom required):

```typescript
// src/lib/ui/__tests__/security-static.test.ts
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { describe, it, expect } from 'vitest';

const COMPONENTS = ['Button', 'TextInput', 'Toggle', 'Dropdown', 'Tooltip', 'Dialog'];
const COMPONENTS_DIR = resolve(__dirname, '..');

describe('UIBC-SEC-014 — no {@html} in base UI components', () => {
  for (const name of COMPONENTS) {
    it(`${name}.svelte contains no {@html} outside comments`, () => {
      const source = readFileSync(resolve(COMPONENTS_DIR, `${name}.svelte`), 'utf-8');
      const withoutHtmlComments = source.replace(/<!--[\s\S]*?-->/g, '');
      const withoutJsLineComments = withoutHtmlComments.replace(/\/\/[^\n]*/g, '');
      const withoutJsBlockComments = withoutJsLineComments.replace(/\/\*[\s\S]*?\*\//g, '');
      expect(withoutJsBlockComments).not.toContain('{@html');
    });
  }
});
```

**UIBC-SEC-015 inline handler check** (Vitest + jsdom):

```typescript
// Each component gets a render test that checks document.body.innerHTML
// for absence of inline event handler attributes.
const html = document.body.innerHTML;
expect(html).not.toMatch(/\s(onclick|onmouseover|onerror|onload)=/i);
```

---

## 11. Design Token Fidelity Scenarios (UIBC-TOK)

> These verify that components reference design tokens (CSS custom properties) rather than hardcoded values. Violation = technical debt that breaks theming.

| ID | Scenario | Component | Check | Expected Result | Priority |
|----|----------|-----------|-------|-----------------|----------|
| UIBC-TOK-001 | Primary button uses `--color-accent` token for background | Button primary | Inspect computed `background-color` CSS property on button element | Value matches `--color-accent` resolved value (#4a92bf), not hardcoded hex | P1 |
| UIBC-TOK-002 | Ghost button hover uses `--color-hover-bg` token | Button ghost | Inspect computed background on hover | Value matches `--color-hover-bg` (#2c2921) | P1 |
| UIBC-TOK-003 | Focus ring uses `--color-focus-ring` token | All components | Inspect outline/border on focused element | Value matches `--color-focus-ring` (#4a92bf) | P1 |
| UIBC-TOK-004 | Tooltip background uses `--color-bg-raised` token | Tooltip | Inspect background | Value matches `--color-bg-raised` (#2c2921) | P2 |
| UIBC-TOK-005 | Dialog backdrop uses `--color-bg-overlay` at 60% opacity | Dialog | Inspect backdrop element | Background color `--color-bg-overlay` (#16140f) at 60% opacity | P2 |
| UIBC-TOK-006 | Toggle accent track uses `--color-accent` token | Toggle (checked) | Inspect track background | Value matches `--color-accent` (#4a92bf) | P2 |
| UIBC-TOK-007 | TextInput focus ring uses `--color-focus-ring` | TextInput (focused) | Inspect border | 2px solid matching `--color-focus-ring` (#4a92bf) | P1 |
| UIBC-TOK-008 | Error state uses `--color-error` and `--color-error-text` tokens | TextInput (error) | Inspect border and error text color | Border `--color-error` (#c44444); error text `--color-error-text` (#d97878) | P1 |

---

## 12. Reduced Motion Scenarios (UIBC-MOTION)

> UXD §9.4, §11.4. All animations must be suppressed under `prefers-reduced-motion: reduce`. State changes must still be visible via color/shape.

| ID | Scenario | Component | Steps | Expected Result | Priority |
|----|----------|-----------|-------|-----------------|----------|
| UIBC-MOTION-001 | Toggle thumb slide disabled under reduced motion | Toggle | Set `prefers-reduced-motion: reduce` (via CSS media or test utility), toggle | State changes instantly; no 100ms slide transition | P2 |
| UIBC-MOTION-002 | Dialog opacity transition disabled under reduced motion | Dialog | `prefers-reduced-motion: reduce`, open dialog | Dialog appears instantly at full opacity; no 100ms fade | P2 |
| UIBC-MOTION-003 | Dropdown menu fade disabled under reduced motion | Dropdown | `prefers-reduced-motion: reduce`, open dropdown | Option list appears instantly | P2 |
| UIBC-MOTION-004 | All state changes still visible without animation | All components | `prefers-reduced-motion: reduce`, interact with all components | Checked/unchecked, hover, focus, error states still distinguishable by color/shape without relying on animation | P1 |

---

## 13. Scenario Summary

| Category | Prefix | Count | Priority P1 | Priority P2 |
|----------|--------|-------|-------------|-------------|
| Button functional | UIBC-FN-BTN | 17 | 12 | 5 |
| TextInput functional | UIBC-FN-INP | 10 | 8 | 2 |
| Toggle functional | UIBC-FN-TOG | 11 | 7 | 4 |
| Dropdown functional | UIBC-FN-DRP | 12 | 8 | 4 |
| Tooltip functional | UIBC-FN-TIP | 10 | 5 | 5 |
| Dialog/Modal functional | UIBC-FN-DLG | 16 | 10 | 6 |
| Accessibility — contrast | UIBC-A11Y (001–013) | 13 | 10 | 3 |
| Accessibility — keyboard | UIBC-A11Y (020–032) | 13 | 13 | 0 |
| Accessibility — ARIA | UIBC-A11Y (040–054) | 15 | 15 | 0 |
| Accessibility — touch targets | UIBC-A11Y (060–064) | 5 | 5 | 0 |
| i18n | UIBC-I18N | 10 | 4 | 6 |
| Security | UIBC-SEC | 15 | 15 | 0 |
| Design token fidelity | UIBC-TOK | 8 | 5 | 3 |
| Reduced motion | UIBC-MOTION | 4 | 1 | 3 |
| **TOTAL** | | **159** | **118** | **41** |

---

## 14. Implementation Notes

### Test framework mapping

| Layer | Tool | Scope |
|-------|------|-------|
| Component unit tests | Vitest + `@testing-library/svelte` | All UIBC-FN-*, UIBC-I18N-*, UIBC-SEC-*, UIBC-TOK-* |
| Computed style assertions | Vitest + `getComputedStyle()` | UIBC-TOK-*, UIBC-A11Y-001 to 013 |
| ARIA inspection | Vitest + `@testing-library/svelte` queries | UIBC-A11Y-040 to 054 |
| Keyboard navigation | Vitest + `userEvent` (keyboard simulation) | UIBC-A11Y-020 to 032 |
| Timing (tooltip delay, transitions) | Vitest + `vi.useFakeTimers()` | UIBC-FN-TIP-002/003, UIBC-FN-TOG-005 |
| Reduced motion | Vitest + `matchMedia` mock | UIBC-MOTION-* |
| Visual / viewport E2E | WebdriverIO | UIBC-FN-TIP-007/008 (flip positioning), UIBC-FN-DLG-004/005 (panel widths) |

### Bits UI usage

All Dropdown, Tooltip, and Dialog components must use Bits UI headless primitives. ARIA attributes listed in §8.3 are expected to be provided by Bits UI and must be verified — do not assume they are present without an assertion.

### Contrast ratios — noted issues

The following contrast situations require attention before sign-off:
- `--color-text-tertiary` (#6b6660) on `--term-bg` (#16140f): estimated ~3.0:1. Placeholder text is WCAG-exempt under 1.4.3, but this should be documented.
- UIBC-A11Y-002 (disabled button): WCAG 1.4.3 explicitly exempts disabled controls. No hard fail, but the ratio must be documented in the test report.

