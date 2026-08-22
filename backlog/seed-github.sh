#!/usr/bin/env bash
#
# Seed the Adventurers' Guild backlog into GitHub Issues.
#
#   ./seed-github.sh --dry-run   # print what would happen; touches no network
#   ./seed-github.sh             # create labels, milestones, and issues
#
# Safe to re-run: labels and milestones that already exist are skipped, and
# issues already created by a previous run are recorded in .seeded and skipped.
# Delete .seeded to force a full re-seed (which WILL create duplicates).
#
# Requires: gh (authenticated), jq.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ISSUES_JSON="${SCRIPT_DIR}/issues.json"
SEEDED_LOG="${SCRIPT_DIR}/.seeded"

DRY_RUN=false
[[ "${1:-}" == "--dry-run" ]] && DRY_RUN=true

# label<TAB>colour<TAB>description
LABELS=$(
	cat <<-'EOF'
		domain	0e8a16	Domain layer: pure business rules
		value-object	1d76db	Immutable, self-validating, no identity
		aggregate	5319e7	Aggregate root and its invariants
		invariant	b60205	Enforces a rule that must never break
		proptest	fbca04	Carries a property-based test
		chore	c2e0c6	Tooling, build, CI
		tdd	d4c5f9	Test-first by construction
	EOF
)

MILESTONES=(
	"M0 Foundations"
	"M1 Ledger"
	"M2 Quest & Escrow"
	"M3 Party Over Time"
	"M4 Settlement"
)

die() {
	printf 'error: %s\n' "$1" >&2
	exit 1
}

note() { printf '  %s\n' "$1"; }

command -v jq >/dev/null || die "jq is required"
command -v gh >/dev/null || die "gh is required"
[[ -f "$ISSUES_JSON" ]] || die "not found: $ISSUES_JSON"
jq -e 'type == "array" and length > 0' "$ISSUES_JSON" >/dev/null ||
	die "issues.json is not a non-empty array"

if $DRY_RUN; then
	REPO="<current repo>"
	printf '\n=== DRY RUN — nothing will be created ===\n'
else
	gh auth status >/dev/null 2>&1 || die "gh is not authenticated; run: gh auth login"
	REPO=$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null) ||
		die "no GitHub repo detected. Create one first:
    gh repo create adventurers-guild --private --source=. --remote=origin --push"
fi

printf '\nRepository: %s\n' "$REPO"

# ---------------------------------------------------------------- labels ----
printf '\nLabels\n'
if $DRY_RUN; then
	existing_labels=""
else
	existing_labels=$(gh label list --limit 200 --json name --jq '.[].name')
fi

while IFS=$'\t' read -r name colour description; do
	[[ -z "$name" ]] && continue
	if grep -qxF "$name" <<<"$existing_labels"; then
		note "exists   $name"
	elif $DRY_RUN; then
		note "would create  $name (#$colour)"
	else
		gh label create "$name" --color "$colour" --description "$description" >/dev/null
		note "created  $name"
	fi
done <<<"$LABELS"

# ------------------------------------------------------------ milestones ----
printf '\nMilestones\n'
if $DRY_RUN; then
	existing_milestones=""
else
	existing_milestones=$(gh api --paginate "repos/${REPO}/milestones?state=all" --jq '.[].title')
fi

for milestone in "${MILESTONES[@]}"; do
	if grep -qxF "$milestone" <<<"$existing_milestones"; then
		note "exists   $milestone"
	elif $DRY_RUN; then
		note "would create  $milestone"
	else
		gh api "repos/${REPO}/milestones" -f title="$milestone" >/dev/null
		note "created  $milestone"
	fi
done

# ---------------------------------------------------------------- issues ----
printf '\nIssues\n'
touch "$SEEDED_LOG" 2>/dev/null || true

created=0
skipped=0
total=$(jq 'length' "$ISSUES_JSON")

for i in $(seq 0 $((total - 1))); do
	title=$(jq -r ".[$i].title" "$ISSUES_JSON")
	body=$(jq -r ".[$i].body" "$ISSUES_JSON")
	milestone=$(jq -r ".[$i].milestone" "$ISSUES_JSON")
	labels=$(jq -r ".[$i].labels | join(\",\")" "$ISSUES_JSON")
	number=$((i + 1))

	if [[ -f "$SEEDED_LOG" ]] && grep -qxF "$title" "$SEEDED_LOG"; then
		printf '  %2d. skip     %s\n' "$number" "$title"
		skipped=$((skipped + 1))
		continue
	fi

	if $DRY_RUN; then
		printf '  %2d. would create  %s\n' "$number" "$title"
		printf '      milestone: %s | labels: %s | body: %s lines\n' \
			"$milestone" "$labels" "$(wc -l <<<"$body")"
		created=$((created + 1))
		continue
	fi

	url=$(gh issue create \
		--title "$title" \
		--body "$body" \
		--milestone "$milestone" \
		--label "$labels")
	printf '  %2d. %s\n' "$number" "$url"
	printf '%s\n' "$title" >>"$SEEDED_LOG"
	created=$((created + 1))
done

printf '\n%s: %d created, %d skipped, %d total\n' \
	"$($DRY_RUN && echo 'Dry run' || echo 'Done')" "$created" "$skipped" "$total"

if ! $DRY_RUN; then
	printf '\nNext:\n  gh issue list --milestone %s\n' '"M0 Foundations"'
fi
