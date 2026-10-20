"""Original interoperability patterns, authored 2026-10-19. Apache-2.0.

This writes our own geometry and measurements in Seamly's documented XML
format. No upstream example, proprietary block, or BETA source is used.
All construction lengths are centimetres. These are test shapes, not fit-tested
garment production patterns.
"""
from pathlib import Path
import json
import xml.etree.ElementTree as X

ROOT = Path(__file__).resolve().parent
MEASURES = {
    "waist": (78, 4), "hip": (100, 4), "chest": (96, 4),
    "skirt_length": (60, 0), "shirt_length": (65, 0.5),
    "trouser_length": (100, 0.5), "crotch_depth": (27, 0.5),
}


def element(parent, tag, content=None, **attrs):
    node = X.SubElement(parent, tag, {k: str(v) for k, v in attrs.items()})
    node.text = content
    return node


def save(root, name):
    X.indent(root, space="    ")
    X.ElementTree(root).write(ROOT / name, encoding="UTF-8", xml_declaration=True)


def measurements():
    root = X.Element("smms")
    element(root, "version", "0.4.5")
    element(root, "read-only", "false")
    element(root, "notes", "Original synthetic measurements authored by us. Apache-2.0. Sizes 48, 50, 52.")
    element(root, "unit", "cm")
    element(root, "pm_system", "998")
    element(root, "size", base=50)
    element(root, "height", base=176)
    body = element(root, "body-measurements")
    for name, (base, inc) in MEASURES.items():
        element(body, "m", name="@" + name, base=base, height_increase=0,
                size_increase=inc, full_name=name.replace("_", " "))
    save(root, "original-multisize.smms")
    # Separate original individual tables let a subset importer be probed after
    # its multisize rejection. They do not replace the real .smms test.
    for size in (48, 50, 52):
        root = X.Element("smis")
        element(root, "version", "0.3.4")
        element(root, "read-only", "false")
        element(root, "notes", "Original derived individual table. Apache-2.0.")
        element(root, "unit", "cm")
        element(root, "pm_system", "998")
        personal = element(root, "personal")
        for tag, value in (("family-name", "Synthetic"), ("given-name", "Test"),
                           ("birth-date", "1900-01-01"), ("gender", "unknown"), ("email", "")):
            element(personal, tag, value)
        body = element(root, "body-measurements")
        for name, (base, inc) in MEASURES.items():
            element(body, "m", name="@" + name, value=base + (size - 50) / 2 * inc,
                    full_name=name.replace("_", " "))
        save(root, f"original-size-{size}.smis")


