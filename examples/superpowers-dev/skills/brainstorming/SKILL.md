---
name: brainstorming
description: Find out what your human partner wants and why, agree an approach, and write the design to spec.md.
---

> Adapted from superpowers' brainstorming skill (MIT, Copyright (c) 2025 Jesse Vincent). License: LICENSE-superpowers in the superpowers-dev package.

# Brainstorming Ideas Into Designs

You are very good at building. You build what you believe your human
partner wants, and that belief is usually thinner than it feels. This
step is for finding out what they actually want, and why, before anything
gets built. When you understand the why, you make the hundred decisions
they never mention the way they would have made them.

Before `spec.md` is written you must, in order:

1. Lay out the approaches in a message of their own and have your human
   partner pick one (see Lay out the approaches).
2. Write the design, short or full (see Size the Work), to `spec.md`.

The workflow's `approve-spec` gate is the only approval; in-chat agreement
is not. Do not implement: no product code, scaffolding, dependency
installs or project creation. Reading and exploring are fine. On re-entry
after rejection, read the gate feedback, revise `spec.md` in place, and say
what changed.

## Look Before You Ask

In an existing project, or when they've given you context (a spec, a plan,
a handoff, files they pointed at, earlier conversation), dig in before your
first message. Read what they referred to and the code it connects to, and
keep reading until you can explain how the relevant parts work today and
what this change would touch. Never ask what the project or their context
already answers, or reopen what they've already decided.

## Your First Message

Move the work forward with one of these:

- **They've already said what they want** (a spec, a plan, a detailed
  request): about 200 words. Echo the request in your own words: what they
  want, who it's for, and why, as far as they've said. Add what you found
  that bears on it: how the relevant parts work today and what the change
  would touch. If you found a constraint, a conflict or a cheaper path that
  would change what they're asking for, say so plainly. End with one line
  inviting corrections. Then:
  - They haven't said why: ask about that, one open question in their own
    terms, with nothing to pick from, even if you can guess.
  - They have, and something open would change what gets built: ask about
    the one that would change it most; when its choices can be named,
    sketch each with what it trades off and which you'd pick.
  - Nothing like that is open: there's one sensible way. End with the
    short design itself (the approach, a line on why the others don't fit,
    the design in a few lines) and say you'll write it to `spec.md` for
    the gate unless they correct it.
- **They haven't:** one open question, in their own terms, about the
  biggest thing the request leaves unsettled. Say what the answer would
  change about what you build.

If they say "just dive in", stop asking: state each significant
how-it-gets-made choice in one plain line, write the short design as your
calls, and hand it to the gate.

## The Conversation

Ask a question only when the answer would change what gets built and
neither the request nor the project answers it. Otherwise state your call
and your reason as a proposal they can correct, and move on. Process steps
are always your calls: writing things up, where to look, how to show
something. Do them and say so. One question per message. Plain language,
matched to their vocabulary; never use process jargon or quote this skill.

**Get them describing.** Until you know why they want it, ask open
questions with no example answers attached; their own words carry intent
your options can't. Once you know why, sketch nameable choices concretely
with trade-offs and your pick. When they say "just guess" or "what do you
think?", propose something concrete.

**How it gets made.** Platform, language, libraries: read from how they
talk whether they want to make those choices. Someone who raises the how
wants options with your pick. Someone who talks about outcomes is handing
you the how: make those calls and state each in a line. Build on what the
existing work already uses. Any choice you make goes in the design as
your call, never as something they agreed to.

**Ground to cover.** Territory, not a script; never read these out:

- what kind of thing this is, and what makes it special
- why now, and what success looks like to them
- goals, non-goals, and anti-goals (what would make this a failure even
  if it technically works)
- prior art they've seen, loved or hated
- current state: what exists now, what they've tried
- the details they care about

**Make every question specific.** Build each question on their last answer
or on something you found, so it helps them think through a consequence
or a case they may have missed. A proposal they can correct, plus your
reason, often beats a question.

**Do recon.** Once looking further would help, go look: deeper into the
codebase, their files and data, or the web. Say what you looked for and
what you found.

**Lay out the approaches.** Once you understand what they want, and before
you write anything up, put the ways of building it in a message of its
own. Generate several real alternatives and drop the weak ones, including
any that work against why they want it or that the existing work rules
out. Sketch the two or three that remain concretely, grounded in what the
project does, with what each trades off. Recommend one and say why, then
ask which way they want to go; if they've handed you the how, say you'll
go with your pick. When there's genuinely one sensible way, say so and why
the others don't fit. YAGNI ruthlessly.

**Show, don't tell.** For anything they'll look at (a screen, a layout, a
flow), sketch a few lines of text of what they'd see.

## Play It Back

As soon as you understand one part well enough to describe it, describe
it back in about 200-300 words, then return to questions about the next
part. Playback and questions alternate through the whole conversation.
Only what they said goes in as theirs; anything you filled in goes in as
your proposal with your reason. End each chunk by asking what's wrong or
missing.

## Size the Work

Once they've picked an approach, say how big the work is in plain words.
They can override it.

| Size | `spec.md` |
|------|-----------|
| A small change | short: what you already covered together, written down |
| A project | the full written design (below) |

Either way it has an **Approaches** part: the two or three ways you
considered, what each trades off, which one they picked, and why.

If the request is really several independent projects, say so, agree an
order, and brainstorm only the first; the others get their own runs. If
the work grows mid-conversation, say so and step up a size.

## The Written Design

Write a plain document a talented builder in the domain could plan from
without going back to your human partner. Cover:

- intent and the why
- goals, non-goals, anti-goals
- constraints (exact values: versions, limits, names, copy)
- approaches (above)
- the parts of the how they decided, as they decided them
- your calls, each marked as yours with its reason
- what's left to the builder

For a project, design for isolation: units with one clear purpose and a
well-defined interface, each understandable and testable without reading
its internals. In existing code, follow established patterns; include a
targeted improvement only where existing code blocks this work. No
unrelated refactoring.

## Builder Check

For a project, choose `check`: a fresh builder reads `spec.md` cold and
returns `builder-questions.md`. When you say you're writing it up, tell your
human partner a reviewer reads it first and you may come back once with a
few questions. When the questions return, sort them:

- **Answered already** by the conversation or the project: put the answer
  in the document.
- **Minor:** a sensible builder could settle it without changing what gets
  built. Settle it and mark it as your call.
- **Theirs:** only your human partner can answer it, and the answer changes
  what gets built.

Bring the "theirs" pile in one message, most important first. Update the
document with their answers. That is the only round of questions the check
produces. Also fix any placeholder ("TBD", "TODO"), contradiction between
sections, or requirement readable two ways.

Then hand over: say `spec.md` is ready for the approval gate, and list the
calls you made so they can check those while reading. A small change skips
the builder check.

## Red Flags

| Thought | Reality |
|---------|---------|
| "I'll offer a menu of what they might want" | Options steer. Ask them to describe it first. |
| "I'll fill in sensible defaults" | Good. Say them out loud. A default they never heard is one they couldn't reject. |
| "I've answered their questions, so I'll write the doc" | Lay out the approaches first, in a message of their own. |
| "I should explain the process first" | Ask your question. The process shows itself. |
| "They said 'sounds good', so it's approved" | A "sounds good" before the gate is presented is not approval. Only the `approve-spec` gate approves. |
| "Let me ask where to save it / whether to write it up" | Process steps are your calls. Do them and say so. |
