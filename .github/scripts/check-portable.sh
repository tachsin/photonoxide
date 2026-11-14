#!/usr/bin/env bash
# The same numbers on every system (#822; CONTRIBUTING.md, "The same numbers on every system"):
# what clippy.toml's ban on the system's maths functions can't check by itself.
#
# 1. The allows ratchet: the lines that mention clippy::disallowed_methods in the Rust sources
#    (a test that takes the system's functions as an independent reference) number exactly what
#    .github/portable-allows holds. More
#    fails: convert the code instead. Fewer fails too, with the number to write there, so that the
#    count only ever goes down.
# 2. `libm::` only inside src/portable/: everything else calls the portable functions.
# 3. No `mul_add` in src/: fused in hardware or in software, whichever the build has.
#
# Run from the repository's root, by CI's Lint job and the pre-push hook (.github/lints.sh).
set -euo pipefail

status=0

stored=$(grep -v '^#' .github/portable-allows | tr -d '[:space:]')
count=$( { git grep -h -c 'clippy::disallowed_methods' -- '*.rs' || true; } | awk '{ n += $1 } END { print n + 0 }')
if [ "$count" -gt "$stored" ]; then
  echo "check-portable: $count lines mention clippy::disallowed_methods, more than the $stored in .github/portable-allows:"
  git grep -n 'clippy::disallowed_methods' -- '*.rs'
  echo "take the functions from photonoxide::portable instead of allowing the system's"
  status=1
elif [ "$count" -lt "$stored" ]; then
  echo "check-portable: $count lines mention clippy::disallowed_methods, fewer than the $stored in .github/portable-allows: write $count there"
  status=1
else
  echo "check-portable: $count allows of the system's maths functions (.github/portable-allows)"
fi

if git grep -n 'libm::' -- '*.rs' ':!src/portable/'; then
  echo "check-portable: libm is called outside src/portable/: call photonoxide::portable's functions"
  status=1
fi

if git grep -n 'mul_add' -- 'src/'; then
  echo "check-portable: mul_add in src/: write a * b + c, rounded twice, the same everywhere"
  status=1
fi

exit "$status"
