"""Routes a board as the KiCad plugin does, without KiCad's window:
python3 plugin_route.py BOARD.kicad_pcb ROUTED.kicad_pcb TRAXIDE. Prints the
routed board's statistics as JSON, like roundtrip.py's import."""

import shutil
import sys
import tempfile
from pathlib import Path

import pcbnew

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "kicad-plugin" / "plugins"))
from traxide_route import route_board  # noqa: E402

from roundtrip import print_stats  # noqa: E402

if __name__ == "__main__":
    board_file, routed, traxide = (Path(a) for a in sys.argv[1:4])
    board = pcbnew.LoadBoard(str(board_file))
    with tempfile.TemporaryDirectory() as folder:
        outcome = route_board(board, traxide, Path(folder))
    pcbnew.SaveBoard(str(routed), board)
    # Saving wrote a project with KiCad's default rules; the board's own are in
    # its project and its custom rules.
    shutil.copy(board_file.with_suffix(".kicad_pro"), routed.with_suffix(".kicad_pro"))
    if board_file.with_suffix(".kicad_dru").is_file():
        shutil.copy(board_file.with_suffix(".kicad_dru"), routed.with_suffix(".kicad_dru"))
    print_stats(board)
    if not outcome.complete:
        raise SystemExit(outcome.summary)
