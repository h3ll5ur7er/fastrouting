"""The KiCad half of traxide's end-to-end test, run with KiCad's own Python.

  python3 roundtrip.py export RECIPE FOLDER
      builds the recipe's board, saves FOLDER/RECIPE.kicad_pcb and exports
      FOLDER/RECIPE.dsn, as a user would with File → Export → Specctra DSN.

  python3 roundtrip.py import FOLDER/RECIPE.kicad_pcb FOLDER/RECIPE.ses
      imports the session into the board, as File → Import → Specctra Session
      does, and saves the result as FOLDER/RECIPE.routed.kicad_pcb.

KiCad's DRC (kicad-cli pcb drc) then judges the routed board.
"""

import sys
from pathlib import Path

import pcbnew

from boards import RECIPES


def export(recipe: str, folder: Path) -> None:
    folder.mkdir(parents=True, exist_ok=True)
    board_file = folder / f"{recipe}.kicad_pcb"
    pcbnew.SaveBoard(str(board_file), RECIPES[recipe]())
    board = pcbnew.LoadBoard(str(board_file))
    if not pcbnew.ExportSpecctraDSN(board, str(folder / f"{recipe}.dsn")):
        raise SystemExit(f"KiCad could not export {recipe} as DSN")


def import_session(board_file: Path, session: Path) -> None:
    board = pcbnew.LoadBoard(str(board_file))
    if not pcbnew.ImportSpecctraSES(board, str(session)):
        raise SystemExit(f"KiCad could not import {session}")
    routed = board_file.with_name(board_file.name.replace(".kicad_pcb", ".routed.kicad_pcb"))
    pcbnew.SaveBoard(str(routed), board)
    print(f"{len(board.GetTracks())} tracks and vias imported into {routed.name}")


if __name__ == "__main__":
    match sys.argv[1:]:
        case ["export", recipe, folder]:
            export(recipe, Path(folder))
        case ["import", board_file, session]:
            import_session(Path(board_file), Path(session))
        case _:
            raise SystemExit(__doc__)
