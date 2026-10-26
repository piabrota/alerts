//! `desmos-gif`: records a Desmos animation as a looping GIF.
//!
//! The binary opens a Desmos graph in headless Chrome, stops the animated slider, sets it to each
//! value from its minimum to its maximum, takes the calculator's own screenshot at each value, and
//! writes the frames as one GIF. GitHub cannot run the calculator in a README; it can show the GIF.
//!
//! Chrome runs with `--user-agent=smart_alerts_core::USER_AGENT`, never a browser User-Agent. The
//! binary speaks the DevTools protocol itself (one WebSocket), with no browser crate.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use serde_json::{Value, json};
use tungstenite::{Message, WebSocket, stream::MaybeTlsStream};

const USAGE: &str = "usage: desmos-gif [--url URL] [--slider NAME] [--frames N] [--size PX] [--delay CS] --out FILE.gif
  --url     the Desmos graph (default: Ammonite by Harry Chen, https://www.desmos.com/calculator/z7zooq9zsh)
  --slider  the LaTeX name of the animated slider (default: t_{animate})
  --frames  the number of frames in one loop (default: 48)
  --size    the width and height of the GIF in pixels (default: 480)
  --delay   the time of one frame in hundredths of a second (default: 6)
Chrome: the CHROME variable names the binary; else chromium, chromium-browser, google-chrome on PATH.\nCHROME_NO_SANDBOX=1 adds --no-sandbox (a container without user namespaces).";

#[derive(Debug)]
struct Args {
    url: String,
    slider: String,
    frames: u32,
    size: u32,
    delay: u16,
    out: PathBuf,
}

fn parse_args(mut it: impl Iterator<Item = String>) -> Result<Args> {
    let mut a = Args {
        url: "https://www.desmos.com/calculator/z7zooq9zsh".into(),
        slider: "t_{animate}".into(),
        frames: 48,
        size: 480,
        delay: 6,
        out: PathBuf::new(),
    };
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .ok_or_else(|| anyhow!("{flag} needs a value\n{USAGE}"))
        };
        match flag.as_str() {
            "--url" => a.url = value()?,
            "--slider" => a.slider = value()?,
            "--frames" => a.frames = value()?.parse()?,
            "--size" => a.size = value()?.parse()?,
            "--delay" => a.delay = value()?.parse()?,
            "--out" => a.out = value()?.into(),
            "-h" | "--help" => bail!("{USAGE}"),
            other => bail!("unknown argument {other}\n{USAGE}"),
        }
    }
    if a.out.as_os_str().is_empty() {
        bail!("--out is required\n{USAGE}");
    }
    if a.frames == 0 || a.size == 0 {
        bail!("--frames and --size must be greater than 0");
    }
    Ok(a)
}

/// Headless Chrome with the DevTools protocol on a WebSocket. The process ends on drop.
struct Chrome {
    child: Child,
    ws: WebSocket<MaybeTlsStream<TcpStream>>,
    next_id: u64,
    profile: PathBuf,
}

impl std::fmt::Debug for Chrome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chrome")
            .field("pid", &self.child.id())
            .finish()
    }
}

impl Drop for Chrome {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

fn chrome_binary() -> Result<PathBuf> {
    if let Some(p) = std::env::var_os("CHROME") {
        return Ok(p.into());
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    for name in [
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
    ] {
        for dir in std::env::split_paths(&path) {
            if dir.join(name).is_file() {
                return Ok(dir.join(name));
            }
        }
    }
    let mac = Path::new("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
    if mac.is_file() {
        return Ok(mac.into());
    }
    bail!("no Chrome or Chromium found; set CHROME to its binary")
}

/// `GET http://127.0.0.1:<port><path>` and the JSON body. Chrome keeps the connection open, so
/// the body ends at its `Content-Length`.
fn http_json(port: u16, path: &str) -> Result<Value> {
    let mut s = TcpStream::connect(("127.0.0.1", port))?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )?;
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = s.read(&mut chunk)?;
        data.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&data);
        if let Some((head, body)) = text.split_once("\r\n\r\n") {
            let len = head
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().to_string())
                })
                .and_then(|v| v.parse::<usize>().ok());
            if n == 0 || len.is_some_and(|len| body.len() >= len) {
                return Ok(serde_json::from_str(body)?);
            }
        } else if n == 0 {
            bail!("bad HTTP answer");
        }
    }
}

