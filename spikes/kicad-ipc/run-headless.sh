#!/usr/bin/env bash
# Opens a board in KiCad 10's PCB editor inside the official Docker image, on a
# virtual display and with the IPC API enabled, then runs the spike against it:
#
#   spikes/kicad-ipc/run-headless.sh BOARD.kicad_pcb [--undo]
#
# KiCad 9 and 10 serve the API only from the GUI, hence the display. Linux only:
# the API's Unix socket is shared through a bind mount, which Docker Desktop's VM
# does not pass through on macOS or Windows.
set -euo pipefail

# -ac: the container's host name does not match xvfb-run's cookie, and the
# throwaway display has nothing to protect.
if [ -z "${DISPLAY:-}" ]; then
  exec xvfb-run -a -s "-ac -screen 0 1600x1000x24" "$0" "$@"
fi

board=$(realpath "${1:?usage: run-headless.sh BOARD.kicad_pcb [--undo]}")
shift
image=${TRAXIDE_KICAD_IMAGE:-kicad/kicad:10.0}
spike_dir=$(dirname "$(realpath "$0")")
state=$(mktemp -d)
container=traxide-kicad-ipc-$$
trap 'docker rm -f "$container" >/dev/null 2>&1 || true; rm -rf "$state"' EXIT

mkdir -p "$state/config/10.0" "$state/socket" "$state/project"
cp -r "$(dirname "$board")"/. "$state/project/"
rm -f "$state/project"/~*.lck
# The image runs KiCad as its own user, who must write the configuration and the socket.
chmod -R a+rwX "$state"
# KiCad greets a new user with a wizard, a modal dialog that keeps the API busy
# until someone closes it, unless all three global library tables exist and the
# wizard's two questions (data collection, update checks) count as answered.
docker run --rm -v "$state/config/10.0:/out" "$image" \
  sh -c 'cp ~/.config/kicad/10.0/fp-lib-table ~/.config/kicad/10.0/sym-lib-table /out/'
cat > "$state/config/10.0/design-block-lib-table" <<'EOF'
(design_block_lib_table
	(version 7)
	(lib (name "KiCad") (type "Table") (uri "/usr/share/kicad/template/design-block-lib-table") (options "") (descr "KiCad Default Libraries"))
)
EOF
cat > "$state/config/10.0/kicad_common.json" <<'EOF'
{
  "api": { "enable_server": true },
  "do_not_show_again": { "data_collection_prompt": true, "update_check_prompt": true }
}
EOF
chmod -R a+rwX "$state/config"

display_socket_dir=/tmp/.X11-unix
docker run -d --name "$container" -e DISPLAY="$DISPLAY" \
  -v "$display_socket_dir:$display_socket_dir" \
  -v "$state/config:/home/kicad/.config/kicad" \
  -v "$state/socket:/tmp/kicad" \
  -v "$state/project:/project" \
  "$image" pcbnew "/project/$(basename "$board")" >/dev/null

for _ in $(seq 120); do
  [ -S "$state/socket/api.sock" ] && break
  sleep 0.5
done
[ -S "$state/socket/api.sock" ] || { docker logs "$container"; exit 1; }

cargo build --quiet --release --manifest-path "$spike_dir/Cargo.toml"
# The socket appears before the board is loaded; KiCad answers "not ready" or
# "no board" until then.
for attempt in $(seq 30); do
  if KICAD_API_SOCKET="ipc://$state/socket/api.sock" \
    "$spike_dir/target/release/traxide-kicad-ipc-spike" "$@"; then
    exit 0
  fi
  echo "attempt $attempt failed; retrying" >&2
  sleep 2
done
exit 1
