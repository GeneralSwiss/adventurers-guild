# Adventurers' Guild

The contract, escrow, and settlement domain of an adventurers' guild, modelled properly
in Rust.

A quest is posted with a bounty held in escrow. A party forms and then *changes* — people
join late, withdraw, die, and occasionally come back. When the quest resolves, the bounty
has to be divided across everyone who served, the Guild takes its cut, the dead are paid
to their estates, and the books have to balance to the copper.

It is a fantasy setting for an unfantastic reason: escrow, temporal membership, and
double-entry accounting are genuinely hard to model, and dressing them as guild
bureaucracy makes the hard parts easier to hold in your head. The domain is the point.

## What this crate is

`guild-domain` is a pure domain core. No I/O, no persistence, no framework, no async, no
clock. Everything is constructible and testable in memory.

The rules it holds itself to:

- Money is integer minor units — there is no `f64` in the crate
- Expected failures return `Result`; `panic!` is reserved for unreachable invariants
- Domain concepts get newtypes, validated in smart constructors
- Variants are enums matched exhaustively, with no `_` arms, so a new variant becomes a
  compile error rather than a silent fallthrough
- Illegal states are unconstructable where the type system can manage it — an unbalanced
  journal entry has no constructor that returns one

## Building it

Work the backlog in order — see [`backlog/BACKLOG.md`](backlog/BACKLOG.md). Each issue is
one Red → Green → Refactor cycle and one commit.

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

## Seeding the backlog into GitHub

```sh
git add -A && git commit -m "chore: scaffold workspace and backlog"
gh repo create adventurers-guild --private --source=. --remote=origin --push
./backlog/seed-github.sh --dry-run   # preview, touches nothing
./backlog/seed-github.sh             # 7 labels, 5 milestones, 27 issues
```

The script is re-runnable — it skips labels, milestones, and issues that already exist.

## Where this is going

This is the first of several bounded contexts sharing one world. The same adventurer will
appear in each of them under a different model: a **contractor** here, a **risk** to the
dungeon insurer, a **licensee** to the licensing board, a **soul** to the registry of the
dead. None of those models is wrong, and none of them is reconcilable with the others —
which is the whole lesson.

Nothing in this crate should assume those contexts exist. But `DepartureReason::Died`
should not be modelled in a way that makes them impossible later.
