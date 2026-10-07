#!/usr/bin/env bash
# Every relative link in the repository's Markdown points at a file or folder that exists.
# Links to the web, to anchors on the same page and to mail are not checked here.
set -euo pipefail
status=0
while IFS= read -r -d '' file; do
  dir=$(dirname "$file")
  # [text](target) and [text](target "title"); the target up to a space, ')' or '#'
  while IFS= read -r target; do
    case "$target" in
      ''|http://*|https://*|mailto:*|\#*) continue ;;
    esac
    path="${target%%#*}"
    [ -z "$path" ] && continue
    if [ "${path#/}" != "$path" ]; then
      resolved=".${path}"
    else
      resolved="$dir/$path"
    fi
    if [ ! -e "$resolved" ]; then
      echo "$file: broken link to $target"
      status=1
    fi
  done < <(grep -oE '\]\([^) ]+' "$file" | sed 's/^](//' || true)
done < <(find . -name '*.md' -not -path './target/*' -not -path '*/node_modules/*' -not -path './.git/*' -print0)
exit "$status"
