#!/usr/bin/env bash
# The codebase report: the repository's size by area, now against the last release, and what the
# program's download is made of. CI's Codebase report workflow posts it on release-plz's open
# release PR, as one comment updated on every merge to main; run it yourself with
#
#   bash .github/codebase-report.sh            # or: just codebase-report
#
# It prints Markdown. Lines are the non-blank lines of the files git tracks (at the release tag
# and at HEAD, so local changes not committed don't count); a test file is one named
# `*_tests.rs` or `tests.rs`, or one in a `tests/` folder.
#
# Optional, from the environment:
#   CODEBASE_REPORT_BASE   the revision to compare with (default: the last tag v*)
#   CODEBASE_REPORT_DIST   the window's built front end (default: studio/dist; measured when it
#                          exists, so run `pnpm exec vite build` in studio/ first)
#   GITHUB_REPOSITORY      owner/name, for the releases' installers (default: tachsin/photonoxide)
#
# Needs git, coreutils, awk and curl; cargo for the crate's packaged size, gh for the installers'
# sizes, jq if there (a sed fallback reads crates.io's answer without it). Bash only, no Python
# (AGENTS.md).
set -euo pipefail
export LC_ALL=C

cd "$(git rev-parse --show-toplevel)"
base=${CODEBASE_REPORT_BASE:-$(git describe --tags --abbrev=0 --match 'v*')}
repo=${GITHUB_REPOSITORY:-tachsin/photonoxide}
dist=${CODEBASE_REPORT_DIST:-studio/dist}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

g() { git -c core.quotePath=false "$@"; }

# --- the files at a revision: path, bytes, non-blank lines -------------------------------------

files_at() { # REV OUT
  g ls-tree -r -l "$1" >"$tmp/tree"
  g grep -I -c -v -e '^[[:space:]]*$' "$1" -- . >"$tmp/grep" || true
  awk -v rev="$1:" '
    FNR == NR { s = substr($0, length(rev) + 1); i = match(s, /:[0-9]+$/)
                lines[substr(s, 1, i - 1)] = substr(s, i + 1); next }
    { t = index($0, "\t"); split(substr($0, 1, t - 1), a, " "); p = substr($0, t + 1)
      if (a[2] != "blob") next
      printf "%s\t%d\t%d\n", p, a[4], lines[p] + 0 }
  ' "$tmp/grep" "$tmp/tree" >"$2"
}
files_at "$base" "$tmp/base.tsv"
files_at HEAD "$tmp/head.tsv"

