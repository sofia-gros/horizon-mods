# Working with Codex

Codex is used for large, well-specified porting work (many classes, many files) where the owner's agent (Claude Code) would run out of tokens. It is fast and has a lot of budget, and it drifts: it forgets rules, widens its scope, writes tests nobody asked for. So its work is fenced: one task file, one branch, one guard, one reviewer.

## Rules for Codex (read this whole page before every task)

1. **Do exactly the task in `docs/codex/tasks/<id>.json`, nothing else.** If something else looks wrong, write it in your final message and leave it.
2. **Read first**: `AGENTS.md`, this page, every file under `read_first` in the task, and the Java files under `java`. The original Java is the specification; the existing Rust is of unknown quality and may be wrong. Port what the Java does, method by method.
3. **Touch only `allowed_paths`.** Everything else is off limits, and `AGENTS.md`, `docs/roadmap.md`, `docs/codex*`, `tools/`, `tests/`, `.github/` are always off limits.
4. **No tests, no new documents, no new binaries** unless the task says so. Do not "improve" the build, the tools, the formatting or other code.
5. **PlutoLib is read-only for tasks that do not list it.** If the task needs a PlutoLib API that does not exist, stop (see below). Never use a direct injection; mods call PlutoLib's API only.
6. **No mod integration** (the original's `plugin` package and compat metals): skip it.
7. **LF line endings** in every file you write. Write commit messages to a file and use `git commit -F`.
8. **Work in your own branch and worktree**, never in the owner's checkout, never on `main`; do not push, merge, tag, open pull requests or touch GitHub. Commit in small steps with plain messages.
9. **Do not say "works" or "verified" for anything you did not run.** Run only the commands under `verify` in the task. You cannot run the game; the owner's agent does that after review.
10. **In the inventory file, change only the classes the task names**, to at most `ported-unchecked`, with a note that says what was ported and what was left. Never write `ported`.

## Start

```sh
git worktree add ../codex-<id> -b codex/<id> main
cd ../codex-<id>
```

Set `TMP`, `TEMP`, `CARGO_HOME` and `CARGO_TARGET_DIR` to folders on the roomy drive before running anything.

## Stop and report (do not guess)

Stop, commit what is finished, and write the reason in the final message when: a PlutoLib API the port needs is missing; Horizon cannot do something the Java does (name it and mark the class `partial` or leave it `todo`); a rule above conflicts with the task; a verify command fails and the cause is not in your change; you are about to touch a path that is not allowed.

## Final message (exactly this shape)

```
TASK <id>
DONE: <classes fully ported, one line each>
PARTIAL / LEFT: <class: what is missing and why>
FILES: <count> files, <count> lines
VERIFY: <each verify command and its result, copied; commands not run are listed as NOT RUN>
QUESTIONS: <anything the owner's agent must decide>
```

## Review (owner's agent)

`python tools/codex_guard.py <id> --base main` must print `0 problem(s)`. Then the local gate in `docs/development.md`, the game probe, and a read of the diff against the Java. Only then is the branch merged, and only the owner's agent marks a class `ported`.
