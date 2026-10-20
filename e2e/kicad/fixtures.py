"""Writes each recipe as KiCad sees it: the board, its DSN export, and the pads
KiCad places (the ground truth that traxide's DSN reader is tested against).

Usage: python3 fixtures.py OUTPUT_FOLDER
"""

import json
import sys
from pathlib import Path

import pcbnew

from boards import RECIPES


def pads_of(board) -> list[dict]:
    copper = [layer for layer in board.GetEnabledLayers().CuStack()]
    pads = []
    for footprint in board.GetFootprints():
        for pad in footprint.Pads():
            box = pad.GetBoundingBox()
            pads.append({
                "name": f"{footprint.GetReference()}-{pad.GetNumber()}",
                "net": pad.GetNetname() or None,
                "x": pad.GetPosition().x,
                "y": pad.GetPosition().y,
                "layers": [board.GetLayerName(layer) for layer in copper if pad.IsOnLayer(layer)],
                "bounds": [box.GetLeft(), box.GetTop(), box.GetRight(), box.GetBottom()],
            })
    return sorted(pads, key=lambda p: p["name"])


def main(output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    for name, recipe in RECIPES.items():
        board_file = output / f"{name}.kicad_pcb"
        pcbnew.SaveBoard(str(board_file), recipe.build())
        board = pcbnew.LoadBoard(str(board_file))
        if not pcbnew.ExportSpecctraDSN(board, str(output / f"{name}.dsn")):
            raise SystemExit(f"KiCad could not export {name} as DSN")
        (output / f"{name}.pads.json").write_text(json.dumps(pads_of(board), indent=1) + "\n")
        print(f"{name}: {board_file}")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
