//! The journal: entries in the order they were written, and never in any
//! other.
//!
//! ```
//! use guild_domain::ledger::{Account, Balance, JournalEntry, Ledger, Posting, Stamps};
//! use guild_domain::money::Coin;
//! use guild_domain::time::{Duration, WorldInstant};
//!
//! let day = |n: u64| WorldInstant::from_seconds_since_founding(n * Duration::SECONDS_PER_DAY);
//! let patron = "lord-bramble".parse()?;
//! let mut ledger = Ledger::new();
//!
//! ledger.post(
//!     JournalEntry::new(
//!         vec![
//!             Posting::debit(Account::GuildVault, Coin::from_coppers(400_000))?,
//!             Posting::credit(Account::ClientEscrow(patron), Coin::from_coppers(400_000))?,
//!         ],
//!         "quest-1 funded".parse()?,
//!     )?,
//!     Stamps::new(day(3), day(3)),
//! )?;
//!
//! assert_eq!(
//!     ledger.balance(&Account::GuildVault)?,
//!     Balance::Debit(Coin::from_coppers(400_000)),
//! );
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Append-only, and what that is worth
//!
//! [`Ledger::post`] is the only method taking `&mut self`, and it only ever
//! pushes. Nothing removes an entry, replaces one, or hands out a `&mut` to
//! reach inside one — not for callers, and not for the tests in this file
//! either.
//!
//! That is an API-surface discipline rather than something a runtime assertion
//! can check: the guarantee is the *absence* of methods, and there is no test
//! that meaningfully asserts a method does not exist. What holds it is review,
//! and the fact that every later feature is designed not to want one.
//!
//! It is what makes the rest of M1 possible. Corrections work by posting a
//! reversal and refolding, which is only honest if the thing being corrected
//! is still there to read. Bitemporal queries ask what the books said at some
//! past moment, which is unanswerable if history can be rewritten. Both of
//! those are cheap here and would be impossible over a mutable store.
//!
//! # Correcting what is already written
//!
//! [`Ledger::reverse`] is the only correction there is. It appends an entry
//! mirroring the mistaken one, so the accounts come back to where they were
//! while both entries stay readable — the mistake and the admission of it.
//!
//! Two things are refused. A reversal cannot be reversed, which the type
//! system settles rather than the ledger: `reverse` is a method on
//! [`NormalEntry`](super::journal_entry::NormalEntry), and a reversal is not
//! one. And an entry cannot be reversed twice, which the ledger does have to
//! check, by looking for an existing correction with
//! [`Ledger::reversal_of`] — a second reversal would carry the accounts past
//! where they started, which is a worse state than the mistake.
//!
//! That check reads the journal instead of marking the entry it reverses.
//! Marking would mean editing something already written, which is the one
//! thing this module does not do, and it would put the truth in two places.
//!
//! # A balance is a fold, not a field
//!
//! [`Ledger::balance`] walks every posting naming an account and nets them.
//! Nothing is cached, and no running total is kept — the [account
//! module](super::account) sets out why at length, but the short version is
//! that a stored balance is a second source of truth, and a ledger with two
//! sources of truth has none.
//!
//! The cost is that `balance` is linear in the size of the journal. That is
//! the right trade at this scale, and if it ever stops being so the answer is
//! a projection built *outside* the domain, invalidated by appends — not a
//! field on the aggregate.
//!
//! # Where an id comes from
//!
//! An entry's [`EntryId`] is its position in the journal, minted by
//! [`EntryId::sequential`]. That keeps ids meaningful without a clock or a
//! random source, neither of which this crate has or wants.

use super::account::Account;
use super::balance::Balance;
use super::direction::Direction;
use super::journal_entry::{JournalEntry, LedgerError};
use super::narrative::Narrative;
use super::posting::Posting;
use super::stamps::Stamps;
use crate::identifiers::EntryId;
use crate::money::Coin;
use crate::time::WorldInstant;
use std::collections::HashSet;

