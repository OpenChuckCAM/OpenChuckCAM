#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""The safe-Z diff check of decision record 0649.

For every golden file changed between two commits, compares the old and new
text line by line. Only these may change:

- a Z word of 40.000 that becomes 45.000 (the built-in Makera Z1's default
  safe Z and safe cylinder radius, 40 -> 45), the rest of the line identical;
- the estimate in an operation's comment, "(operation ..., about N s)";
- in a setup sheet (.html, .txt), the lines that state the safe Z, the safe
  cylinder radius, the expected work offset or the programmed time;
- a rounding tie of the layout shift (decision 0649, decision 9): the same
  words, one X word 0.001 mm apart, and the F word that follows from it;
- in the first-cut programs only, a rapid link height between roughing
  layers ("G0 Z..."), which depends on the 45 mm safe cylinder (decision 2):
  printed with its old and new value for review.

Prints, per file, the number of lines changed of each kind, and exits non-zero
if any other line changed or the line counts differ.

Usage: python3 zdiff.py <old-commit> [<new-commit>]   (new defaults to the
working tree). Run from the repository root.
"""
import re
import subprocess
import sys

ESTIMATE = re.compile(r", about [0-9]+ s\)$")
WORD = re.compile(r"([A-Z])(-?[0-9.]+)")
LINK = re.compile(r"^G0 Z(-?[0-9.]+)$")
LINK_FILES = ("first_cut/",)


def tie(old, new):
    """The same words, with X 0.001 mm apart at most and any F changed. X is
    modal, so a tie can also make the post write an X word it left out (or
    leave out one it wrote): the other words must still match."""
    a, b = WORD.findall(old), WORD.findall(new)
    if [k for k, _ in a] != [k for k, _ in b]:
        strip = lambda w: [(k, v) for k, v in w if k not in "XF"]
        return strip(a) == strip(b) and any(k == "X" for k, _ in a + b)
    if not any(k == "X" for k, _ in a):
        return False
    for (k, x), (_, y) in zip(a, b):
        if k == "F":
            continue
        if k == "X" and abs(float(x) - float(y)) <= 0.0010001:
            continue
        if x != y:
            return False
    return True
Z_OLD = re.compile(r"(?<![0-9.])Z40\.000(?![0-9])")


def show(commit, path):
    if commit is None:
        with open(path, encoding="utf-8") as f:
            return f.read()
    return subprocess.run(
        ["git", "show", f"{commit}:{path}"], check=True, capture_output=True, text=True
    ).stdout


def kind(old, new, sheet, path=""):
    if Z_OLD.search(old) and Z_OLD.sub("Z45.000", old) == new:
        return "safe Z"
    if ESTIMATE.search(old) and ESTIMATE.sub("", old) == ESTIMATE.sub("", new):
        return "estimate"
    if tie(old, new):
        return "tie"
    if any(f in path for f in LINK_FILES) and LINK.match(old) and LINK.match(new):
        return "link"
    if sheet and re.search(r"(?i)safe z|safe cylinder|clearance|G5[4-9]|time|duration|minutes|\b40\b|\b45\b", old + new):
        return "sheet"
    return None


def main():
    old = sys.argv[1]
    new = sys.argv[2] if len(sys.argv) > 2 else None
    args = ["git", "diff", "--name-only", old] + ([new] if new else []) + ["--", "testdata/golden"]
    files = subprocess.run(args, check=True, capture_output=True, text=True).stdout.split()
    bad = 0
    print(f"{'file':58} {'lines':>6} {'safe Z':>6} {'estim.':>6} {'sheet':>5} {'tie':>4} {'link':>4}  other")
    for path in sorted(files):
        if path.endswith(".pdf"):
            print(f"{path:58} binary: compare its .html")
            continue
        a, b = show(old, path).splitlines(), show(new, path).splitlines()
        sheet = path.endswith((".html", ".txt"))
        counts = {"safe Z": 0, "estimate": 0, "sheet": 0, "tie": 0, "link": 0}
        links = []
        other = []
        if len(a) != len(b):
            other.append(f"line count {len(a)} -> {len(b)}")
        else:
            for n, (x, y) in enumerate(zip(a, b), 1):
                if x == y:
                    continue
                k = kind(x, y, sheet, path)
                if k:
                    counts[k] += 1
                    if k == "link":
                        links.append(f"line {n}: {x} -> {y}")
                else:
                    other.append(n)
        bad += bool(other)
        print(f"{path:58} {len(b):>6} {counts['safe Z']:>6} {counts['estimate']:>6} {counts['sheet']:>5} {counts['tie']:>4} {counts['link']:>4}  {other[:5] if other else 'none'}")
        for line in links:
            print(f"    link height {line}")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
