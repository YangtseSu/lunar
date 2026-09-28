"""Check the READMEs' `console` blocks against the real binary.

Every `$ lunar …` line in a console block is an executable sample: the lines
under it must be what the tool prints, byte for byte. That is why the
expected output is always copied from a run and never hand-computed — the
grid's column width is per-grid, so a hand-written expectation is wrong by
construction.

Both documents are checked, because the Chinese README is a copy of the same
output: a mistyped line there is just as wrong as one here, and a check that
only read `README.md` could not see it.

Exits non-zero on any mismatch, so a caller can gate on it. Stdlib only.

    cargo build && python3 tools/check_samples.py
    python3 tools/check_samples.py README.md          # one file
"""

import pathlib
import re
import shlex
import subprocess
import sys

BINARY = "./target/debug/lunar"
FENCE = "`" * 3
DEFAULT_FILES = ("README.md", "README.zh-CN.md")


def check(path):
    """Return the number of mismatching `$ lunar …` samples in `path`."""
    blocks = re.findall(f"{FENCE}console\n(.*?){FENCE}", path.read_text(), re.S)
    fails = 0
    for bi, block in enumerate(blocks):
        lines = block.rstrip("\n").split("\n")
        i = 0
        while i < len(lines):
            if not lines[i].startswith("$ "):
                i += 1
                continue
            # A trailing comment is documentation of the invocation, not part
            # of it: `lunar date -l -d 2026-07-15      # 农历 2026 年七月十五`.
            cmd = re.split(r"\s{4,}#", lines[i][2:])[0]
            args = shlex.split(cmd)[1:]
            want = []
            j = i + 1
            while j < len(lines) and not lines[j].startswith("$ "):
                want.append(lines[j])
                j += 1
            done = subprocess.run([BINARY] + args, capture_output=True, text=True)
            got = (done.stdout + done.stderr).rstrip("\n").split("\n")
            while want and not want[-1].strip():
                want.pop()
            if got != want:
                fails += 1
                print(f"{path} BLOCK {bi} MISMATCH: {cmd}")
                print("  WANT:", want)
                print("  GOT :", got)
            i = j
    print(f"{path} blocks: {len(blocks)} fails: {fails}")
    return len(blocks), fails


def main(argv):
    paths = [pathlib.Path(a) for a in argv[1:]] or [
        pathlib.Path(a) for a in DEFAULT_FILES
    ]
    total = 0
    fails = 0
    for path in paths:
        blocks, bad = check(path)
        total += blocks
        fails += bad
    if fails:
        print(f"{fails} documented sample(s) do not reproduce the binary")
        return 1
    if not total:
        print("no console blocks found — is the fence the tool expects?")
        return 1
    print("every documented sample reproduces the binary")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
