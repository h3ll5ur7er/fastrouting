"""Test boards for traxide, built with KiCad's own Python API (KiCad 9 and 10)."""

from dataclasses import dataclass

import pcbnew

MM = pcbnew.FromMM


@dataclass(frozen=True)
class Smd:
    number: str
    x: float
    y: float
    width: float
    height: float
    net: str | None = None
    rotation: float = 0.0


@dataclass(frozen=True)
class Tht:
    number: str
    x: float
    y: float
    diameter: float
    drill: float
    net: str | None = None


class BoardBuilder:
    def __init__(self, width: float, height: float, copper_layers: int = 2):
        self.board = pcbnew.BOARD()
        self.board.SetCopperLayerCount(copper_layers)
        self._nets: dict[str, pcbnew.NETINFO_ITEM] = {}
        self._outline(width, height)

    def _outline(self, width: float, height: float) -> None:
        corners = [(0, 0), (width, 0), (width, height), (0, height)]
        for start, end in zip(corners, corners[1:] + corners[:1]):
            edge = pcbnew.PCB_SHAPE(self.board)
            edge.SetShape(pcbnew.SHAPE_T_SEGMENT)
            edge.SetStart(pcbnew.VECTOR2I(MM(start[0]), MM(start[1])))
            edge.SetEnd(pcbnew.VECTOR2I(MM(end[0]), MM(end[1])))
            edge.SetLayer(pcbnew.Edge_Cuts)
            edge.SetWidth(MM(0.05))
            self.board.Add(edge)

    def _net(self, name: str | None):
        if name is None:
            return None
        if name not in self._nets:
            net = pcbnew.NETINFO_ITEM(self.board, name)
            self.board.Add(net)
            self._nets[name] = net
        return self._nets[name]

    def footprint(self, reference: str, x: float, y: float, pads, rotation: float = 0.0, back: bool = False):
        footprint = pcbnew.FOOTPRINT(self.board)
        footprint.SetReference(reference)
        footprint.SetFPID(pcbnew.LIB_ID("traxide", reference.rstrip("0123456789")))
        footprint.Reference().SetVisible(False)
        footprint.Value().SetVisible(False)
        self.board.Add(footprint)
        for spec in pads:
            footprint.Add(self._pad(footprint, spec))
        footprint.SetPosition(pcbnew.VECTOR2I(MM(x), MM(y)))
        footprint.SetOrientationDegrees(rotation)
        if back:
            footprint.Flip(footprint.GetPosition(), pcbnew.FLIP_DIRECTION_LEFT_RIGHT)
        return footprint

    def _pad(self, footprint, spec):
        pad = pcbnew.PAD(footprint)
        pad.SetNumber(spec.number)
        if isinstance(spec, Smd):
            pad.SetAttribute(pcbnew.PAD_ATTRIB_SMD)
            pad.SetShape(pcbnew.PAD_SHAPE_RECTANGLE)
            pad.SetSize(pcbnew.VECTOR2I(MM(spec.width), MM(spec.height)))
            pad.SetLayerSet(pad.SMDMask())
            pad.SetOrientationDegrees(spec.rotation)
        else:
            pad.SetAttribute(pcbnew.PAD_ATTRIB_PTH)
            pad.SetShape(pcbnew.PAD_SHAPE_CIRCLE)
            pad.SetSize(pcbnew.VECTOR2I(MM(spec.diameter), MM(spec.diameter)))
            pad.SetDrillSize(pcbnew.VECTOR2I(MM(spec.drill), MM(spec.drill)))
            pad.SetLayerSet(pad.PTHMask())
        pad.SetFPRelativePosition(pcbnew.VECTOR2I(MM(spec.x), MM(spec.y)))
        net = self._net(spec.net)
        if net is not None:
            pad.SetNet(net)
        return pad


def placements() -> pcbnew.BOARD:
    """Footprints on both sides at awkward angles, to pin down how DSN places pads."""
    builder = BoardBuilder(60, 40)
    asymmetric = lambda net: [
        Smd("1", -1.5, 0, 1.0, 0.6, net),
        Smd("2", 1.0, 0.8, 0.8, 0.5, net, rotation=30),
    ]
    builder.footprint("U1", 10, 10, asymmetric("A"))
    builder.footprint("U2", 25, 10, asymmetric("B"), rotation=30)
    builder.footprint("U3", 40, 10, asymmetric("C"), back=True)
    builder.footprint("U4", 10, 25, asymmetric("D"), rotation=90, back=True)
    builder.footprint("U5", 25, 25, asymmetric("E"), rotation=30, back=True)
    builder.footprint("J1", 45, 28, [Tht("1", -1.27, 0, 1.7, 1.0, "A"), Tht("2", 1.27, 0, 1.7, 1.0, "B")], rotation=45)
    return builder.board


def crossing() -> pcbnew.BOARD:
    """Two nets whose straight lines cross: one must change layers."""
    builder = BoardBuilder(30, 30)
    pad = lambda net: [Smd("1", 0, 0, 1.2, 1.2, net)]
    builder.footprint("TP1", 5, 15, pad("ACROSS"))
    builder.footprint("TP2", 25, 15, pad("ACROSS"))
    builder.footprint("TP3", 15, 5, pad("DOWN"))
    builder.footprint("TP4", 15, 25, pad("DOWN"))
    return builder.board


KEY_PITCH = 19.05


def key_matrix(rows: int, columns: int, copper_layers: int = 2) -> pcbnew.BOARD:
    """A keyboard's switch matrix: an MX switch and a diode per key, rows and
    columns wired to a header along the top edge, as on a hand-wired NKRO board."""
    header_pins = rows + columns
    width = max(columns * KEY_PITCH, header_pins * 2.54 + 10)
    builder = BoardBuilder(width, rows * KEY_PITCH + 12, copper_layers)
    header = [Tht(str(i + 1), i * 2.54, 0, 1.7, 1.0, net) for i, net in
              enumerate([f"ROW{r}" for r in range(rows)] + [f"COL{c}" for c in range(columns)])]
    builder.footprint("J1", (width - (header_pins - 1) * 2.54) / 2, 4, header)
    for r in range(rows):
        for c in range(columns):
            x, y = KEY_PITCH * (c + 0.5), 12 + KEY_PITCH * (r + 0.5)
            key = f"K{r}_{c}"
            builder.footprint(f"SW{r * columns + c + 1}", x, y, [
                Tht("1", -3.81, -2.54, 2.2, 1.5, f"COL{c}"),
                Tht("2", 2.54, -5.08, 2.2, 1.5, key),
            ])
            builder.footprint(f"D{r * columns + c + 1}", x + 4.5, y + 3, [
                Smd("1", -1.65, 0, 0.9, 1.2, f"ROW{r}"),
                Smd("2", 1.65, 0, 0.9, 1.2, key),
            ], rotation=90, back=True)
    return builder.board


RECIPES = {
    "placements": placements,
    "crossing": crossing,
    "matrix-3x4": lambda: key_matrix(3, 4),
    "matrix-6x17": lambda: key_matrix(6, 17),
}
