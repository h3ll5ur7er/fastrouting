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


RECIPES = {"placements": placements}
