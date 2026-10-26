#!/usr/bin/env python3
"""Write docs/drawings/wave.svg: a Fibonacci-spiral wave and an abstract bell.

Usage: python3 docs/drawings/wave.py > docs/drawings/wave.svg
       python3 docs/drawings/wave.py squares   # also draw the Fibonacci squares
"""
import sys
s=5; fib=[1,1,2,3,5,8,13,21]; radii=[s*f for f in reversed(fib)]  # 105..5, outside in
x0,y0=20,164
right=lambda h:(-h[1],h[0])
P=(x0,y0); h=(0,-1); arcs=[]; squares=[]
for r in radii:
    rv=right(h); C=(P[0]+r*rv[0],P[1]+r*rv[1]); E=(C[0]+r*h[0],C[1]+r*h[1])
    arcs.append(f"A {r} {r} 0 0 1 {E[0]} {E[1]}")
    xs=sorted([P[0],E[0]]); ys=sorted([P[1],E[1]]); squares.append((xs[0],ys[0],r))
    P=E; h=rv
spiral=f"M{x0} {y0} "+" ".join(arcs)
# An abstract bell in the top right corner: a dome, a rim, a clapper dot and two sound arcs.
bx,by=272,46  # centre of the rim
bell=(f"M{bx-14} {by} A 14 14 0 0 1 {bx+14} {by} M{bx-19} {by} L {bx+19} {by} "
      f"M{bx-26} {by-20} A 18 18 0 0 0 {bx-26} {by+6} M{bx+26} {by-20} A 18 18 0 0 1 {bx+26} {by+6}")
clapper=f'<circle cx="{bx}" cy="{by+7}" r="2.6" fill="#1682D4" stroke="none"/>'
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