/// One entry as the journal holds it: what was written, what it was named,
/// and the two moments it answers to.
///
/// The stamps live here rather than on [`JournalEntry`] for the same reason
/// the [`EntryId`] does. Both are minted when the entry is *filed*, and an
/// entry that has not been posted has no id and no `recorded_at` — there is
/// nothing yet to have recorded it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    id: EntryId,
    stamps: Stamps,
    entry: JournalEntry,
}

impl Record {
    /// What the journal named this entry.
    #[must_use]
    pub fn id(&self) -> &EntryId {
        &self.id
    }

    /// When it happened, and when the Guild found out.
    #[must_use]
    pub fn stamps(&self) -> Stamps {
        self.stamps
    }

    /// The entry itself.
    #[must_use]
    pub fn entry(&self) -> &JournalEntry {
        &self.entry
    }
}

/// An append-only journal of balanced entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    entries: Vec<Record>,
}

impl Ledger {
    /// An empty journal.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Appends `entry` under `stamps` and names it.
    ///
    /// The only way to change a ledger, and it only ever grows.
    ///
    /// Nothing here objects to the entry itself — one that exists has already
    /// proved it balances. The one thing that can be refused is *when it was
    /// filed*, which is a fact about the journal rather than about the entry.
    ///
    /// # Errors
    ///
    /// [`LedgerError::KnowledgeRunsBackwards`] if `stamps` were recorded
    /// earlier than an entry already in the journal. Knowledge accumulates:
    /// filing something behind what is already filed would mean the Guild
    /// un-learned it, and every as-known-on query afterwards would describe a
    /// journal that was never in that state.
    ///
    /// Equal moments are fine — one report from a returning party lands as
    /// several entries recorded in the same breath.
    ///
    /// This says nothing about [`occurred_at`](Stamps::occurred_at), which may
    /// fall wherever the truth puts it, before or after. Backdating is the
    /// case the Guild lives on.
    ///
    /// ```
    /// use guild_domain::ledger::{Account, JournalEntry, Ledger, Posting, Stamps};
    /// use guild_domain::money::Coin;
    /// use guild_domain::time::{Duration, WorldInstant};
    ///
    /// let day = |n: u64| WorldInstant::from_seconds_since_founding(n * Duration::SECONDS_PER_DAY);
    /// let patron = "lord-bramble".parse()?;
    /// let mut ledger = Ledger::new();
    ///
    /// let id = ledger.post(
    ///     JournalEntry::new(
    ///         vec![
    ///             Posting::debit(Account::GuildVault, Coin::from_coppers(400_000))?,
    ///             Posting::credit(Account::ClientEscrow(patron), Coin::from_coppers(400_000))?,
    ///         ],
    ///         "quest-1 funded".parse()?,
    ///     )?,
    ///     Stamps::new(day(12), day(21)),
    /// )?;
    ///
    /// assert!(ledger.entry(&id).is_some());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn post(&mut self, entry: JournalEntry, stamps: Stamps) -> Result<EntryId, LedgerError> {
        if let Some(latest) = self.latest_record()
            && stamps.recorded_at() < latest
        {
            return Err(LedgerError::KnowledgeRunsBackwards {
                recorded_at: stamps.recorded_at(),
                latest,
            });
        }
        let id = EntryId::sequential(self.entries.len() as u64 + 1);
        self.entries.push(Record {
            id: id.clone(),
            stamps,
            entry,
        });
        Ok(id)
    }

    /// The most recent moment anything was filed, or `None` while the journal
    /// is empty.
    ///
    /// The journal's own sense of "now": there is no clock in this crate, so
    /// what bounds a recording is the newest recording already in it.
    #[must_use]
    pub fn latest_record(&self) -> Option<WorldInstant> {
        self.entries
            .last()
            .map(|record| record.stamps.recorded_at())
    }

    /// The two moments the entry `id` names answers to.
    #[must_use]
    pub fn stamps(&self, id: &EntryId) -> Option<Stamps> {
        self.record(id).map(Record::stamps)
    }

    /// Finds the whole journal row `id` names — the entry, its stamps, and
    /// its name.
    #[must_use]
    pub fn record(&self, id: &EntryId) -> Option<&Record> {
        self.entries.iter().find(|record| &record.id == id)
    }

    /// Undoes the entry `original` names by appending its mirror image, and
    /// names the entry that does the undoing.
    ///
    /// A correction never edits. The mistaken entry stays exactly where it
    /// was, and a second entry mirroring every posting is written after it, so
    /// the books show both what was said and that it was taken back.
    ///
    /// # Errors
    ///
    /// - [`LedgerError::EntryNotFound`] if nothing in the journal answers to
    ///   `original`.
    /// - [`LedgerError::UnableToReverse`] if `original` names a reversal.
    ///   Undoing an undoing would say "the first entry stands after all",
    ///   which is written by posting that entry again, under its own
    ///   narrative, rather than by cancelling a correction the books have
    ///   already recorded.
    /// - [`LedgerError::AlreadyReversed`] if `original` has been reversed
    ///   before, naming the reversal that already exists. A second one would
    ///   carry the accounts past where they started.
    ///
    /// ```
    /// use guild_domain::ledger::{Account, Balance, JournalEntry, Ledger, Posting, Stamps};
    /// use guild_domain::money::Coin;
    /// use guild_domain::time::{Duration, WorldInstant};
    ///
    /// let day = |n: u64| WorldInstant::from_seconds_since_founding(n * Duration::SECONDS_PER_DAY);
    /// let patron = "lord-bramble".parse()?;
    /// let mut ledger = Ledger::new();
    ///
    /// let funded = ledger.post(
    ///     JournalEntry::new(
    ///         vec![
    ///             Posting::debit(Account::GuildVault, Coin::from_coppers(400_000))?,
    ///             Posting::credit(Account::ClientEscrow(patron), Coin::from_coppers(400_000))?,
    ///         ],
    ///         "quest-1 funded".parse()?,
    ///     )?,
    ///     Stamps::new(day(3), day(3)),
    /// )?;
    ///
    /// // Learned on day 9 that the quest never began, effective day 3.
    /// let undone = ledger.reverse(
    ///     funded.clone(),
    ///     "quest-1 never began".parse()?,
    ///     Stamps::new(day(3), day(9)),
    /// )?;
    ///
    /// // The vault is back where it started, and both entries are still there.
    /// assert_eq!(ledger.balance(&Account::GuildVault)?, Balance::Nil);
    /// assert_eq!(ledger.len(), 2);
    /// assert_eq!(ledger.entry(&undone).and_then(JournalEntry::reverses), Some(&funded));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn reverse(
        &mut self,
        original: EntryId,
        narrative: Narrative,
        stamps: Stamps,
    ) -> Result<EntryId, LedgerError> {
        let entry = self.entry(&original).ok_or(LedgerError::EntryNotFound {
            id: original.clone(),
        })?;
        let JournalEntry::Normal(normal) = entry else {
            return Err(LedgerError::UnableToReverse(original));
        };
        if let Some(reversal) = self.reversal_of(&original) {
            return Err(LedgerError::AlreadyReversed {
                original: original.clone(),
                reversal: reversal.clone(),
            });
        }
        let reversal: JournalEntry = normal.reverse(original, narrative).into();
        self.post(reversal, stamps)
    }

    /// Finds the entry that undoes the one `original` names, if one was
    /// posted.
    ///
    /// The question [`reverse`](Self::reverse) asks before it agrees to write
    /// a correction, and the one a caller asks to find out whether it still
    /// needs to. Answered by reading the journal rather than by a flag on the
    /// entry being reversed — the same argument the [account
    /// module](super::account) makes about balances, and for the same reason:
    /// a flag would be a second place the truth is kept, and an entry already
    /// written is not there to be flagged.
    #[must_use]
    pub fn reversal_of(&self, original: &EntryId) -> Option<&EntryId> {
        self.entries
            .iter()
            .find(|record| record.entry.reverses() == Some(original))
            .map(Record::id)
    }

    /// Finds the entry `id` names.
    #[must_use]
    pub fn entry(&self, id: &EntryId) -> Option<&JournalEntry> {
        self.record(id).map(Record::entry)
    }

    /// Nets everything posted to `account`.
    ///
    /// A fold over every posting naming that account, computed on demand and
    /// never stored — see the [account module](super::account) for why.
    ///
    /// # Errors
    ///
    /// [`LedgerError::SideOverflowed`] if one side of the account totals past
    /// `u64::MAX` coppers. Individually valid entries can add up to that, so
    /// it is a state the journal can genuinely reach rather than a theoretical
    /// one.
    pub fn balance(&self, account: &Account) -> Result<Balance, LedgerError> {
        let debits = self.total(account, Direction::Debit)?;
        let credits = self.total(account, Direction::Credit)?;
        Ok(Balance::netting(debits, credits))
    }

    /// Sums one side of one account across the whole journal.
    fn total(&self, account: &Account, side: Direction) -> Result<Coin, LedgerError> {
        self.postings()
            .filter(|posting| posting.account() == account && posting.direction() == side)
            .try_fold(Coin::ZERO, |sum, posting| sum.checked_add(posting.amount()))
            .map_err(|source| LedgerError::SideOverflowed { side, source })
    }

    /// Every posting in the journal, in the order they were written.
    fn postings(&self) -> impl Iterator<Item = &Posting> {
        self.entries
            .iter()
            .flat_map(|record| record.entry.postings())
    }

    /// Every account any entry has touched, once each, first-posted first.
    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        let mut seen: HashSet<&Account> = HashSet::new();
        self.postings()
            .map(Posting::account)
            .filter(move |account| seen.insert(account))
    }

    /// How many entries the journal holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing has been posted yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifiers::AdventurerId;
    use crate::ledger::Narrative;
    use crate::time::Duration;
    use proptest::prelude::*;

    fn adventurer(name: &str) -> AdventurerId {
        name.parse().expect("a well-formed id")
    }

    fn narrative() -> Narrative {
        "settlement of quest-1".parse().expect("a narrative")
    }

    fn correction() -> Narrative {
        "quest-1 abandoned".parse().expect("a narrative")
    }

    fn day(count: u64) -> WorldInstant {
        WorldInstant::from_seconds_since_founding(count * Duration::SECONDS_PER_DAY)
    }

    /// Stamps for a test that is not about time. Every moment is the
    /// Founding, which no later recording can run behind.
    fn whenever() -> Stamps {
        Stamps::new(WorldInstant::FOUNDING, WorldInstant::FOUNDING)
    }

    /// Posts `entry` at a moment the journal will always accept, for the
    /// tests that care what a balance comes to rather than when it was known.
    fn post(ledger: &mut Ledger, entry: JournalEntry) -> EntryId {
        ledger
            .post(entry, whenever())
            .expect("stamps that do not run backwards")
    }

    /// Reverses `original` at a moment the journal will always accept.
    fn reverse(ledger: &mut Ledger, original: EntryId) -> Result<EntryId, LedgerError> {
        ledger.reverse(original, correction(), whenever())
    }

    fn entry(postings: Vec<Posting>) -> JournalEntry {
        JournalEntry::new(postings, narrative()).expect("a balanced entry")
    }

    fn debit(account: Account, coppers: u64) -> Posting {
        Posting::debit(account, Coin::from_coppers(coppers)).expect("a posting of something")
    }

    fn credit(account: Account, coppers: u64) -> Posting {
        Posting::credit(account, Coin::from_coppers(coppers)).expect("a posting of something")
    }

    fn patron() -> Account {
        Account::ClientEscrow(adventurer("lord-bramble"))
    }

    /// A funding entry: coin into the vault, and the debt to show for it.
    fn funding(coppers: u64) -> JournalEntry {
        entry(vec![
            debit(Account::GuildVault, coppers),
            credit(patron(), coppers),
        ])
    }

    #[test]
    fn should_start_empty() {
        let ledger = Ledger::new();

        assert!(ledger.is_empty());
        assert_eq!(ledger.len(), 0);
    }

    #[test]
    fn should_hand_back_an_id_that_finds_the_entry_again() {
        let mut ledger = Ledger::new();
        let posted = funding(400_000);

        let id = post(&mut ledger, posted.clone());

        assert_eq!(ledger.entry(&id), Some(&posted));
    }

    #[test]
    fn should_name_every_entry_differently() {
        // Two identical entries are still two entries. Were the ids to
        // collide, a reversal in M1 would undo the wrong one.
        let mut ledger = Ledger::new();

        let first = post(&mut ledger, funding(400_000));
        let second = post(&mut ledger, funding(400_000));

        assert_ne!(first, second);
        assert_eq!(ledger.len(), 2);
    }

    #[test]
    fn should_stamp_an_entry_with_when_it_happened_and_when_it_was_learned() {
        let mut ledger = Ledger::new();

        let id = ledger
            .post(funding(400_000), Stamps::new(day(12), day(21)))
            .expect("stamps that do not run backwards");

        assert_eq!(ledger.stamps(&id), Some(Stamps::new(day(12), day(21))));
    }

    #[test]
    fn should_refuse_an_entry_recorded_before_the_journal_already_knew() {
        // Knowledge only accumulates. An entry written into the journal
        // *earlier* than one already in it would mean the Guild un-learned
        // something, and every as-known-on query after it would be answering
        // about a journal that was never in that state.
        //
        // The entry being backdated is fine and is the whole point — it is
        // the *recording* that may not go backwards.
        let mut ledger = Ledger::new();
        ledger
            .post(funding(400_000), Stamps::new(day(12), day(21)))
            .expect("stamps that do not run backwards");

        let refused = ledger.post(funding(1_000), Stamps::new(day(1), day(20)));

        assert_eq!(
            refused,
            Err(LedgerError::KnowledgeRunsBackwards {
                recorded_at: day(20),
                latest: day(21),
            })
        );
    }

    #[test]
    fn should_accept_a_second_entry_recorded_at_the_very_same_moment() {
        // Two things learned in the same breath. The rule is that knowledge
        // does not go backwards, not that it strictly advances — a settlement
        // posts several entries from one report.
        let mut ledger = Ledger::new();
        ledger
            .post(funding(400_000), Stamps::new(day(12), day(21)))
            .expect("stamps that do not run backwards");

        let second = ledger.post(funding(1_000), Stamps::new(day(3), day(21)));

        assert!(second.is_ok());
    }

    #[test]
    fn should_find_nothing_for_an_id_it_never_minted() {
        let ledger = Ledger::new();

        assert_eq!(ledger.entry(&EntryId::sequential(1)), None);
    }

    #[test]
    fn should_report_nil_for_an_account_nothing_was_posted_to() {
        let ledger = Ledger::new();

        assert_eq!(ledger.balance(&Account::GuildFeeIncome), Ok(Balance::Nil));
    }

    #[test]
    fn should_net_an_accounts_postings_across_every_entry() {
        // The vault takes 400,000 in and pays 60,000 back out. The balance is
        // a fold over both entries, not a number either one of them holds.
        let mut ledger = Ledger::new();
        post(&mut ledger, funding(400_000));
        post(
            &mut ledger,
            entry(vec![
                debit(Account::GuildFeeIncome, 60_000),
                credit(Account::GuildVault, 60_000),
            ]),
        );

        assert_eq!(
            ledger.balance(&Account::GuildVault),
            Ok(Balance::Debit(Coin::from_coppers(340_000)))
        );
    }

    #[test]
    fn should_list_each_account_once_however_often_it_was_posted_to() {
        let mut ledger = Ledger::new();
        post(&mut ledger, funding(400_000));
        post(&mut ledger, funding(1_000));

        let accounts: Vec<&Account> = ledger.accounts().collect();

        assert_eq!(accounts, vec![&Account::GuildVault, &patron()]);
    }

    #[test]
    fn should_refuse_a_balance_that_totals_more_than_a_purse_can_hold() {
        // Two entries of u64::MAX each are both individually valid, and
        // together they push one account past what a Coin can count. Reported
        // rather than wrapped, for the same reason the entry reports it: a
        // ledger that silently rolls over is worse than one that stops.
        let mut ledger = Ledger::new();
        post(&mut ledger, funding(u64::MAX));
        post(&mut ledger, funding(u64::MAX));

        let refused = ledger
            .balance(&Account::GuildVault)
            .expect_err("a debit side past u64::MAX");

        assert!(matches!(
            refused,
            LedgerError::SideOverflowed {
                side: Direction::Debit,
                ..
            }
        ));
    }

    #[test]
    fn should_undo_the_original_entrys_effect_on_every_balance() {
        // What a reversal is for. Both accounts the funding touched are back
        // where they started, and neither is back there because anything was
        // erased — the journal now holds two entries that cancel.
        let mut ledger = Ledger::new();
        let id = post(&mut ledger, funding(400_000));

        reverse(&mut ledger, id).expect("a normal entry to reverse");

        assert_eq!(ledger.balance(&Account::GuildVault), Ok(Balance::Nil));
        assert_eq!(ledger.balance(&patron()), Ok(Balance::Nil));
    }

    #[test]
    fn should_restore_prior_balances_when_an_entry_is_reversed() {
        // The acceptance criterion, stated as it is written: *prior*, not
        // nil. The vault is carrying an earlier entry that the correction has
        // no business touching, so a reversal that simply emptied the account
        // would pass a nil assertion and still be wrong.
        let mut ledger = Ledger::new();
        post(&mut ledger, funding(1_000));
        let prior = ledger.balance(&Account::GuildVault);
        let mistake = post(&mut ledger, funding(400_000));

        reverse(&mut ledger, mistake).expect("a normal entry to reverse");

        assert_eq!(ledger.balance(&Account::GuildVault), prior);
        assert_eq!(
            ledger.balance(&Account::GuildVault),
            Ok(Balance::Debit(Coin::from_coppers(1_000)))
        );
    }

    #[test]
    fn should_find_the_reversal_that_undoes_an_entry() {
        let mut ledger = Ledger::new();
        let id = post(&mut ledger, funding(400_000));

        let reversal = reverse(&mut ledger, id.clone()).expect("a normal entry to reverse");

        assert_eq!(ledger.reversal_of(&id), Some(&reversal));
    }

    #[test]
    fn should_find_no_reversal_for_an_entry_that_still_stands() {
        // The other half of the question `reverse` asks itself. An entry that
        // has not been corrected must not look corrected, or every reversal
        // after the first one in the journal would be refused.
        let mut ledger = Ledger::new();
        let id = post(&mut ledger, funding(400_000));
        post(&mut ledger, funding(1_000));

        assert_eq!(ledger.reversal_of(&id), None);
    }

    #[test]
    fn should_leave_the_entry_it_reverses_in_the_journal() {
        // The append-only rule, at the one place there would be a temptation
        // to break it. Bitemporal queries in M1 ask what the books said at a
        // past moment, which is unanswerable if a correction removes its
        // original.
        let mut ledger = Ledger::new();
        let posted = funding(400_000);
        let id = post(&mut ledger, posted.clone());

        reverse(&mut ledger, id.clone()).expect("a normal entry to reverse");

        assert_eq!(ledger.entry(&id), Some(&posted));
        assert_eq!(ledger.len(), 2);
    }

    #[test]
    fn should_name_the_reversal_with_an_id_of_its_own() {
        // The reversal is an entry like any other: it is found by its own id,
        // and it carries the narrative the caller gave for making it.
        let mut ledger = Ledger::new();
        let id = post(&mut ledger, funding(400_000));

        let reversal = reverse(&mut ledger, id.clone()).expect("a normal entry to reverse");

        assert_ne!(reversal, id);
        assert_eq!(
            ledger
                .entry(&reversal)
                .map(|entry| entry.narrative().as_str()),
            Some("reversal of entry-1: quest-1 abandoned")
        );
    }

    #[test]
    fn should_refuse_to_reverse_an_entry_it_never_minted() {
        let mut ledger = Ledger::new();

        let refused = reverse(&mut ledger, EntryId::sequential(1));

        assert_eq!(
            refused,
            Err(LedgerError::EntryNotFound {
                id: EntryId::sequential(1)
            })
        );
    }

    #[test]
    fn should_refuse_to_reverse_an_entry_that_was_already_reversed() {
        // Twice undone is worse than not undone at all: the second reversal
        // carries the accounts past where they started, and the journal ends
        // up holding two corrections for one mistake.
        //
        // The error names the reversal that already exists, because the
        // caller asking for this is one who has lost track of it — telling
        // them only "no" leaves them no way to go and look.
        let mut ledger = Ledger::new();
        let id = post(&mut ledger, funding(400_000));
        let reversal = reverse(&mut ledger, id.clone()).expect("a normal entry to reverse");

        let refused = reverse(&mut ledger, id.clone());

        assert_eq!(
            refused,
            Err(LedgerError::AlreadyReversed {
                original: id,
                reversal
            })
        );
    }

    #[test]
    fn should_refuse_to_reverse_a_reversal() {
        // Otherwise a reversal could be reversed back into the original, and
        // "what does this entry correct" stops having one answer. A reversal
        // posted in error is corrected by posting the original again, which
        // reads honestly in the journal.
        let mut ledger = Ledger::new();
        let id = post(&mut ledger, funding(400_000));
        let reversal = reverse(&mut ledger, id).expect("a normal entry to reverse");

        let refused = reverse(&mut ledger, reversal.clone());

        assert_eq!(refused, Err(LedgerError::UnableToReverse(reversal)));
    }

    #[test]
    fn should_append_nothing_when_it_refuses_to_reverse() {
        // A refusal that had already pushed an entry would leave the journal
        // holding half a correction.
        let mut ledger = Ledger::new();
        let id = post(&mut ledger, funding(400_000));
        let reversal = reverse(&mut ledger, id).expect("a normal entry to reverse");

        let _ = reverse(&mut ledger, reversal);
        let _ = reverse(&mut ledger, EntryId::sequential(99));

        assert_eq!(ledger.len(), 2);
    }

    #[test]
    fn should_reverse_each_of_several_entries_independently() {
        // Reversing the first entry must undo the first entry, not the last
        // one posted or the one the id happens to sit next to.
        let mut ledger = Ledger::new();
        let first = post(&mut ledger, funding(400_000));
        post(&mut ledger, funding(1_000));

        reverse(&mut ledger, first).expect("a normal entry to reverse");

        assert_eq!(
            ledger.balance(&Account::GuildVault),
            Ok(Balance::Debit(Coin::from_coppers(1_000)))
        );
    }

    /// The five accounts the properties draw from.
    ///
    /// Deliberately small, so that entries collide on accounts and the
    /// balances being netted are folds over several entries rather than one.
    fn pool() -> Vec<Account> {
        vec![
            Account::GuildVault,
            Account::GuildFeeIncome,
            patron(),
            Account::AdventurerPayable(adventurer("bramblewick-thorne")),
            Account::EstatePayable(adventurer("alder-quill")),
        ]
    }

    /// One balanced entry, as indices into [`pool`] plus amounts: a single
    /// debit discharged across one or more credits.
    fn entry_parts() -> impl Strategy<Value = (usize, Vec<(usize, u64)>)> {
        (
            0_usize..5,
            proptest::collection::vec((0_usize..5, 1_u64..100_000), 1..5),
        )
    }

    fn build(parts: &(usize, Vec<(usize, u64)>), pool: &[Account]) -> JournalEntry {
        let total: u64 = parts.1.iter().map(|(_, amount)| amount).sum();
        let mut postings = vec![debit(pool[parts.0].clone(), total)];
        postings.extend(
            parts
                .1
                .iter()
                .map(|(index, amount)| credit(pool[*index].clone(), *amount)),
        );
        entry(postings)
    }

    proptest! {
        /// However many entries are posted, the journal's debits still equal
        /// its credits.
        ///
        /// This is the invariant `JournalEntry` establishes, held at the scale
        /// where a bug in `post` would break it — dropping a posting, or
        /// storing an entry other than the one it was handed.
        #[test]
        fn should_balance_debits_against_credits_after_any_sequence_of_posts(
            parts in proptest::collection::vec(entry_parts(), 0..8)
        ) {
            let pool = pool();
            let mut ledger = Ledger::new();
            let ids: Vec<EntryId> = parts
                .iter()
                .map(|part| post(&mut ledger, build(part, &pool)))
                .collect();

            let mut debits: u128 = 0;
            let mut credits: u128 = 0;
            for id in &ids {
                let entry = ledger.entry(id).expect("an entry just posted");
                for posting in entry.postings() {
                    let amount = u128::from(posting.amount().as_coppers());
                    match posting.direction() {
                        Direction::Debit => debits += amount,
                        Direction::Credit => credits += amount,
                    }
                }
            }

            prop_assert_eq!(debits, credits);
        }

        /// The trial balance across every account comes to zero.
        ///
        /// The property the issue calls out as the one that will catch a bug
        /// in `settle()` three milestones from now.
        ///
        /// What it adds over the raw sum above is asymmetry between accounts:
        /// stop `accounts` deduplicating and this fails while the raw sum does
        /// not, because one account gets counted twice on one side only. That
        /// is the shape a settlement bug takes — one member's share landing
        /// somewhere the others' do not.
        ///
        /// It is worth being just as precise about what it does *not* see,
        /// since a property that looks total and is not is worse than one
        /// whose limits are written down. It compares two sums, so any fault
        /// that mistreats both sides alike survives it: netting every account
        /// backwards swaps the totals and leaves them equal, and a `balance`
        /// that ignored which account a posting named would report nil
        /// everywhere, which also sums to zero. Both of those are caught by
        /// the examples above rather than here.
        #[test]
        fn should_leave_a_trial_balance_of_zero_after_any_sequence_of_posts(
            parts in proptest::collection::vec(entry_parts(), 0..8)
        ) {
            let pool = pool();
            let mut ledger = Ledger::new();
            for part in &parts {
                post(&mut ledger, build(part, &pool));
            }

            let mut debits: u128 = 0;
            let mut credits: u128 = 0;
            for account in ledger.accounts() {
                match ledger.balance(account).expect("amounts far below u64::MAX") {
                    Balance::Nil => {}
                    Balance::Debit(amount) => debits += u128::from(amount.as_coppers()),
                    Balance::Credit(amount) => credits += u128::from(amount.as_coppers()),
                }
            }

            prop_assert_eq!(debits, credits);
        }

        /// Reversing every entry in a journal empties every account.
        ///
        /// The end-to-end claim the whole reversal path exists to support,
        /// held at the scale where the pieces meet: `Posting::reverse` flips a
        /// side, `NormalEntry::reverse` flips all of them, `Ledger::reverse`
        /// finds the right entry and appends. A fault in any one of those
        /// leaves coin stranded in some account here.
        ///
        /// It is the counterpart to
        /// `should_leave_a_trial_balance_of_zero_after_any_sequence_of_posts`
        /// and sees what that one cannot: a trial balance of zero says the two
        /// sides agree, while nil in every account says nothing is left at all.
        #[test]
        fn should_empty_every_account_when_every_entry_is_reversed(
            parts in proptest::collection::vec(entry_parts(), 0..8)
        ) {
            let pool = pool();
            let mut ledger = Ledger::new();
            let ids: Vec<EntryId> = parts
                .iter()
                .map(|part| post(&mut ledger, build(part, &pool)))
                .collect();

            for id in ids {
                ledger
                    .reverse(id, narrative(), whenever())
                    .expect("a normal entry to reverse");
            }

            for account in ledger.accounts() {
                prop_assert_eq!(ledger.balance(account), Ok(Balance::Nil));
            }
        }

    }
}
