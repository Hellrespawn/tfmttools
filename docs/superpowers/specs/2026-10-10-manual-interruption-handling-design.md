# Incremental history and manual interruption handling

Date: 2026-10-10
Status: Approved

## Implementation priorities

The purpose of this change is to **reduce complexity and increase consistency**.
These are acceptance criteria, not incidental benefits. Implement this as a
replacement and deletion of recovery machinery, not an additional layer around
the existing protocol.

Use one execution and history-recording flow for apply, undo, and redo. Use the
same reporting and explicit manual-resolution rules for ordinary execution
errors and process interruptions. Keep differences that belong to a particular
action, especially tag-edit backup handling, inside that action's module.

Prefer fewer states, smaller interfaces, and direct control flow. Do not preserve
old abstractions solely to minimize the diff. Do not recreate automatic recovery
through outcome classifiers, filesystem probes, reconciliation helpers, generic
backup managers, or elaborate cleanup tracking. A current-attempt marker is
durable reporting information, not the seed of a new recovery engine.

When a detail admits several implementations, choose the simplest one that
satisfies the agreed guarantees. Any proposed extra state, persisted field,
special execution path, or guarantee must justify its complexity against those
guarantees. If satisfying a detail would substantially rebuild the machinery
being removed, raise the tradeoff for design review rather than expanding scope.

Review the final implementation for deleted responsibilities as well as correct
behavior. Recovery entry points, remaining-plan persistence, expected-state
reconstruction, cleanup progress, obsolete interfaces, and their tests should
disappear where this design makes them unnecessary. Consistency means both
accurate confirmed history and uniform behavior across commands; it does not
mean inferring filesystem outcomes or promising automatic crash recovery.

## Outcome

Remove automatic recovery and continuation of interrupted commands. A later
session must never execute leftover actions, restore files, infer whether an
interrupted action succeeded, or reconcile filesystem state with history.
It reports the last attempted action and the confirmed history, leaving
filesystem recovery to the user.

Ordinary failures leave an undoable record of the actions confirmed as applied.
Tag edits retain their original until successful application has been durably
recorded. Backup handling belongs to tag editing, not a general recovery engine.

This design supersedes the durable operation progress, automatic recovery,
and finalization requirements of
[SQLite history and exact audio replay](2026-10-08-sqlite-exact-replay-design.md).
Its SQLite validation, locking, binary patch, stale-input rejection, and audio
verification contracts remain in force except where explicitly changed here.

## Terms and responsibilities

- A confirmed action has successfully returned from filesystem execution and
  has had its completion committed to history.
- An attempted action has durable intent but no confirmed completion. History
  makes no claim about its actual filesystem outcome.
- A partial run stopped before completing its requested actions. Its confirmed
  actions remain available for undo; its unattempted actions are discarded.
- Manual resolution records the user's account of an attempted action. It
  changes history only and does not inspect or change the affected files.

Core owns records, patches, current-attempt persistence, and history-only
resolution. Filesystem modules own action execution and useful error details.
The tag-edit module owns candidate and original-backup handling. The CLI reports
errors and interruptions and accepts the user's resolution.

The shared execution flow does not classify filesystem outcomes. It neither
queries paths to decide what happened nor branches into recovery strategies.

## Normal execution

Keep the session lock and execute actions in their planned order. Do not store
the remaining executable plan for use by another session.

For each action:

1. Prepare the action and any information needed for reporting and exact replay.
2. Commit a single current-attempt marker before applying filesystem effects.
3. Execute the action.
4. In one database transaction, record the confirmed action and its patches,
   update replay bookkeeping if applicable, and clear the attempt marker.
5. Perform action-specific post-confirmation housekeeping, then continue.

At most one action is attempted at a time. An action's history completion must
be committed before the next action starts. Preparation failures before intent
is committed leave no attempted action and must not change the original file.

Incrementally append confirmed actions to the run. Close the run as complete
only after all requested actions succeed. A durable open-run indicator lets a
later invocation identify interruption between actions, even with no surviving
attempt marker. That case has no uncertain action; close it as partial without
executing any filesystem work.

Redo supersession must take effect when the first new action is confirmed,
rather than waiting for the whole run to finish. A run with no applied actions
must not invalidate redo solely because it was started.

## Ordinary failures

