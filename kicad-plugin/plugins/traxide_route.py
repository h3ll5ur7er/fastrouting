"""Routing a board open in KiCad with traxide: KiCad exports it as Specctra
DSN, traxide routes it, and KiCad imports the session, which replaces the
board's tracks and vias with the routed ones. Kept apart from the action, so
that it runs without KiCad's window, as the end-to-end test runs it."""

import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

import pcbnew


class RouteError(Exception):
    """Routing did not happen; the message says why, for the designer."""


@dataclass
class Outcome:
    summary: str
    complete: bool


def find_traxide() -> Path | None:
    """traxide named by $TRAXIDE, next to this plugin in bin/, or on the PATH."""
    executable = "traxide.exe" if sys.platform == "win32" else "traxide"
    named = os.environ.get("TRAXIDE")
    candidates = [
        Path(named) if named else None,
        Path(__file__).parent / "bin" / executable,
        Path(found) if (found := shutil.which("traxide")) else None,
    ]
    return next((c for c in candidates if c is not None and c.is_file()), None)


def route_board(board: "pcbnew.BOARD", traxide: Path, folder: Path) -> Outcome:
    design = folder / "board.dsn"
    session = folder / "board.ses"
    if not pcbnew.ExportSpecctraDSN(board, str(design)):
        raise RouteError("KiCad could not export the board as a Specctra design.")
    run = subprocess.run(
        [str(traxide), "route", "--quiet", str(design), "-o", str(session)],
        capture_output=True,
        text=True,
    )
    # 0 is complete, 2 incomplete and 3 a broken rule: a session was written for each.
    if run.returncode not in (0, 2, 3) or not session.is_file():
        raise RouteError((run.stderr or run.stdout).strip() or "traxide failed.")
    if not pcbnew.ImportSpecctraSES(board, str(session)):
        raise RouteError("KiCad could not import the session traxide wrote.")
    return Outcome(summary=run.stdout.strip(), complete=run.returncode == 0)
