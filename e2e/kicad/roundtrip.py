"""The KiCad half of traxide's end-to-end test, run with KiCad's own Python.

  python3 roundtrip.py export RECIPE FOLDER
      builds the recipe's board, saves FOLDER/RECIPE.kicad_pcb and exports
      FOLDER/RECIPE.dsn, as a user would with File → Export → Specctra DSN.

  python3 roundtrip.py import FOLDER/RECIPE.kicad_pcb FOLDER/RECIPE.ses
      imports the session into the board, as File → Import → Specctra Session
      does, and saves the result as FOLDER/RECIPE.routed.kicad_pcb.

KiCad's DRC (kicad-cli pcb drc) then judges the routed board.
"""

import shutil
import subprocess
import sys
from pathlib import Path

import pcbnew

from boards import RECIPES


def build(recipe: str, folder: Path) -> None:
    folder.mkdir(parents=True, exist_ok=True)
    pcbnew.SaveBoard(str(folder / f"{recipe}.kicad_pcb"), RECIPES[recipe].build())


def export(recipe: str, folder: Path) -> None:
    # A board built in memory has no project, so saving it writes no net
    # classes. Loaded by a fresh process it has one, and saving it then writes
    # RECIPE.kicad_pro, where KiCad's DRC reads the rules.
    subprocess.run([sys.executable, __file__, "build", recipe, str(folder)], check=True)
    board_file = folder / f"{recipe}.kicad_pcb"
    board = pcbnew.LoadBoard(str(board_file))
    RECIPES[recipe].apply_rules(board)
    pcbnew.SaveBoard(str(board_file), board)
    if not pcbnew.ExportSpecctraDSN(board, str(folder / f"{recipe}.dsn")):
        raise SystemExit(f"KiCad could not export {recipe} as DSN")


def import_session(board_file: Path, session: Path) -> None:
    board = pcbnew.LoadBoard(str(board_file))
    if not pcbnew.ImportSpecctraSES(board, str(session)):
        raise SystemExit(f"KiCad could not import {session}")
    routed = board_file.with_name(board_file.name.replace(".kicad_pcb", ".routed.kicad_pcb"))
    pcbnew.SaveBoard(str(routed), board)
    # Saving wrote a project with KiCad's default rules; the board's own are in its project.
    shutil.copy(board_file.with_suffix(".kicad_pro"), routed.with_suffix(".kicad_pro"))
    print(f"{len(board.GetTracks())} tracks and vias imported into {routed.name}")


if __name__ == "__main__":
    match sys.argv[1:]:
        case ["build", recipe, folder]:
            build(recipe, Path(folder))
        case ["export", recipe, folder]:
            export(recipe, Path(folder))
        case ["import", board_file, session]:
            import_session(Path(board_file), Path(session))
        case _:
            raise SystemExit(__doc__)
