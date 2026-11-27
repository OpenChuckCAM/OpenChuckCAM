#!/bin/bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# timed.sh BIN case...: wall time, peak RSS of the program and of the STEP
# worker (sampled every 0.5 s), and the number of G-code lines, for each case
# directory under $RUNS (default ./runs). See README.md beside this file.
BIN=$(realpath "$1"); shift
cd "${RUNS:-runs}" || exit 1
for p in "$@"; do
  rm -rf $p/out
  start=$(date +%s.%N)
  timeout 3600 $BIN --threads 4 regenerate $p --post linuxcnc --tool-numbers keep --out $p/out --acknowledge-partial-path --acknowledge-collision-warnings > $p.log 2>&1 &
  pid=$!; peak=0; wpeak=0
  while kill -0 $pid 2>/dev/null; do
    for c in $(pgrep -P $pid) $(pgrep -f "openchuck-step-worker" ); do
      r=$(awk '/VmRSS/{print $2}' /proc/$c/status 2>/dev/null)
      case "$(cat /proc/$c/comm 2>/dev/null)" in
        openchuck-step*) [ -n "$r" ] && [ "$r" -gt "$wpeak" ] && wpeak=$r ;;
        *) [ -n "$r" ] && [ "$r" -gt "$peak" ] && peak=$r ;;
      esac
    done
    sleep 0.5
  done
  wait $pid; rc=$?
  end=$(date +%s.%N)
  printf "%s rc=%s %.1f s peakMB=%d workerMB=%d lines=%s\n" "$p" "$rc" "$(echo "$end - $start" | bc)" $((peak/1024)) $((wpeak/1024)) "$(cat $p/out/*.ngc 2>/dev/null | wc -l)"
done