class Pattern:
    def __init__(self, name):
        self.name = name
        self.root = X.Element("pattern")
        element(self.root, "version", "0.7.5")
        element(self.root, "unit", "cm")
        element(self.root, "description", f"Original synthetic {name} interoperability pattern. Apache-2.0.")
        element(self.root, "notes", "Authored by us from simple geometric constraints; not copied from any sample.")
        grad = element(self.root, "gradation", custom="true", defHeight=176, defSize=50)
        element(grad, "heights", all="false", h176="true")
        element(grad, "sizes", all="false", s48="true", s50="true", s52="true")
        element(self.root, "patternName", name)
        element(self.root, "patternNumber", "ORIGINAL-20261019")
        element(self.root, "company", "Original interoperability test data")
        element(self.root, "measurements", "original-multisize.smms")
        self.next_id = 1
        self.pieces = []

    def new_id(self):
        result = self.next_id
        self.next_id += 1
        return result

    def block(self, name, vertices, curves=None, dart=None, quantity=2, material="cotton", notch=2):
        block = element(self.root, "draftBlock", name=name)
        calc = element(block, "calculation")
        model = element(block, "modeling")
        pieces = element(block, "pieces")
        origin = self.new_id()
        element(calc, "point", id=origin, name=f"P{origin}", type="single", x=0, y=0, mx=0.2, my=0.2)

        def point(x, y):
            horizontal = self.new_id()
            element(calc, "point", id=horizontal, name=f"P{horizontal}", type="endLine",
                    basePoint=origin, angle=0, length=x, lineType="none", lineColor="black", mx=0.2, my=0.2)
            result = self.new_id()
            element(calc, "point", id=result, name=f"P{result}", type="endLine",
                    basePoint=horizontal, angle=270, length=y, lineType="none", lineColor="black", mx=0.2, my=0.2)
            return result

        points = [point(x, y) for x, y in vertices]
        boundary = []
        for i, pid in enumerate(points):
            boundary.append(("point", pid))
            if curves and i in curves:
                a1, l1, a2, l2 = curves[i]
                sid = self.new_id()
                element(calc, "spline", id=sid, type="simple", point1=pid,
                        point4=points[(i + 1) % len(points)], angle1=a1, angle2=a2,
                        length1=l1, length2=l2, color="black", penStyle="solidLine")
                boundary.append(("spline", sid))

        def alias(tag, pid):
            mid = self.new_id()
            element(model, tag, id=mid, idObject=pid,
                    type="modelingSpline" if tag == "spline" else "modeling", inUse="true")
            return mid

        modeled = [(tag, alias(tag, pid)) for tag, pid in boundary]
        path_id = None
        if dart:
            dart_points = [point(x, y) for x, y in dart]
            dart_aliases = [alias("point", pid) for pid in dart_points]
            path_id = self.new_id()
            path = element(model, "path", id=path_id, type=2, name="Waist dart", cut="false",
                           lineColor="black", lineType="solidLine", inUse="true")
            nodes = element(path, "nodes")
            for mid in dart_aliases:
                element(nodes, "node", idObject=mid, type="NodePoint")

        piece_id = self.new_id()
        piece = element(pieces, "piece", id=piece_id, name=name, version=2, mx=len(self.pieces) * 1300,
                        my=0, inLayout="true", forbidFlipping="false", width=1,
                        seamAllowance="true", seamAllowanceBuiltIn="false", hideMainPath="false")
        label = element(piece, "data", visible="true", quantity=quantity, letter=name[0],
                        annotation="One mirrored pair" if quantity == 2 else "Single piece",
                        onFold="false", fontSize=10, mx=200, my=450, width=14, height=10, rotation=0)
        for text in ("Piece Name: %pName%", "Size: %size%", "Quantity: %pQuantity%",
                     f"Material: {material}", "%pAnnotation%"):
            element(label, "line", text=text, alignment=0, bold="false", italic="false", sfIncrement=0)
        element(piece, "grainline", visible="true", mx=350, my=850, length=25,
                rotation=90, arrows=0, arrowLength=1)
        nodes = element(piece, "nodes")
        point_index = 0
        for i, (tag, mid) in enumerate(modeled):
            attrs = {"idObject": mid, "type": "NodePoint" if tag == "point" else "NodeSpline"}
            if tag == "point" and point_index == notch:
                attrs.update(notch="true", notchType="slit", notchSubtype="straightforward",
                             showNotch="true", notchLength=0.5, notchWidth=0.2, notchCount=1)
            element(nodes, "node", **attrs)
            if tag == "point":
                point_index += 1
        if path_id:
            paths = element(piece, "iPaths")
            element(paths, "record", path=path_id)
        self.pieces.append({"name": name, "quantity": quantity, "intended_mirror_quantity": 1,
                            "material": material, "seam_allowance_mm": 10, "has_dart": bool(dart),
                            "curved_segments": len(curves or {})})

    def save(self):
        save(self.root, self.name + ".sm2d")
        return {"file": self.name + ".sm2d", "unit": "cm", "sizes": [48, 50, 52], "height_cm": 176,
                "pieces": self.pieces}


def main():
    measurements()
    manifests = []
    skirt = Pattern("skirt-block")
    for suffix, dart_x in (("front", 9), ("back", 11)):
        skirt.block("Skirt-" + suffix,
                    [(0, 0), ("@waist/4+1", 0), ("@hip/4+1", 20), ("@hip/4+2", "@skirt_length"),
                     (0, "@skirt_length")],
                    curves={1: (270, 7, 95, 7)},
                    dart=[(dart_x-1, 0), (dart_x, 11), (dart_x+1, 0)])
    manifests.append(skirt.save())
    shirt = Pattern("shirt-front-back")
    for suffix, neck_depth in (("front", 8), ("back", 2.5)):
        shirt.block("Shirt-" + suffix,
                    [(0, neck_depth), (7.5, 0), (19.5, 3.5), ("@chest/4+2", 23.5),
                     ("@waist/4+4", 42), ("@hip/4+1", "@shirt_length"), (0, "@shirt_length")],
                    curves={0: (0, 4, 270, 3), 2: (285, 7, 90, 8)},
                    dart=[(11.5, 42), (13, 31), (14.5, 42), (13, 54), (11.5, 42)])
    manifests.append(shirt.save())
    trousers = Pattern("trouser-front")
    trousers.block("Trouser-front",
                   [(0, 0), ("@waist/4+3", 0), ("@hip/4+1", 20),
                    ("@hip/4+6", "@crotch_depth"), (19, "@trouser_length"),
                    (1, "@trouser_length"), (3, "@crotch_depth")],
                   curves={2: (270, 4, 180, 4), 6: (90, 9, 270, 9)},
                   dart=[(9, 0), (10, 12), (11, 0)], material="twill")
    manifests.append(trousers.save())
    (ROOT / "expected.json").write_text(json.dumps(manifests, indent=2), encoding="utf-8")
    print("Created three original patterns, one multisize table, three derived individual tables, and expected.json")


if __name__ == "__main__":
    main()
