//! `wave`: writes docs/drawings/wave.svg, a Fibonacci-spiral wave and an abstract bell.
//!
//! ```console
//! cargo run -q -p smart-alerts-drawings --bin wave > docs/drawings/wave.svg
//! cargo run -q -p smart-alerts-drawings --bin wave squares   # also draw the Fibonacci squares
//! cargo run -q -p smart-alerts-drawings --bin wave desmos > docs/drawings/wave.desmos.json
//! ```

use std::f64::consts::PI;

use serde_json::{Value, json};

/// The SVG height: Desmos has y up, so `y_desmos = H - y_svg`.
const H: f64 = 180.0;
/// The scale: radii are 5 × (21, 13, 8, 5, 3, 2, 1, 1), outside in.
const S: i64 = 5;
const FIB: [i64; 8] = [1, 1, 2, 3, 5, 8, 13, 21];
const START: (i64, i64) = (20, 164);
/// The centre of the bell's rim, in the top right corner.
const BELL: (i64, i64) = (272, 46);

/// One quarter arc of the spiral: centre, start point, radius.
#[derive(Debug, Clone, Copy)]
struct Quarter {
    c: (i64, i64),
    start: (i64, i64),
    r: i64,
}

#[derive(Debug)]
struct Spiral {
    path: String,
    squares: Vec<(i64, i64, i64)>,
    quarters: Vec<Quarter>,
}

fn spiral() -> Spiral {
    let right = |h: (i64, i64)| (-h.1, h.0);
    let (mut p, mut h) = (START, (0, -1));
    let (mut arcs, mut squares, mut quarters) = (vec![], vec![], vec![]);
    for r in FIB.iter().rev().map(|f| S * f) {
        let rv = right(h);
        let c = (p.0 + r * rv.0, p.1 + r * rv.1);
        let e = (c.0 + r * h.0, c.1 + r * h.1);
        arcs.push(format!("A {r} {r} 0 0 1 {} {}", e.0, e.1));
        squares.push((p.0.min(e.0), p.1.min(e.1), r));
        quarters.push(Quarter { c, start: p, r });
        (p, h) = (e, rv);
    }
    Spiral {
        path: format!("M{} {} {}", START.0, START.1, arcs.join(" ")),
        squares,
        quarters,
    }
}

fn bell_path() -> String {
    let (bx, by) = BELL;
    format!(
        "M{} {by} A 14 14 0 0 1 {} {by} M{} {by} L {} {by} M{} {} A 18 18 0 0 0 {} {} M{} {} A 18 18 0 0 1 {} {}",
        bx - 14,
        bx + 14,
        bx - 19,
        bx + 19,
        bx - 26,
        by - 20,
        bx - 26,
        by + 6,
        bx + 26,
        by - 20,
        bx + 26,
        by + 6
    )
}

/// A Desmos parametric arc: `(cx + r cos t, cy + r sin t)` over `[lo, hi]`.
fn arc(c: (i64, i64), start: (i64, i64), sweep: f64, r: i64) -> Value {
    let (cx, cy) = (c.0 as f64, H - c.1 as f64);
    let a0 = ((H - start.1 as f64) - cy).atan2(start.0 as f64 - cx);
    let (lo, hi) = (a0.min(a0 + sweep), a0.max(a0 + sweep));
    json!({
        "latex": format!("\\left({}+{r}\\cos(t),{}+{r}\\sin(t)\\right)", c.0, num(cy)),
        "parametricDomain": {"min": format!("{lo:.6}"), "max": format!("{hi:.6}")},
    })
}

/// A number as Python prints it: an integer value without a decimal point.
fn num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// The drawing as a Desmos graph state.
fn desmos_state(sp: &Spiral) -> Value {
    let (bx, by) = BELL;
    let mut exprs: Vec<Value> = sp
        .quarters
        .iter()
        .map(|q| arc(q.c, q.start, -PI / 2.0, q.r)) // clockwise on screen
        .collect();
    exprs.push(arc((bx, by), (bx - 14, by), -PI, 14)); // dome
    exprs.push(json!({
        "latex": format!("\\left({}+38t,{}\\right)", bx - 19, num(H - by as f64)),
        "parametricDomain": {"min": "0", "max": "1"},
    })); // rim
    for side in [-1.0_f64, 1.0] {
        // Sound arcs: radius 18 through (bx ± 26, by - 20) and (bx ± 26, by + 6). The centre is
        // on the bell side of the chord.
        let (half, rr) = (13.0_f64, 18.0_f64);
        let d = (rr * rr - half * half).sqrt();
        let (cx, cy) = (bx as f64 + side * 26.0 - side * d, (by - 7) as f64);
        let a = half.atan2(d);
        let (lo, hi) = if side < 0.0 {
            (PI - a, PI + a)
        } else {
            (-a, a)
        };
        exprs.push(json!({
            "latex": format!("\\left({cx:.4}+18\\cos(t),{}+18\\sin(t)\\right)", num(H - cy)),
            "parametricDomain": {"min": format!("{lo:.6}"), "max": format!("{hi:.6}")},
        }));
    }
    exprs.push(json!({"latex": format!("\\left({bx},{}\\right)", num(H - (by + 7) as f64)), "pointSize": "8"})); // clapper
    let items: Vec<Value> = exprs
        .into_iter()
        .enumerate()
        .map(|(i, e)| {
            let mut item = json!({"type": "expression", "id": (i + 1).to_string(), "color": "#1682D4", "lineWidth": "3.2"});
            if let (Some(m), Value::Object(e)) = (item.as_object_mut(), e) {
                m.extend(e);
            }
            item
        })
        .collect();
    json!({
        "version": 11,
        "graph": {"viewport": {"xmin": 0, "xmax": 320, "ymin": 0, "ymax": 180},
                  "showGrid": false, "showXAxis": false, "showYAxis": false},
        "expressions": {"list": items},
    })
}

