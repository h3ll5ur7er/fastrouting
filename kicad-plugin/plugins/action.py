import tempfile
from pathlib import Path

import pcbnew
import wx

from .traxide_route import RouteError, find_traxide, route_board


class TraxideRoute(pcbnew.ActionPlugin):
    def defaults(self):
        self.name = "Route with traxide"
        self.category = "Routing"
        self.description = "Routes the board with traxide, a fast autorouter"
        self.show_toolbar_button = False

    def Run(self):
        traxide = find_traxide()
        if traxide is None:
            wx.MessageBox(
                "traxide was not found. Put it in this plugin's bin folder, "
                "on the PATH, or name it in the TRAXIDE environment variable.",
                "traxide",
                wx.OK | wx.ICON_ERROR,
            )
            return
        board = pcbnew.GetBoard()
        try:
            with wx.BusyCursor(), tempfile.TemporaryDirectory() as folder:
                outcome = route_board(board, traxide, Path(folder))
        except RouteError as error:
            wx.MessageBox(str(error), "traxide", wx.OK | wx.ICON_ERROR)
            return
        pcbnew.Refresh()
        icon = wx.ICON_INFORMATION if outcome.complete else wx.ICON_WARNING
        wx.MessageBox(outcome.summary, "traxide", wx.OK | icon)