Stop at the first error and do not attempt later actions. Preserve the confirmed
prefix as a partial run. Undo reverses only actions recorded as applied, in
reverse execution order, including staging moves and directory actions.

A failure before any filesystem execution can discard its intent immediately.
Once execution has been attempted, an error alone does not prove that nothing
changed. Retain its marker and report it for manual resolution; do not introduce
an outcome-classification protocol to distinguish clean and uncertain failures.

This means an ordinary command error can require resolving its final attempted
action before undo. After resolution, history accurately represents the user's
confirmed outcome and undo covers only the applied actions. Earlier confirmed
actions are never lost because a subsequent action failed.

Failure to commit completion after successful filesystem execution follows the
same manual-resolution path. Do not claim history is consistent with files when
the database could not record the result. An ambiguous database commit requires
reopening or reading committed database state before making history assertions;
it does not justify filesystem inspection or automatic repair.

Partial runs have a separate redo-eligibility flag. They are undoable but cannot
be redone after undo. Completeness and redo eligibility must not be encoded by
overloading the applied/undone/redone/superseded state.

## Interruption reporting and manual resolution

SQLite transactions protect the database's internal consistency. The expected
interruption problem is disagreement between recorded completion and filesystem
effects, not arbitrary database corruption. Existing invalid-database rejection
continues to apply; manual resolution does not repair corrupt SQLite files.

On the next invocation, report an open run or surviving attempt marker without
performing filesystem recovery. Include:

- Run identity, command metadata, and whether this was apply, undo, or redo.
- The confirmed history prefix and number of completed actions.
- The last attempted action, its direction, and affected source/target paths.
- For tag replacement, original, candidate, and retained-backup paths.
- A statement that completion is unconfirmed and no remaining work will run.
- Action-specific instructions describing what the user should inspect.

Reporting describes recorded intent, not inferred current file state. Do not
hash files, inspect existence, or reconstruct expected states to choose a
resolution. Repeated reporting is read-only and must not repeat any effect.

Provide an explicit history-only resolution operation for the identified attempt.
The user first checks or repairs the files manually, then chooses one of:

- Applied: record the attempted action as completed, using its saved patches
  and action information.
- Not applied: discard the attempted action's intent and unneeded patch data.

Both choices clear the attempt marker transactionally and close the interrupted
run as partial. Confirmed earlier actions remain recorded. If the actual state
matches neither choice, the user must finish manual filesystem repair before
resolving; tfmt offers no guessed or automatic third outcome. Resolution never
deletes a candidate or backup. Display enough information for the user to remove
these manually when appropriate.

The exact CLI spelling is an implementation choice. Resolution must identify
the attempt and require an explicit outcome; an unrelated confirmation prompt
or a new mutating command must not implicitly acknowledge it. Block new file
mutations, undo, redo, and clear-history while an attempt remains unresolved.
Read-only history and dry-run commands may report the interruption and continue
their existing read-only behavior, clearly noting that history is unresolved.

## Action-specific behavior

### Rename

Retain destination collision checks and same-filesystem atomic rename behavior.
An interruption can leave the file at its source or destination. Report both
paths and ask the user to establish which operation happened. Do not inspect
them on the user's behalf or automatically retry the move.

Cross-filesystem moves use copy followed by source deletion rather than claiming
atomic rename semantics. Existing staging moves remain ordinary ordered actions;
their recorded temporary paths must appear in interruption reports and undo.

### Copy and copy followed by deletion

A failed or interrupted copy leaves the source intact and may leave a partial
destination. Delete a source only after its destination copy is complete and
verified. Synchronize the destination and its directory entry before deleting
the source where required by the retained power-loss durability contract.

Report source, destination, and any copy candidate path. The user checks whether
copying completed and whether the source was removed. Do not automatically
delete either copy or finish source deletion in a later session.

When a compound operation stops between copying and deletion, the user restores
the not-applied state or completes the applied state manually before recording
the corresponding resolution. No shared outcome detector is needed.

### Tag edits and tag replay

Keep exact-byte forward/reverse patches, candidate verification, permissions
handling, symlink-target behavior, hard-link restrictions, and stale-input
checks from the existing design.

Prepare and verify a sibling candidate without changing the original. Save
patches and artifact paths in the durable attempt before the first file switch.
Retain the original as a sibling backup, install the candidate, and synchronize
the file changes. Delete the original backup only after history completion is
durably confirmed. The backup can be the original moved aside; an additional
full-file copy is not required. Portable staged replacement may briefly leave
the original pathname absent.

