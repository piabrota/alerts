#!/usr/bin/env python3
"""Write docs/drawings/wave.svg: a Fibonacci-spiral wave and an abstract bell.

Usage: python3 docs/drawings/wave.py > docs/drawings/wave.svg
       python3 docs/drawings/wave.py squares   # also draw the Fibonacci squares
       python3 docs/drawings/wave.py desmos > docs/drawings/wave.desmos.json   # Desmos state
"""
import json
import math
import sys
s=5; fib=[1,1,2,3,5,8,13,21]; radii=[s*f for f in reversed(fib)]  # 105..5, outside in
x0,y0=20,164
right=lambda h:(-h[1],h[0])
P=(x0,y0); h=(0,-1); arcs=[]; squares=[]; quarters=[]
for r in radii:
    rv=right(h); C=(P[0]+r*rv[0],P[1]+r*rv[1]); E=(C[0]+r*h[0],C[1]+r*h[1])
    arcs.append(f"A {r} {r} 0 0 1 {E[0]} {E[1]}")
    xs=sorted([P[0],E[0]]); ys=sorted([P[1],E[1]]); squares.append((xs[0],ys[0],r))
    quarters.append((C,P,r))
    P=E; h=rv
spiral=f"M{x0} {y0} "+" ".join(arcs)
# An abstract bell in the top right corner: a dome, a rim, a clapper dot and two sound arcs.
bx,by=272,46  # centre of the rim
bell=(f"M{bx-14} {by} A 14 14 0 0 1 {bx+14} {by} M{bx-19} {by} L {bx+19} {by} "
      f"M{bx-26} {by-20} A 18 18 0 0 0 {bx-26} {by+6} M{bx+26} {by-20} A 18 18 0 0 1 {bx+26} {by+6}")
clapper=f'<circle cx="{bx}" cy="{by+7}" r="2.6" fill="#1682D4" stroke="none"/>'


def desmos_state():
    """The same drawing as Desmos expressions. Desmos has y up: y_desmos = 180 - y_svg."""
    H = 180
    def arc(c, start, sweep, r):
        cx, cy = c[0], H - c[1]
        a0 = math.atan2((H - start[1]) - cy, start[0] - cx)
        lo, hi = sorted([a0, a0 + sweep])
        return {"latex": f"\\left({cx}+{r}\\cos(t),{cy}+{r}\\sin(t)\\right)",
                "parametricDomain": {"min": f"{lo:.6f}", "max": f"{hi:.6f}"}}
    exprs = [arc(c, start, -math.pi / 2, r) for c, start, r in quarters]  # screen clockwise
    exprs.append(arc((bx, by), (bx - 14, by), -math.pi, 14))  # dome
    exprs.append({"latex": f"\\left({bx-19}+{38}t,{H-by}\\right)",
                  "parametricDomain": {"min": "0", "max": "1"}})  # rim
    for side in (-1, 1):  # sound arcs: radius 18 through (bx +- 26, by - 20) and (bx +- 26, by + 6)
        half = 13; d = math.sqrt(18 ** 2 - half ** 2)
        cx, cy = bx + side * 26 - side * d, by - 7  # the centre is on the bell side of the chord
        a = math.atan2(half, d)
        lo, hi = (math.pi - a, math.pi + a) if side < 0 else (-a, a)
        exprs.append({"latex": f"\\left({cx:.4f}+18\\cos(t),{H-cy}+18\\sin(t)\\right)",
                      "parametricDomain": {"min": f"{lo:.6f}", "max": f"{hi:.6f}"}})
    exprs.append({"latex": f"\\left({bx},{H-(by+7)}\\right)", "pointSize": "8"})  # clapper
    items = [dict(type="expression", id=str(i + 1), color="#1682D4", lineWidth="3.2", **e)
             for i, e in enumerate(exprs)]
    return {"version": 11,
            "graph": {"viewport": {"xmin": 0, "xmax": 320, "ymin": 0, "ymax": 180},
                      "showGrid": False, "showXAxis": False, "showYAxis": False},
            "expressions": {"list": items}}


if sys.argv[1:] == ["desmos"]:
    print(json.dumps(desmos_state(), indent=2))
    sys.exit(0)
debug=len(sys.argv)>1
sq="".join(f'<rect x="{x}" y="{y}" width="{r}" height="{r}" fill="none" stroke="#FF4F00" stroke-width="0.8" stroke-dasharray="2 2"/>' for x,y,r in squares) if debug else ""
label="A Fibonacci wave and a bell: an original line drawing"
print(f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 180" width="320" height="180" role="img" aria-label="{label}">
  <title>{label}</title>
  <desc>The wave is a Fibonacci spiral: eight quarter-circle arcs with radii 5 x (21, 13, 8, 5, 3, 2, 1, 1).</desc>{sq}
  <g fill="none" stroke="#1682D4" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round">
    <path d="{spiral}"/>
    <path d="{bell}"/>
    {clapper}
  </g>
</svg>''')
