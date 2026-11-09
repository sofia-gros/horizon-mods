# Working with Codex

Codex does the whole port while Claude Code is away (until Thursday, 2026-11-12). It runs in a Linux container in the cloud, one task for each run; the owner runs the game on the PC and merges. **Nobody reviews its work in between, so the scripts in `tools/` are the reviewer, and they are strict on purpose.** Codex is fast and has a lot of budget, and it drifts: it writes tests only to pass, writes pages of documents, widens its scope, ports the wrong thing, and ignores the roadmap. This page and the guard exist to stop each of those. Read the whole page before every task. When this page and your own judgement disagree, this page wins.

## The only workflow (one task for each run)

1. `python tools/codex_next.py` prints the table of tasks and `NEXT: <id>`. **Do that task and no other.** You do not choose work, reorder it, merge two tasks, or start something you think is more useful. If it says nothing is left, stop and write the final message.
2. `python tools/codex_start.py <id>` checks that the task is the next one, fetches the original's Java source into `.local/tconstruct-source` (needs internet once) and remembers the commit you start from. Run it before any change, in a clean checkout of `main`.
3. Read, in this order: `AGENTS.md`, this page, every file under `read_first` in `docs/codex/tasks/<id>.json`, then the Java files under `java` (`.local/tconstruct-source/src/main/java/`, read only). **The Java is the specification.** The existing Rust is of unknown quality; copy its style (see `src/smeltery/` and `src/recipes/` in the library), not its content.
4. Port the classes of the task one group at a time, in the order of the task. Commit after each group (message in a file, `git commit -F`). After each group run `cargo test --locked --manifest-path tconstruct/mixins/tconstruct/api/library/Cargo.toml` (library tasks).
5. When every class is ported or honestly left, run the `verify` commands of the task yourself, write the final message to a file, and run `python tools/codex_finish.py <id> --final-message FILE`. It runs the guard and the verify commands and prints `READY` only if all pass.
6. If it prints `GATE FAILED`, fix the cause and run it again. After three failed tries stop and report; never work around a gate.
7. When it prints `READY`, commit everything and end the run with the final message. **Do not start a second task in the same run**: the next task starts from the merged result of this one, and the owner merges it first.

You never run `git push`, `gh`, `git merge` or `git tag`, and never edit `main`; the cloud run opens the pull request and the owner merges it.

## Two kinds of task

- **`library`**: plain Rust in `tconstruct/mixins/tconstruct/api/library`, checked by `cargo test`, which you can run. Tests are required (below).
- **`game`**: code in the Tinkers API crate and content files, on PlutoLib's API only. The crate cannot be built in your container (it needs the game launcher), so you cannot compile it. Write small, careful changes in the style of `station.rs` and `world.rs`, check every PlutoLib call against `pluto-lib/docs/api/*.md`, and use only what those pages document. The owner builds and runs the game on the PC (`tools/codex_pc_check.py`) and gives you the compiler errors or the probe log as a follow-up in the same task: fix exactly those, nothing else. A game task has no unit tests; its proof is a line in the game probe (`tools/smoke-local/probe_pluto_lib.py`) that your code prints and the probe expects.

## What you may touch

- Only the paths in `allowed_paths` of the task. `AGENTS.md`, `CLAUDE.md`, `docs/roadmap.md`, `docs/codex*`, `tools/` (except the one probe file a game task names), `tests/`, `.github/`, `pluto-lib/` (except the probe consumer a game task names), `Cargo.toml`, `Cargo.lock` and any `*.md` are off limits. The guard fails the work if you change one.
- In the task's inventory file change only the classes the task names, and only to `ported-unchecked`, `partial` or `not-code`, each with a note that says where the port is and what is tested or which probe line proves it (at least 40 characters), **ending with the task tag `[<id>]`**. A class you could not finish stays `partial` with a tagged note, or stays `todo`. **Never write `ported`** (only the owner's agent may, after the game is run). Never remove a class. A task counts as done when none of its classes is `todo` and every `partial` one carries the tag.
- You do not read or change the roadmap. Its order for you is `codex_next.py`.
- No mod integration: skip the original's `plugin` package and anything that exists only for another mod.
- A game task adds; it does not delete or edit the legacy injections (`source-*.rs`, `horizon-mixin.toml`). They are removed later by the owner's agent.
- PlutoLib is read-only. If it lacks an API the port needs, stop and report (see below); never use a direct injection.

## Tests (library tasks): few, real, from the Java

Tests exist to catch a wrong port, not to turn a check green. A test that was written to pass, or that mirrors the code under test, is worse than none, and the guard counts and inspects them.