impl Chrome {
    /// Starts Chrome on `url` and connects to the page.
    fn open(url: &str, size: u32) -> Result<Self> {
        let profile = std::env::temp_dir().join(format!("desmos-gif-{}", std::process::id()));
        std::fs::create_dir_all(&profile)?;
        let mut cmd = Command::new(chrome_binary()?);
        cmd.arg("--headless=new")
            .arg("--remote-debugging-port=0")
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg(format!("--user-agent={}", smart_alerts_core::USER_AGENT))
            .arg(format!("--window-size={},{}", size + 400, size + 400))
            .args([
                "--no-first-run",
                "--no-default-browser-check",
                "--hide-scrollbars",
            ]);
        if std::env::var_os("CHROME_NO_SANDBOX").is_some() {
            cmd.arg("--no-sandbox");
        }
        let child = cmd
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("start Chrome")?;

        // Chrome writes its DevTools port in the profile, then lists the page at /json/list.
        let start = Instant::now();
        let port: u16 = loop {
            if let Ok(t) = std::fs::read_to_string(profile.join("DevToolsActivePort"))
                && let Some(Ok(p)) = t.lines().next().map(str::parse)
            {
                break p;
            }
            if start.elapsed() > Duration::from_secs(30) {
                bail!("Chrome did not open its DevTools port");
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        let ws_url = loop {
            let list = http_json(port, "/json/list").context("list the DevTools targets")?;
            let page = list
                .as_array()
                .and_then(|l| l.iter().find(|t| t["type"] == "page"));
            if let Some(u) = page.and_then(|t| t["webSocketDebuggerUrl"].as_str()) {
                break u.to_string();
            }
            if start.elapsed() > Duration::from_secs(30) {
                bail!("Chrome has no page target");
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        let (ws, _) = tungstenite::connect(ws_url.as_str()).context("connect to DevTools")?;
        Ok(Self {
            child,
            ws,
            next_id: 0,
            profile,
        })
    }

    /// Evaluates `js` in the page, awaits a promise, and returns the JSON value.
    fn eval(&mut self, js: &str) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let call = json!({"id": id, "method": "Runtime.evaluate",
            "params": {"expression": js, "awaitPromise": true, "returnByValue": true}});
        self.ws.send(Message::text(call.to_string()))?;
        loop {
            let Message::Text(text) = self.ws.read()? else {
                continue;
            };
            let msg: Value = serde_json::from_str(text.as_str())?;
            if msg["id"] != id {
                continue; // an event
            }
            if let Some(e) = msg
                .get("error")
                .or_else(|| msg["result"].get("exceptionDetails"))
            {
                bail!("JavaScript error: {e}");
            }
            return Ok(msg["result"]["result"]["value"].clone());
        }
    }
}

/// Waits until the page has the calculator object `Calc` with a state.
fn wait_for_calc(chrome: &mut Chrome, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        let ready = chrome.eval(
            "typeof Calc === 'object' && typeof Calc.getState === 'function' && Calc.getState().expressions.list.length > 0",
        )?;
        if ready == Value::Bool(true) {
            return Ok(());
        }
        if start.elapsed() > timeout {
            bail!("the page has no Desmos calculator (Calc) after {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

/// The JavaScript that finds the slider by its LaTeX name and stops its animation. It returns
/// `{id, min, max}`, or `{error}`, as JSON text (CDP returns an object by reference, not by value).
fn find_slider_js(slider: &str) -> Result<String> {
    let name = serde_json::to_string(slider)?;
    Ok(format!(
        r#"(() => {{
  const name = {name}.replace(/\s/g, '');
  const e = Calc.getState().expressions.list.find(x =>
    typeof x.latex === 'string' && x.latex.replace(/\s/g, '').startsWith(name + '='));
  if (!e) return JSON.stringify({{ error: 'no slider ' + name }});
  const s = e.slider || {{}};
  const num = (t, d) => {{ const v = Number(t); return Number.isFinite(v) ? v : d; }};
  Calc.setExpression({{ id: e.id, playing: false }});
  return JSON.stringify({{ id: e.id, min: num(s.min, 0), max: num(s.max, 1) }});
}})()"#
    ))
}

/// Sets the slider to `v` and resolves with the calculator's PNG screenshot as a data URI.
fn frame_js(id: &str, slider: &str, v: f64, size: u32) -> Result<String> {
    let id = serde_json::to_string(id)?;
    let latex = serde_json::to_string(&format!("{slider}={v}"))?;
    Ok(format!(
        r#"new Promise(resolve => {{
  Calc.setExpression({{ id: {id}, latex: {latex} }});
  Calc.asyncScreenshot({{ width: {size}, height: {size}, targetPixelRatio: 1 }}, resolve);
}})"#
    ))
}

