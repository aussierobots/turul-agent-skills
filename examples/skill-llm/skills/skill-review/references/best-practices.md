# Review rubric

Source: https://agentskills.io/skill-creation/best-practices (paraphrased; read the
page for the full argument). Each entry gives the id to use in the report, what good
looks like, and a sign of failure.

## real-expertise
Good: content comes from real tasks, runbooks, schemas or past corrections, so it holds
facts specific to this project or domain.
Failing sign: generic procedures such as "handle errors appropriately" that any model
would write unprompted.

## adds-what-agent-lacks
Good: each paragraph states something the agent would otherwise get wrong, such as a
convention, a non-obvious tool choice or an edge case.
Failing sign: explanations of common knowledge, such as what a file format is.

## coherent-unit
Good: the skill covers one unit of work that composes with other skills, like a
well-scoped function.
Failing sign: several skills would always have to load together for one task, or one
skill spans unrelated jobs and activates imprecisely.

## moderate-detail
Good: concise, stepwise guidance with a working example; rare edge cases are left to the
agent's judgment.
Failing sign: exhaustive coverage of every case, so the agent follows instructions that
do not apply to the task at hand.

## progressive-disclosure
Good: `SKILL.md` carries the core instructions; bulky material sits in `references/` or
`assets/`, and the body says exactly when to load each file. The measurable advisories
(length, nesting depth) are inputs here.
Failing sign: a generic "see references/" with no trigger, a very long body, or a
reference file that points to further files.

## specificity-matches-fragility
Good: freedom, with the reason given, where several approaches work; exact commands and
"do not change" wording where an operation is fragile or order matters.
Failing sign: rigid steps for a task that tolerates variation, or loose prose for a
destructive or order-sensitive operation.

## defaults-not-menus
Good: one default tool or approach, with a brief escape hatch for the exceptional case.
Failing sign: a list of equal alternatives and no recommendation.

## procedures-over-declarations
Good: teaches a method that works for any instance of the task; specific details appear
as templates, constraints or tool usage.
Failing sign: the answer to one particular instance, such as a hard-coded query for a
named table.

## gotchas
Good: a section in `SKILL.md` of concrete, environment-specific corrections to mistakes
the agent would make.
Failing sign: no gotchas at all where the domain has surprises, or gotchas that are
general advice, or gotchas parked in a file the agent may never open.

## output-templates
Good: when the output has a required shape, a concrete template is given, inline if
short or in `assets/` if long or occasional.
Failing sign: the format is described only in prose, or varies run to run.

## checklists
Good: a multi-step workflow with dependencies or gates is a checklist the agent can
tick off in order.
Failing sign: a long ordered procedure where steps get skipped because nothing tracks
progress.

## validation-loops
Good: the agent runs a validator (script, reference check or self-check), fixes what it
reports and repeats until clean; batch or destructive work validates a plan first.
Failing sign: the skill produces output or edits files and never says how to check them.

## bundled-scripts
Good: logic the agent would otherwise reinvent every run lives in a tested script under
`scripts/`, with the command to run it.
Failing sign: long inline code the agent retypes each time, or a script that is
referenced but never explained.

## description-what-and-when
Good: the description says what the skill does and when to use it, in the words a user
would use, within the length limit.
Failing sign: a label with no trigger, a trigger with no capability, or vocabulary the
user would never type.