# the areas, in the table's order, and the functions every awk below shares
AREAS="src/fdtd|src/fdfd|src/mode|src/circuit|src/compact|src/geometry|src/job|src/material|src/thermal, electro, statics|src/export|src/, the rest|native/|python/|studio/src-tauri/src|studio/src|academy/|docs/|examples/|jobs/|site/|tests/|.github/|everything else"
LIB='
function area(p,   x) {
  if (p ~ /^src\/(fdtd|fdfd|mode|circuit|compact|geometry|job|material|export)(\/|\.rs$)/) {
    split(p, x, /[\/.]/); return "src/" x[2] }
  if (p ~ /^src\/(thermal|electro|statics)(\/|\.rs$)/) return "src/thermal, electro, statics"
  if (p ~ /^src\//) return "src/, the rest"
  if (p ~ /^studio\/src-tauri\/src\//) return "studio/src-tauri/src"
  if (p ~ /^studio\/src\//) return "studio/src"
  if (p ~ /^(native|python|academy|docs|examples|jobs|site|tests|\.github)\//) { split(p, x, "/"); return x[1] "/" }
  return "everything else"
}
function is_test(p) { return p ~ /(^|\/)tests\// || p ~ /(^|\/)tests\.rs$/ || p ~ /_tests\.rs$/ }
function n(x,   s, r, neg) { neg = x < 0; if (neg) x = -x; s = sprintf("%d", x); r = ""
  while (length(s) > 3) { r = "," substr(s, length(s) - 2) r; s = substr(s, 1, length(s) - 3) }
  return (neg ? "-" : "") s r }
function sn(x) { return (x > 0 ? "+" : "") n(x) }
function kib(b) { return b >= 1048576 ? sprintf("%.1f MiB", b / 1048576) : b >= 1024 ? sprintf("%.0f KiB", b / 1024) : sprintf("%d B", b) }
'

head_sha=$(git rev-parse --short HEAD)
echo "<!-- codebase-report -->"
echo "## Codebase report"
echo
echo "\`$base\` against \`$head_sha\` ($(git log -1 --format=%cs HEAD)). Lines are non-blank lines in the files git tracks; tests are the files named \`*_tests.rs\` or \`tests.rs\` and those in a \`tests/\` folder."
echo

# --- size by area ------------------------------------------------------------------------------

echo "### Size by area"
echo
awk -F'\t' -v order="$AREAS" "$LIB"'
  FNR == 1 { f++ }
  { a = area($1); t = is_test($1)
    if (f == 1) { bl[a] += $3; bl["T"] += $3 }
    else { l[a] += $3; nf[a]++; if (t) tl[a] += $3
           l["T"] += $3; nf["T"]++; if (t) tl["T"] += $3 } }
  END {
    print "| Area | Lines | of which tests | Files | Change since the release |"
    print "|---|--:|--:|--:|--:|"
    k = split(order, o, "|")
    for (i = 1; i <= k; i++) { a = o[i]
      if (!(a in l) && !(a in bl)) continue
      printf "| `%s` | %s | %s | %s | %s |\n", a, n(l[a]), (tl[a] ? n(tl[a]) : ""), n(nf[a]), sn(l[a] - bl[a]) }
    printf "| **All** | **%s** | **%s** | **%s** | **%s** |\n", n(l["T"]), n(tl["T"]), n(nf["T"]), sn(l["T"] - bl["T"])
  }' "$tmp/base.tsv" "$tmp/head.tsv"
echo

# the pie: today's lines, the areas under 3% together as "other"
echo '```mermaid'
echo "pie showData"
echo "  title Lines by area at $head_sha"
awk -F'\t' "$LIB"'
  { l[area($1)] += $3; T += $3 }
  END { for (a in l) if (a != "everything else" && l[a] >= 0.03 * T) printf "%d\t%s\n", l[a], a; else o += l[a]
        if (o) printf "%d\tother\n", o }' "$tmp/head.tsv" |
  sort -t"$(printf '\t')" -k1,1nr | awk -F'\t' '{ printf "  \"%s\" : %d\n", $2, $1 }'
echo '```'
echo

# --- the biggest changes -----------------------------------------------------------------------

echo "### The biggest changes since $base"
echo
echo "| Area | Lines added | Lines removed | Net |"
echo "|---|--:|--:|--:|"
g diff --numstat --no-renames "$base" HEAD |
  awk -F'\t' "$LIB"'
    $1 != "-" { a = area($3); ad[a] += $1; rm[a] += $2 }
    END { for (a in ad) printf "%d\t%d\t%s\n", ad[a], rm[a], a }' |
  sort -t"$(printf '\t')" -k1,1nr | head -n 10 |
  awk -F'\t' "$LIB"'{ printf "| `%s` | %s | %s | %s |\n", $3, sn($1), n(-$2), sn($1 - $2) }'
echo

# --- counts ------------------------------------------------------------------------------------

counts_at() { # REV: one count a line, in the table's order
  local r=$1
  g grep -h -c -e '^[[:space:]]*#\[test\]' "$r" -- '*.rs' | awk '{ s += $1 } END { print s + 0 }'
  { g show "$r:docs/validation.md" 2>/dev/null || true; } | grep -c '^| `' || true
  g ls-tree --name-only "$r" examples/ | grep -c '\.rs$' || true
  g ls-tree -r --name-only "$r" jobs/ | grep -cE '\.(ya?ml|toml)$' || true
  { g grep -L -e '^status: coming' "$r" -- 'academy/*.md' || true; } | grep -vc '/README\.md$' || true
  g ls-tree --name-only "$r" docs/methods/ | grep '\.md$' | grep -vc '/README\.md$' || true
  { g show "$r:Cargo.lock" 2>/dev/null || true; } | grep -c '^\[\[package\]\]' || true
}
counts_at "$base" >"$tmp/counts.base"
counts_at HEAD >"$tmp/counts.head"

echo "### Counts"
echo
echo "| | $base | now | Change |"
echo "|---|--:|--:|--:|"
printf '%s\n' "\`#[test]\` functions, in all the Rust" "validation cases (\`docs/validation.md\`'s rows)" \
  "examples in \`examples/\`" "jobs in \`jobs/\`" "Academy lessons written (not \`status: coming\`)" \
  "method write-ups in \`docs/methods/\`" "packages in \`Cargo.lock\`" >"$tmp/counts.what"
paste "$tmp/counts.what" "$tmp/counts.base" "$tmp/counts.head" |
  awk -F'\t' "$LIB"'{ printf "| %s | %s | %s | %s |\n", $1, n($2), n($3), sn($3 - $2) }'
echo

# --- the crate ---------------------------------------------------------------------------------

crate_now=""
if command -v cargo >/dev/null; then
  version=$(cargo pkgid -p photonoxide 2>/dev/null | sed 's/.*[#@]//') || true
  if [ -n "$version" ] && cargo package -p photonoxide --no-verify --allow-dirty --quiet >/dev/null 2>"$tmp/package.err"; then
    f="${CARGO_TARGET_DIR:-target}/package/photonoxide-$version.crate"
    if [ -f "$f" ]; then crate_now=$(wc -c <"$f" | tr -d ' '); fi
  fi
fi
crate_base=""
ua="photonoxide-codebase-report (https://github.com/$repo)"
if answer=$(curl -fsSL -A "$ua" "https://crates.io/api/v1/crates/photonoxide/${base#v}" 2>/dev/null); then
  if command -v jq >/dev/null; then
    crate_base=$(jq -r '.version.crate_size // empty' <<<"$answer")
  else
    crate_base=$(sed -n 's/.*"crate_size":\([0-9]*\).*/\1/p' <<<"$answer")
  fi
fi
size() { awk -v b="$1" "$LIB"'BEGIN { print kib(b) }'; }
if [ -n "$crate_base" ]; then at_base="$(size "$crate_base") at $base (crates.io)"; else at_base="not found at $base on crates.io"; fi
if [ -n "$crate_now" ]; then at_head="$(size "$crate_now") now"; else at_head="not measured now (cargo package failed, or no cargo)"; fi
echo "### The crate"
echo
echo "The \`photonoxide\` crate as published (\`cargo package\`, compressed): $at_base, $at_head."
echo

# --- the download ------------------------------------------------------------------------------

echo "### Download size"
echo
echo "#### The installers"
echo
# the latest two releases' installers, by their names less photonoxide-VERSION-
if command -v gh >/dev/null &&
  g_rel=$(gh api "repos/$repo/releases?per_page=10" --jq '[.[] | select(.draft | not) | select(.prerelease | not)][0:2][] | .tag_name as $t | .assets[] | [$t, .name, (.size | tostring)] | join("\t")' 2>/dev/null) &&
  [ -n "$g_rel" ]; then
  awk -F'\t' "$LIB"'
    $2 ~ /\.(json|sig)$/ { next }
    { if (!($1 in seen)) { seen[$1] = 1; tags[++k] = $1 }
      kind = $2; sub(/^photonoxide-[0-9][^-]*-/, "", kind)
      if (!(kind in size1) && !(kind in size2)) kinds[++m] = kind
      if ($1 == tags[1]) size1[kind] = $3; else size2[kind] = $3 }
    END {
      if (k > 1) { print "| Installer | " tags[1] " | " tags[2] " | Change |"; print "|---|--:|--:|--:|" }
      else { print "| Installer | " tags[1] " |"; print "|---|--:|" }
      for (i = 1; i <= m; i++) { c = kinds[i]
        a = (c in size1) ? kib(size1[c]) : ""; b = (c in size2) ? kib(size2[c]) : ""
        if (k > 1) printf "| `%s` | %s | %s | %s |\n", c, a, b, ((c in size1) && (c in size2)) ? sprintf("%+.1f MiB", (size1[c] - size2[c]) / 1048576) : ""
        else printf "| `%s` | %s |\n", c, a }
    }' <<<"$g_rel"
else
  echo "Not available here (needs gh, signed in)."
fi
echo

# what the program builds in: every include_str!, include_bytes! and include_dir! in the studio's
# and the library's code, outside test files and inline test modules, each target once
echo "#### What's built into the program"
echo
g ls-files -- 'studio/src-tauri/src/*.rs' 'src/*.rs' |
  awk "$LIB"'
    !is_test($0) { print }' >"$tmp/rs"
# shellcheck disable=SC2046
awk '
  function norm(p,   k, a, i, o, m) { k = split(p, a, "/"); m = 0
    for (i = 1; i <= k; i++) { if (a[i] == "" || a[i] == ".") continue
      if (a[i] == "..") { if (m) m--; continue } o[++m] = a[i] }
    p = ""; for (i = 1; i <= m; i++) p = p (i > 1 ? "/" : "") o[i]; return p }
  FNR == 1 { pending = 0; dir = FILENAME; sub(/[^\/]*$/, "", dir)
             manifest = (FILENAME ~ /^studio\/src-tauri\//) ? "studio/src-tauri/" : "" }
  pending && /[^[:space:]]/ { if ($0 ~ /^[[:space:]]*(pub )?mod [A-Za-z_]+ *\{/) { nextfile } pending = 0 }
  /^[[:space:]]*#\[cfg\(test\)\]/ { pending = 1; next }
  { s = $0
    while (match(s, /include_(str|bytes|dir)!\(/)) {
      s = substr(s, RSTART + RLENGTH); arg = s
      folder = (arg ~ /^[[:space:]]*concat!\(/)
      if (!match(arg, /"[^"]*"/)) break
      p = substr(arg, RSTART + 1, RLENGTH - 2); rest = substr(arg, RSTART + RLENGTH)
      suffix = ""
      if (folder && match(rest, /^[^;]*"\.[A-Za-z0-9]+"/)) { suffix = substr(rest, RSTART, RLENGTH); sub(/.*"\./, ".", suffix); sub(/"$/, "", suffix) }
      if (p ~ /^\$CARGO_MANIFEST_DIR/) { sub(/^\$CARGO_MANIFEST_DIR\/?/, "", p); p = manifest p } else p = dir p
      printf "%s\t%s\t%s\n", (folder || $0 ~ /include_dir!/) ? "folder" : "file", norm(p), suffix
    } }' $(cat "$tmp/rs") | sort -u >"$tmp/includes"

# each included file once, with its bytes: a folder's files (those with the macro's suffix), not
# its subfolders
while IFS=$'\t' read -r kind p suffix; do
  if [ "$kind" = folder ]; then
    g ls-files -- "$p" | awk -v pre="$p/" -v suf="$suffix" '
      { r = substr($0, length(pre) + 1) } index($0, pre) == 1 && r !~ /\// && (suf == "" || substr(r, length(r) - length(suf) + 1) == suf)'
  else
    echo "$p"
  fi
done <"$tmp/includes" | sort -u >"$tmp/included"
awk -F'\t' 'FNR == NR { want[$0] = 1; next } ($1 in want) { print }' "$tmp/included" "$tmp/head.tsv" >"$tmp/included.tsv"

echo "Each file once, uncompressed, grouped by folder, biggest first (text compresses several-fold in the installers):"
echo
echo "| Built in | Files | Size |"
echo "|---|--:|--:|"
awk -F'\t' "$LIB"'
  { d = $1; if (d ~ /\//) sub(/\/[^\/]*$/, "/", d); else d = "./"
    b[d] += $2; c[d]++; names[d] = names[d] (c[d] > 1 ? "`, `" : "") $1; N++ }
  END { for (d in b) printf "%d\t%s\t%d\n", b[d], (c[d] <= 2 ? names[d] : d), c[d]
        printf "%d\t**All**\t%d\n", -1, N }' "$tmp/included.tsv" |
  sort -t"$(printf '\t')" -k1,1nr >"$tmp/built"
total_built=$(awk -F'\t' '{ s += $2 } END { print s + 0 }' "$tmp/included.tsv")
awk -F'\t' -v total="$total_built" "$LIB"'
  $1 >= 0 { printf "| `%s` | %s | %s |\n", $2, n($3), kib($1) }
  $1 < 0 { printf "| **All** | **%s** | **%s** |\n", n($3), kib(total) }' "$tmp/built"
echo

echo "#### The window's front end"
echo
if [ -d "$dist" ]; then
  ( cd "$dist" && find . -type f -print | sed 's|^\./||' | sort | while IFS= read -r f; do
      printf '%s\t%s\t%s\n' "$f" "$(wc -c <"$f" | tr -d ' ')" "$(gzip -9c "$f" | wc -c | tr -d ' ')"
    done ) >"$tmp/dist.tsv"
  awk -F'\t' "$LIB"'
    { k = $1 ~ /\.js$/ ? "scripts" : $1 ~ /\.css$/ ? "styles" : $1 ~ /\.(woff2?|ttf|otf)$/ ? "fonts" : $1 ~ /\.(png|svg|jpe?g|ico|webp)$/ ? "images" : "other"
      b[k] += $2; z[k] += $3; c[k]++; B += $2; Z += $3 }
    END { printf "The bundle Tauri builds in (`%s`): %s, %s with gzip.\n\n", "'"$dist"'", kib(B), kib(Z)
      print "| Kind | Files | Size | gzip |"; print "|---|--:|--:|--:|"
      split("scripts styles fonts images other", o, " ")
      for (i = 1; i <= 5; i++) if (o[i] in b) printf "| %s | %s | %s | %s |\n", o[i], n(c[o[i]]), kib(b[o[i]]), kib(z[o[i]]) }' "$tmp/dist.tsv"
  echo
  # KaTeX ships each font as .woff2, .woff and .ttf; its CSS lists .woff2 first, which the
  # webviews Tauri uses (WebView2, WebKit) all read
  awk -F'\t' "$LIB"'$1 ~ /KaTeX_.*\.(woff|ttf)$/ { s += $2; c++ }
    END { if (c) printf "Of the fonts, %s files (%s) are KaTeX'"'"'s `.woff` and `.ttf` copies of the `.woff2` beside them, which its CSS lists first and every webview Tauri uses reads.\n\n", n(c), kib(s) }' "$tmp/dist.tsv"
  echo "The biggest files:"
  echo
  echo "| File | Size | gzip |"
  echo "|---|--:|--:|"
  sort -t"$(printf '\t')" -k2,2nr "$tmp/dist.tsv" | head -n 8 |
    awk -F'\t' "$LIB"'{ printf "| `%s` | %s | %s |\n", $1, kib($2), kib($3) }'
else
  echo "Not measured: \`$dist\` isn't built (run \`pnpm exec vite build\` in \`studio/\` first)."
fi
echo

# --- what could move out -----------------------------------------------------------------------

# the bytes of the files whose paths match an awk regex (through the environment, which keeps its
# backslashes), in a revision's files, and in the files built in
bytes_of() { # TSV REGEX
  re=$2 awk -F'\t' '$1 ~ ENVIRON["re"] { s += $2 } END { print s + 0 }' "$1"
}
academy_code='^studio/src-tauri/src/(academy\.rs|charts\.rs|charts/|diagrams\.rs|diagrams/)|^studio/src/(pages/Academy\.svelte|lib/academy\.svelte\.ts|components/Lesson[^/]*\.svelte|components/PaperTimeline\.svelte|components/diagrams/)'
inc() { re=$1 awk -F'\t' '$1 ~ ENVIRON["re"] { s += $2; c++ } END { printf "%d\t%d\n", s, c }' "$tmp/included.tsv"; }

echo "#### What could move out of the download"
echo
echo "Parts the program could fetch when wanted, with the bytes they build in (uncompressed):"
echo
echo "| Part | Built in | Files |"
echo "|---|--:|--:|"
{
  IFS=$'\t' read -r b c < <(inc '^academy/')
  printf 'A\tthe Academy'"'"'s lessons (`academy/`); its studio code is %s more source\t%d\t%d\n' \
    "$(awk -v b="$(bytes_of "$tmp/head.tsv" "$academy_code")" "$LIB"'BEGIN { print kib(b) }')" "$b" "$c"
  IFS=$'\t' read -r b c < <(inc '^docs/methods/')
  printf 'B\tthe method write-ups (`docs/methods/`)\t%d\t%d\n' "$b" "$c"
  IFS=$'\t' read -r b c < <(inc '^examples/')
  printf 'B\tthe examples'"'"' outputs and their README (`examples/`)\t%d\t%d\n' "$b" "$c"
  IFS=$'\t' read -r b c < <(inc '^(docs/validation\.md|CHANGELOG\.md)$')
  printf 'B\tthe validation report and the changelog\t%d\t%d\n' "$b" "$c"
  IFS=$'\t' read -r b c < <(inc '^jobs/')
  printf 'B\tthe built-in jobs (`jobs/`)\t%d\t%d\n' "$b" "$c"
} | sort -t"$(printf '\t')" -k1,1 -k3,3nr | awk -F'\t' "$LIB"'$3 > 0 { printf "| %s | %s | %s |\n", $2, kib($3), n($4) }'
echo

# --- the Academy's share -----------------------------------------------------------------------

echo "### The Academy's share"
echo
echo "The Academy's lessons (\`academy/\`) and the studio code serving them (\`academy.rs\`, \`charts*\`, \`diagrams*\` and the window's Academy components) against the whole studio (\`studio/\` with \`academy/\`), in bytes of source:"
echo
echo "| | $base | now |"
echo "|---|--:|--:|"
for r in base head; do
  t=$tmp/$r.tsv
  echo "$(bytes_of "$t" '^academy/') $(bytes_of "$t" "$academy_code") $(bytes_of "$t" '^(studio|academy)/') $(bytes_of "$t" '^studio/(src|src-tauri/src)/')"
done | awk "$LIB"'
  { L[NR] = $1; C[NR] = $2; S[NR] = $3; K[NR] = $4 }
  END {
    printf "| the lessons | %s | %s |\n", kib(L[1]), kib(L[2])
    printf "| the studio code serving them | %s | %s |\n", kib(C[1]), kib(C[2])
    printf "| the whole studio | %s | %s |\n", kib(S[1]), kib(S[2])
    printf "| **the Academy'"'"'s share** | **%.1f%%** | **%.1f%%** |\n", 100 * (L[1] + C[1]) / S[1], 100 * (L[2] + C[2]) / S[2]
    printf "| its code'"'"'s share of the studio'"'"'s code | %.1f%% | %.1f%% |\n", 100 * C[1] / K[1], 100 * C[2] / K[2]
  }'