/// Decodes a `data:image/png;base64,...` URI to RGBA pixels of the given size.
fn decode_png(uri: &str, size: u32) -> Result<Vec<u8>> {
    let b64 = uri
        .strip_prefix("data:image/png;base64,")
        .ok_or_else(|| anyhow!("the screenshot is not a PNG data URI"))?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info()?;
    let mut buf = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or_else(|| anyhow!("PNG too large"))?
    ];
    let info = reader.next_frame(&mut buf)?;
    if info.width != size || info.height != size {
        bail!(
            "the screenshot is {}x{}, not {size}x{size}",
            info.width,
            info.height
        );
    }
    let px = &buf[..info.buffer_size()];
    Ok(match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px.chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::Grayscale => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => px
            .chunks(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Indexed => bail!("indexed PNG was not expanded"),
    })
}

fn record(a: &Args) -> Result<Vec<Vec<u8>>> {
    let mut chrome = Chrome::open(&a.url, a.size)?;
    wait_for_calc(&mut chrome, Duration::from_secs(60))?;

    let slider = chrome.eval(&find_slider_js(&a.slider)?)?;
    let slider: Value = serde_json::from_str(slider.as_str().unwrap_or("null"))?;
    if let Some(e) = slider.get("error") {
        bail!("{e}");
    }
    let field = |k: &str| slider.get(k).cloned().ok_or_else(|| anyhow!("{slider}"));
    let id = field("id")?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("{slider}"))?;
    let (min, max) = (
        field("min")?.as_f64().unwrap_or(0.0),
        field("max")?.as_f64().unwrap_or(1.0),
    );

    let mut frames = Vec::new();
    for i in 0..a.frames {
        // One loop: the last frame stops one step before `max`, which is the first frame again.
        let v = min + (max - min) * f64::from(i) / f64::from(a.frames);
        let uri = chrome.eval(&frame_js(&id, &a.slider, v, a.size)?)?;
        let uri = uri
            .as_str()
            .ok_or_else(|| anyhow!("no screenshot at {v}"))?;
        frames.push(decode_png(uri, a.size)?);
        eprintln!(
            "desmos-gif: frame {}/{} ({}={v:.4})",
            i + 1,
            a.frames,
            a.slider
        );
    }
    Ok(frames)
}

fn write_gif(a: &Args, frames: Vec<Vec<u8>>) -> Result<()> {
    let side = u16::try_from(a.size).context("--size must fit in 16 bits")?;
    let tmp = a.out.with_extension("gif.tmp");
    {
        let file = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        let mut enc = gif::Encoder::new(file, side, side, &[])?;
        enc.set_repeat(gif::Repeat::Infinite)?;
        for mut rgba in frames {
            let mut frame = gif::Frame::from_rgba_speed(side, side, &mut rgba, 10);
            frame.delay = a.delay;
            enc.write_frame(&frame)?;
        }
    }
    std::fs::rename(&tmp, &a.out)?;
    Ok(())
}

fn main() -> Result<()> {
    let a = parse_args(std::env::args().skip(1))?;
    let frames = record(&a)?;
    write_gif(&a, frames)?;
    eprintln!("desmos-gif: {}", a.out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn args(s: &str) -> Result<Args> {
        parse_args(s.split_whitespace().map(String::from))
    }

    #[test]
    fn defaults_record_ammonite() {
        let a = args("--out x.gif").unwrap();
        assert!(a.url.ends_with("z7zooq9zsh"));
        assert_eq!(
            (a.slider.as_str(), a.frames, a.size),
            ("t_{animate}", 48, 480)
        );
    }

    #[test]
    fn out_is_required() {
        assert!(args("--frames 2").is_err());
        assert!(args("--frames 0 --out x.gif").is_err());
    }

    #[test]
    fn frame_js_quotes_the_latex() {
        let js = frame_js("12", "t_{animate}", 0.5, 64).unwrap();
        assert!(js.contains(r#"latex: "t_{animate}=0.5""#));
    }
}
