#!/usr/bin/env bash
# The Markdown's math uses no macro GitHub refuses to render ("The following macros are not
# allowed"). \operatorname is one: write \mathop{\mathrm{Re}} instead, which spaces the same.
set -euo pipefail
refused='\\operatorname'
if git grep -n -E "$refused" -- '*.md'; then
  echo "GitHub refuses \\operatorname in math: write \\mathop{\\mathrm{...}} instead"
  exit 1
fi