/// Evaluates one coordinate of the expressions above: `a`, `a+bt` or `a+b\cos(t)` / `a+b\sin(t)`.
fn eval(expr: &str, t: f64) -> Option<f64> {
    let Some((a, rest)) = expr.split_once('+') else {
        return expr.parse().ok();
    };
    let a: f64 = a.parse().ok()?;
    if let Some(b) = rest.strip_suffix("\\cos(t)") {
        Some(a + b.parse::<f64>().ok()? * t.cos())
    } else if let Some(b) = rest.strip_suffix("\\sin(t)") {
        Some(a + b.parse::<f64>().ok()? * t.sin())
    } else {
        Some(a + rest.strip_suffix('t')?.parse::<f64>().ok()? * t)
    }
}

/// Samples each Desmos expression into SVG: the orange line of the default render.
fn desmos_layer(state: &Value) -> Option<String> {
    let mut out = vec![];
    for e in state["expressions"]["list"].as_array()? {
        let inner = e["latex"]
            .as_str()?
            .strip_prefix("\\left(")?
            .strip_suffix("\\right)")?;
        let (fx, fy) = inner.split_once(',')?;
        if let Some(d) = e.get("parametricDomain") {
            let lo: f64 = d["min"].as_str()?.parse().ok()?;
            let hi: f64 = d["max"].as_str()?.parse().ok()?;
            let n = 64;
            let pts = (0..=n)
                .map(|i| {
                    let t = lo + (hi - lo) * f64::from(i) / f64::from(n);
                    Some(format!("{:.2},{:.2}", eval(fx, t)?, H - eval(fy, t)?))
                })
                .collect::<Option<Vec<_>>>()?;
            out.push(format!("<polyline points=\"{}\"/>", pts.join(" ")));
        } else {
            out.push(format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"1.4\" fill=\"#FF4F00\"/>",
                num(eval(fx, 0.0)?),
                num(H - eval(fy, 0.0)?)
            ));
        }
    }
    Some(out.join("\n    "))
}

fn svg(sp: &Spiral, squares: bool) -> Option<String> {
    let (bx, by) = BELL;
    let sq: String = if squares {
        sp.squares
            .iter()
            .map(|(x, y, r)| format!("<rect x=\"{x}\" y=\"{y}\" width=\"{r}\" height=\"{r}\" fill=\"none\" stroke=\"#FF4F00\" stroke-width=\"0.8\" stroke-dasharray=\"2 2\"/>"))
            .collect()
    } else {
        String::new()
    };
    let label = "A Fibonacci wave and a bell, by @h0ffmann: an original line drawing, rendered from its Desmos expressions";
    let layer = desmos_layer(&desmos_state(sp))?;
    Some(format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 180" width="320" height="180" role="img" aria-label="{label}">
  <title>{label}</title>
  <desc>The wave is a Fibonacci spiral: eight quarter-circle arcs with radii 5 x (21, 13, 8, 5, 3, 2, 1, 1). The blue line is the geometry; the orange line is each expression of wave.desmos.json, sampled.</desc>{sq}
  <g fill="none" stroke="#1682D4" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round">
    <path d="{}"/>
    <path d="{}"/>
    <circle cx="{bx}" cy="{}" r="2.6" fill="#1682D4" stroke="none"/>
  </g>
  <g fill="none" stroke="#FF4F00" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">
    {layer}
  </g>
</svg>"##,
        sp.path,
        bell_path(),
        by + 7
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let sp = spiral();
    let out = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["desmos"] => serde_json::to_string_pretty(&desmos_state(&sp))?,
        [] => svg(&sp, false).ok_or("cannot sample the Desmos expressions")?,
        _ => svg(&sp, true).ok_or("cannot sample the Desmos expressions")?,
    };
    println!("{out}");
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn radii_are_fibonacci() {
        let r: Vec<i64> = spiral().quarters.iter().map(|q| q.r).collect();
        assert_eq!(r, [105, 65, 40, 25, 15, 10, 5, 5]);
    }

    #[test]
    fn the_committed_files_are_the_output() {
        let sp = spiral();
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/drawings/");
        let svg_file = std::fs::read_to_string(format!("{dir}wave.svg")).unwrap();
        let json_file = std::fs::read_to_string(format!("{dir}wave.desmos.json")).unwrap();
        assert_eq!(svg_file, svg(&sp, false).unwrap() + "\n");
        assert_eq!(
            json_file,
            serde_json::to_string_pretty(&desmos_state(&sp)).unwrap() + "\n"
        );
    }
}
