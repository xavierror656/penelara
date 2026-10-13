"""Show compact memory/file state transitions from Replay-Ghc JSONL."""

import argparse
import json
from pathlib import Path


parser = argparse.ArgumentParser()
parser.add_argument("log", type=Path)
args = parser.parse_args()

previous = None
for line in args.log.open(encoding="utf-8-sig"):
    item = json.loads(line)
    if item.get("stage") != "running":
        continue
    mem = item.get("engineMemory") or {}
    result = item.get("result") or {}
    counters = mem.get("resultCounters") or {}
    state = (
        mem.get("bestCompleteWeight"),
        (result.get("sha256") or "")[:8],
        counters.get("teacherGaps"),
        counters.get("teacherNonpreferred"),
    )
    if state != previous:
        print(f"{item.get('second'):6.3f}s  score={state[0]}  file={state[1]}  gaps={state[2]}  noPref={state[3]}")
        previous = state