- **At most `max_new_tests` new `#[test]` in the whole task** (the number is in the task file). One test per behaviour of the Java, not per line, per getter or per type. Put it in a `#[cfg(test)] mod tests` at the bottom of the file it tests. Never add a test file; the only test file you may extend is `tests/original_data.rs`.
- **Write the expected value from the Java, not from your output.** Work out by hand what the Java returns for the input (read the method, do the arithmetic), then write that number. Do not run your code and paste its result as the expectation.
- **Check once that each test can fail**: break the code on purpose, see the test fail, put the code back. Say in the final message that you did, for how many.
- Forbidden, and the guard looks for them: `assert!(true)`, comparing a value with itself, `#[ignore]`, removing or weakening an existing test, a test with no assertion, a test of a getter or a constructor, and tests that only check that parsing does not panic. A test of the original's data files (in `original_data.rs`) must assert how many files loaded, with the number taken from the folder, not a loose `> 0`.
- Never change an existing test to make it pass. If a test you did not write fails, the cause is in your change or it is a stop condition below.

## Documents and comments: none

- **No new document of any kind** (`.md`, `.txt`, `.json` other than the inventory, diagrams). No README, no notes, no changelog, no summary file. The final message is your report.
- Code comments are short and say *why* when it is not obvious from the Java. Each public item gets one `///` line saying what it is in the original's terms. No block comments restating the code, no commented-out code, no TODO lists. The guard fails a file whose new lines are mostly comments.
- No `println!`, `dbg!`, `todo!`, `unimplemented!`, `#[allow(...)]`: the guard fails them. (A game task prints its probe line with `println!` in the Tinkers API crate; the guard allows exactly that.)

## Port, do not invent

- Port what the Java does, method by method, including the edge cases (clamps, defaults, rounding, order of checks). If the Java is awkward, port the awkwardness. Do not "improve" it, rename its concepts, or simplify a rule because it looks unnecessary.
- If the Java does something the library cannot do yet because it needs the game, put a small trait where the game would answer, port the rest, and mark the class `partial` with that reason. Do not fake the game side.
- Do not touch code outside the task to make yours fit. If an existing type is missing something, stop and report (below).
- LF line endings everywhere.

## Stop and report (do not guess)

Stop, commit what is finished, mark the classes you did not finish `partial` (tagged) or leave them `todo`, and write the reason in the final message when:

- a type, function or PlutoLib API the port needs is missing and is not part of your task;
- Horizon cannot do what the Java does (name it);
- a rule on this page conflicts with the task;
- a verify command fails and the cause is not in your change;
- you are about to touch a path that is not allowed;
- the guard or `codex_finish.py` keeps failing for a reason you cannot fix in three tries.

Stopping with a partial result and an honest message is a good outcome. A finished-looking change that is wrong is the worst outcome.

## Honesty

- Say "verified" only for commands you ran and whose output you copy. You cannot run the game; anything about the game is `NOT RUN`.
- Do not claim a class is complete when a method is missing. List what is left under `PARTIAL / LEFT`.
- Do not edit the guard, the scripts or the task file to make a gate pass.

## Final message (exactly this shape; `codex_finish.py` checks the headings; it is also the pull request description, so it starts with `TASK <id>`)

```
TASK <id>
DONE: <classes fully ported, one line each>
PARTIAL / LEFT: <class: what is missing and why; "none" if none>
FILES: <count> files, <count> lines
VERIFY: <each verify command and its result, copied; commands not run are listed as NOT RUN>
TESTS: <count> new tests; <count> checked to fail when the code is broken
QUESTIONS: <anything the owner's agent must decide; "none" if none>
```

## Paste this into Codex to start (and again after each task is merged)

```
Read docs/codex.md in this repository completely, then follow "The only workflow" exactly. Run python tools/codex_next.py to learn the task; do that task and nothing else, and only one task in this run. Do not read or change docs/roadmap.md. Do not write documents. Write few tests (library tasks), with expected values worked out from the Java. When the task is finished run tools/codex_finish.py as the page says and end with the final message. If codex_next.py says nothing is left, stop.
```

## Setting up the cloud environment (owner, once)

In the Codex environment settings for this repository: internet access on during setup, a setup command `python tools/fetch_original.py` (it fetches the original's source, about 100 MB, so a run does not have to), and Rust stable with `cargo`, Python 3.10 or newer and `git` available. Codex needs no GitHub credentials: the platform opens the pull request.

## Merging and the game (owner)

After a run: open its pull request, then on the PC

```
python tools/codex_pc_check.py <PR number>            # checks it out, runs the guard, the tests, and the game probe if the change touches the game side
python tools/codex_pc_check.py <PR number> --merge    # the same, and merges when everything passed
```

If a step fails, give its output to Codex as the follow-up of that task; do not merge. Merge each pull request before starting the next task, because the next task starts from `main`. The owner's agent reads the merged diffs against the Java from Thursday, removes weak tests and marks classes `ported`.
