---
name: skill-review
description: Review an Agent Skill against the agentskills.io best practices. Use when creating or changing a SKILL.md, before publishing it, or when a skill under-performs.
license: MIT OR Apache-2.0
---
# Skill review

Review one skill directory and report. Do not edit the skill under review.

Progress:
- [ ] Step 1: Validate the skill
- [ ] Step 2: Collect the measurable advisories
- [ ] Step 3: Judge each practice
- [ ] Step 4: Write the report

## Step 1: Validate

Run `turul-skills validate <dir>`. If it prints errors, stop and report them: a skill
that does not validate cannot be reviewed for quality. Otherwise continue.

## Step 2: Collect the measurable advisories

Take these as given; do not re-count lines or re-trace links yourself.

- `skill-md-too-long`: the rendered `SKILL.md` exceeds 500 lines.
- `nested-reference`: a referenced file references another file, so the chain is deeper
  than one level.

Both come from `turul_agent_skills::advise`, or from the host that invoked you. Record
each as an `improve` finding for `progressive-disclosure`.

## Step 3: Judge each practice

Read `references/best-practices.md` now, and only now: it holds the rubric with one
entry per practice id. Read the skill's `SKILL.md` and list its bundled files, opening
only files whose content a verdict depends on.

For every id in the rubric, decide `pass`, `improve` or `not-applicable`, and quote the
skill text that supports the verdict. A verdict without a quotation is a guess; read
again or mark it `not-applicable`.

If you cannot decide a practice after one reading, say so in the evidence rather than
choosing a verdict.

## Step 4: Write the report

Read `assets/report-template.md` and fill it in. Use it exactly: one row per rubric id,
the verdict `good` only when no row says `improve`, otherwise `needs-work`.

Before finishing, check that every row has a quotation or a stated reason, and that
every suggestion names a concrete change to a named file or section.

## Gotchas

- A short skill is not a defect. Judge whether each line changes what the agent does,
  not how many lines there are.
- A minimal skill can legitimately mark `bundled-scripts`, `output-templates` or
  `progressive-disclosure` as `not-applicable`.
- Do not rewrite the skill under review. Report findings and suggestions only.
- Judge the description as the skill's only trigger: the agent sees it before it sees
  anything else, so "when to use" belongs there, not in the body.
- Advisories are measurements, not verdicts on quality. A long skill that earns its
  length can still pass the other practices.
