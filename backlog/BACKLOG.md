# Backlog — the Guild domain core

27 issues, five milestones, dependency-ordered. Each one is a single
Red → Green → Refactor cycle, so each is a commit.

Full acceptance criteria live in `issues.json` (and in GitHub once seeded). This
file is the checklist for working offline.

**Seed them into GitHub:**

```sh
gh repo create adventurers-guild --private --source=. --remote=origin --push
./backlog/seed-github.sh --dry-run
./backlog/seed-github.sh
```

---

### M0 Foundations
- [ ] **chore: CI running fmt, clippy, and tests**
  `chore`
- [ ] **feat(ids): typed identifiers that cannot be swapped**
  `domain` `value-object`
- [ ] **feat(money): Coin as integer minor units**
  `domain` `value-object`
- [ ] **feat(money): allocate a purse without losing a copper**
  `domain` `value-object` `proptest`
- [ ] **feat(money): Share as an exact ratio**
  `domain` `value-object`
- [ ] **feat(time): WorldInstant and Duration**
  `domain` `value-object`

### M1 Ledger
- [ ] **feat(ledger): the Account sum type**
  `domain` `value-object`
- [ ] **feat(ledger): Posting and Direction**
  `domain` `value-object`
- [ ] **feat(ledger): a JournalEntry that cannot be unbalanced**
  `domain` `invariant`
- [ ] **feat(ledger): an append-only Ledger**
  `domain` `aggregate` `invariant` `proptest`
- [ ] **feat(ledger): corrections by reversal, never by edit**
  `domain` `invariant`
- [ ] **feat(ledger): bitemporal stamps on every entry**
  `domain` `invariant`

### M2 Quest & Escrow
- [ ] **feat(quest): HazardTier**
  `domain` `value-object`
- [ ] **feat(quest): Bounty and the escrow state machine**
  `domain` `invariant`
- [ ] **feat(quest): QuestState and its legal transitions**
  `domain` `invariant`
- [ ] **feat(quest): the Quest aggregate**
  `domain` `aggregate`
- [ ] **feat(quest): resolution outcomes**
  `domain` `value-object`

### M3 Party Over Time
- [ ] **feat(party): Enlistment as a half-open interval**
  `domain` `value-object` `invariant`
- [ ] **feat(party): DepartureReason**
  `domain` `value-object`
- [ ] **feat(party): the Party aggregate**
  `domain` `aggregate` `invariant` `proptest`
- [ ] **feat(party): membership queries over time**
  `domain` `aggregate`
- [ ] **feat(party): death and resurrection**
  `domain` `aggregate` `invariant`

### M4 Settlement
- [ ] **feat(settlement): the FeeSchedule trait**
  `domain`
- [ ] **feat(settlement): the SplitPolicy trait**
  `domain`
- [ ] **feat(settlement): settle() emits balanced entries**
  `domain` `invariant`
- [ ] **feat(settlement): estate payouts for the fallen**
  `domain`
- [ ] **test: the Bramblewick scenario, end to end**
  `domain` `invariant` `tdd`

---

## The three that carry the project

If time runs short, these are the ones worth finishing:

1. **A `JournalEntry` that cannot be unbalanced** (M1) — the invariant everything
   downstream leans on
2. **Death and resurrection** (M3) — the case that justifies modelling membership
   as intervals rather than a set
3. **The Bramblewick scenario** (M4) — the end-to-end proof, and the thing to show
   people

## Open questions to settle as you go

Decide these deliberately and record the reasoning in the module docs; they are
the interesting parts of the design, not incidental details.

- Is `Party` inside the `Quest` aggregate, or referenced by `PartyId`? (M2)
- Does time spent dead count toward a share? (M3)
- Signed or unsigned inner type for `Coin`? (M0, and do not revisit it in M1)
- Typestate for escrow, or `Result`-returning transitions? (M2)
