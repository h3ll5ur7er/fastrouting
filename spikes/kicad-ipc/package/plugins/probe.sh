#!/bin/sh
{ echo "argv: $0 $*"; echo "cwd: $(pwd)"; env | sort; } > "${TRAXIDE_PROBE_OUT:-/tmp/traxide-probe.txt}" 2>&1
