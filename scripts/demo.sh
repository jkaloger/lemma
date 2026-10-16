#!/usr/bin/env bash
# Fake `lemma lint`, `lemma plan <DELTA>` and `lemma build <DELTA>` for README
# screenshots. Mirrors the live-mode output of crates/lemma/src/lint.rs,
# plan.rs, build.rs, output.rs and runner.rs's spinner.
# Any other subcommand is passed through to the real lemma.
#
#   alias lemma="$PWD/scripts/demo.sh"
#   LEMMA_DEMO_PACE=2 lemma build DELTA-123   # slower playback
#   LEMMA_DEMO_HOLD=1 lemma build DELTA-123   # skip the collapse, keep every step on screen

set -euo pipefail
export LC_ALL=${LC_ALL:-en_US.UTF-8}

case ${1:-} in
  lint | plan | build) ;;
  *) exec "$(type -P lemma)" "$@" ;;
esac

DELTA=${2:-DELTA-123}
PACE=${LEMMA_DEMO_PACE:-1}
HOLD=${LEMMA_DEMO_HOLD:-}

if [[ -n ${NO_COLOR:-} ]]; then
  BOLD= GREEN= RED= DIM= RESET=
else
  BOLD=$'\e[1m' GREEN=$'\e[32m' RED=$'\e[31m' DIM=$'\e[2m' RESET=$'\e[0m'
fi

COLS=$(tput cols 2>/dev/null || echo 80)
SPIN=(⠁ ⠂ ⠄ ⡀ ⢀ ⠠ ⠐ ⠈)
rows=0

pause() { sleep "$(awk -v s="$1" -v p="$PACE" 'BEGIN { print s * p }')"; }

elapsed_text() {
  local secs=$1
  if ((secs >= 60)); then printf '%dm%02ds' $((secs / 60)) $((secs % 60)); else printf '%ds' "$secs"; fi
}

# Block::say: print a line and count the terminal rows it occupies.
say() {
  printf '%s\n' "$1"
  local plain
  plain=$(printf '%s' "$1" | sed $'s/\e\\[[0-9;]*m//g')
  local width=${#plain}
  rows=$((rows + (width + COLS - 1) / COLS))
  ((width == 0)) && rows=$((rows + 1))
  return 0
}

heading() { say "${BOLD}$1${RESET}"; }
pass()    { say "   ${GREEN}✓${RESET} $1"; }
fail()    { say "   ${RED}✗${RESET} $1"; }
info()    { say "   ${DIM}·${RESET} $1"; }

# runner::spinner with a followed transcript: `{spinner} <label> <elapsed> · <activity>` on stderr.
# Each argument after the label and total seconds is one tool call; the fake clock reaches `total` on the last frame.
run() {
  local label=$1 total=$2; shift 2
  local frames=$(($# * 12)) frame=0 secs activity line
  for activity in "$@"; do
    for _ in 1 2 3 4 5 6 7 8 9 10 11 12; do
      secs=$((total * frame / frames))
      line="$label $(elapsed_text "$secs") · $activity"
      ((${#line} > COLS - 2)) && line="${line:0:COLS-3}…"
      printf '\r\e[2K%s %s' "${SPIN[frame % 8]}" "$line" >&2
      frame=$((frame + 1))
      pause 0.1
    done
  done
  printf "\r\e[2K" >&2
}

commit() {
  pass "commit $1 $DELTA $2: $3"
}

# Block::finish: wipe the block and replace it with one summary line.
collapse() {
  local id=$1 title=$2 elapsed=$3 review=$4 sha=$5
  if [[ -n $HOLD ]]; then
    rows=0
    return
  fi
  pause 0.6
  printf '\e[%dA\e[J' "$rows"
  printf '%s %s %s  %s · %s · %s\n' "${GREEN}✓${RESET}" "$id" "$title" "$elapsed" "$review" "$sha"
  rows=0
}

next() { printf '%s %s\n' "${DIM}next:${RESET}" "$1"; }

if [[ $1 == lint ]]; then
  pause 0.4
  printf '%s\n' "docs/deltas/DELTA-098-transfers/claims.md:7: warning: test crates/ledger/tests/transfer.rs::moves_funds changed; DELTA-098/AC-2 claims it; confirm intended"
  exit 0
fi

if [[ $1 == plan ]]; then
  run "plan" 47 \
    "Read docs/deltas/$DELTA-balances/design.md" \
    "Read crates/ledger/src/account.rs" \
    "Grep fn transfer" \
    "Read crates/ledger/src/transfer.rs"
  printf '%s\n' "review clean, planning" "claims.md: 3 claims tagged AC-1..AC-3" "tasks.md: 3 tasks written"
  next "read the diff, accept or reject each [proposed] claim, then lemma build $DELTA"
  exit 0
fi

T1='add `Balance` type and the `SUM` query'
T2='reject a transfer that overdraws the account'
T3='document balances in the README'

heading "T1 $T1 (settles AC-1)"
run "T1 runner" 128 \
  "Read crates/ledger/src/account.rs" \
  "Grep fn sum_entries" \
  "Write crates/ledger/src/balance.rs" \
  "Edit crates/ledger/src/lib.rs" \
  "Bash cargo test -p ledger balance"
pass "runner: $(elapsed_text 128), 3 files"
pass "lint: clean"
pass "AC-1 @ref crates/ledger/tests/balance.rs#sums_every_entry resolves"
run "T1 review" 22 "Read crates/ledger/src/balance.rs" "Bash git diff HEAD"
pass "review: clean"
commit 3f1c9a2 T1 "$T1"
collapse T1 "$T1" 2m52s "review clean" 3f1c9a2

heading "T2 $T2 (settles AC-2, AC-3)"
run "T2 runner" 104 \
  "Read crates/ledger/src/transfer.rs" \
  "Edit crates/ledger/src/transfer.rs" \
  "Write crates/ledger/tests/overdraw.rs" \
  "Bash cargo test -p ledger overdraw"
pass "runner: $(elapsed_text 104), 2 files"
pass "lint: clean"
pass "AC-2 @ref crates/ledger/tests/overdraw.rs#rejects_overdraw resolves"
info "AC-3 [intent]"
run "T2 review" 31 "Read crates/ledger/src/transfer.rs" "Bash git diff HEAD"
fail "review: 1 finding"
info "fix round 1"
run "T2 fix" 41 \
  "Read crates/ledger/src/transfer.rs" \
  "Edit crates/ledger/src/transfer.rs" \
  "Bash cargo test -p ledger"
pass "fix: $(elapsed_text 41), 1 files"
pass "lint: clean"
pass "AC-2 @ref crates/ledger/tests/overdraw.rs#rejects_overdraw resolves"
info "AC-3 [intent]"
run "T2 review" 18 "Bash git diff HEAD"
pass "review: clean"
commit 8b07e4d T2 "$T2"
collapse T2 "$T2" 3m47s "1 finding fixed" 8b07e4d

heading "T3 $T3 (chore)"
run "T3 runner" 37 "Read README.md" "Edit README.md"
pass "runner: $(elapsed_text 37), 1 files"
pass "lint: clean"
run "T3 review" 14 "Bash git diff HEAD"
pass "review: clean"
commit c41d5e0 T3 "$T3"
collapse T3 "$T3" 58s "review clean" c41d5e0

printf 'all tasks complete\n'