On failure or unconfirmed completion, preserve the original backup and useful
candidate artifacts. Report their paths for manual recovery. Do not implement
automatic restore, retry, or next-session installation. Preparation artifacts
created before a committed attempt remain locally owned and should be removed
on ordinary preparation failure; a crash during preparation can leave an orphan
candidate, which does not justify a global artifact-recovery mechanism.

Backup deletion after confirmed completion is housekeeping. If it fails, report
the leftover path and stop the command, preserving the confirmed action in
history. Do not reclassify it as unconfirmed, roll it back, or persist cleanup
progress. A crash in that interval may leave a backup without a pending marker;
automatic discovery and deletion of such backups are outside this design.

## Undo and redo failures

Use the same current-attempt protocol for apply, undo, and redo. Commit replay
progress after each confirmed action so a partial replay does not falsely mark
an entire record as undone or redone.

Keep the record's original action ordering and persist a replay cursor indicating
which prefix remains applied. Undo removes actions from the end of that prefix;
redo adds actions in original order. Cursor updates and attempt clearance are
one transaction. Record selection must account for partially undone records.

A later explicitly requested undo may continue undoing the applied prefix. This
is a new user-requested operation, not startup resumption. An interrupted replay
requires manual resolution of its attempted action before another replay starts.
Disable redo for records affected by failed or interrupted replay, keeping this
rule simple and conservative. Fully successful replay retains existing behavior.

## Storage and compatibility

Replace the durable remaining plan, per-action recovery descriptors, and
completed/cleaned progress journal with confirmed record actions, a replay
cursor, run completeness/redo eligibility, and at most one current-attempt
marker. The marker stores reporting data and any patches needed to confirm a
tag action later; it is not an executable recovery plan.

Keep durable intent-before-effect and completion transactions, session locking,
STRICT tables, foreign keys, validated payloads, database identity, and existing
connection durability settings. Never keep a SQLite transaction open across
filesystem effects.

Use a schema-version migration if the existing format has been published.
Preserve completed history and binary patches. Do not silently discard existing
pending operations or reinterpret them as completed. Refuse migration when old
pending work exists, with instructions to resolve it using the compatible
version first. Regenerate schema snapshots when implementing the change.

## Removed guarantees and mechanisms

- No automatic recovery, rollback, filesystem reconciliation, or continuation.
- No durable storage of unattempted actions for a later session.
- No reconstruction or verification of a whole run's expected filesystem state.
- No generalized filesystem outcome categories in shared execution.
- No durable artifact-cleanup progress or automatic orphan scanning.
- No all-or-nothing run guarantee or guarantee that every returned error leaves
  history synchronized with files without user intervention.

Exact-byte replay for confirmed actions, verified tag candidates, original
retention until tag completion is confirmed, and useful interruption reports
remain required. External programs can change files independently; reports and
manual resolution must not claim otherwise.

## Verification and documentation

Implementation tests must cover:

- Ordinary failure preserving an undoable confirmed prefix and disabling redo.
- Failures before execution versus attempts that return an error after starting.
- Interruption before and after intent, filesystem effects, completion commit,
  and tag-backup deletion; no invocation resumes or reverses file effects.
- Interruption between actions with an open run but no current attempt.
- Manual applied/not-applied resolution, transaction failure, and patch retention.
- Copy failure preserving the source and verified copy before source deletion.
- Tag-original retention until durable confirmation and cleanup-only errors.
- Partial undo/redo cursors, resolution direction, and correct record selection.
- Read-only reporting, dry runs, unresolved-attempt mutation blocking, and locks.
- Completed-history migration and refusal to migrate old pending operations.

The implementation review must also verify that the old recovery protocol has
been removed, shared execution does not infer filesystem outcomes, tag-backup
handling remains local to tag editing, and apply/undo/redo follow the same
attempt-and-confirm rules. Passing behavior tests alone does not satisfy the
complexity-reduction goal.

Update README, CHANGELOG, history documentation, relevant fixtures, and crate
guidance to remove automatic-recovery promises and describe manual resolution.
Replace automatic-recovery tests with these behavior tests. Run focused crate
tests followed by the repository's workspace test, clippy, and lint gates.
