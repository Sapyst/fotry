//! Fotry (Arch Quilly): Image › Upscale…, enlarging the whole document 2×, 3× or 4×.
//!
//! - `lanczos` and `preserveDetails` are Image Size with that filter: every layer, mask, vector,
//!   effect and guide scales together.
//! - `aiPhoto` and `aiArt` do the same first, then redraw every pixel layer from its original
//!   pixels with Real-ESRGAN (realesrgan-ncnn-vulkan, on the GPU through Vulkan): a neural network
//!   that invents plausible fine detail instead of blurring it. The engine is an external one-shot
//!   process per layer, so nothing stays in memory between uses. It lives in
//!   `~/.local/share/arch-quilly/fotry/realesrgan`; `image.upscale.installEngine` downloads the
//!   official release there (about 45 MB).
//!
//! The whole upscale is one background job and one undo step. The AI works on 8-bit RGBA copies
//! of the layers; transparency follows the Lanczos-resized alpha when the engine drops it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use photocraft_doc::{Document, LayerContent, LayerId};
use photocraft_geom::Rect;
use photocraft_raster::Surface;
use serde_json::{Value, json};

use crate::commands::CommandSpec;
use crate::{EngineError, Result, Session};

const ENGINE_URL: &str = "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-ubuntu.zip";
const ENGINE_BIN: &str = "realesrgan-ncnn-vulkan";

fn has_doc(s: &Session) -> std::result::Result<(), String> {
    s.active().map(|_| ()).ok_or_else(|| "no document open".into())
}

fn always(_: &Session) -> std::result::Result<(), String> {
    Ok(())
}

fn not_installed(_: &Session) -> std::result::Result<(), String> {
    if engine().is_some() { Err("the AI upscale engine is already installed".into()) } else { Ok(()) }
}

fn bad(msg: impl Into<String>) -> EngineError {
    EngineError::BadParams { cmd: "image.upscale".into(), msg: msg.into() }
}

/// `~/.local/share/arch-quilly/fotry/realesrgan` (XDG_DATA_HOME respected).
fn engine_dir() -> Option<PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    let dir = data.join("arch-quilly/fotry/realesrgan");
    // The engine Array (the app's earlier name) installed is used where it already is.
    let legacy = data.join("arch-quilly/array/realesrgan");
    Some(if !dir.join(ENGINE_BIN).is_file() && legacy.join(ENGINE_BIN).is_file() { legacy } else { dir })
}

/// The engine binary and its models folder, when installed.
fn engine() -> Option<(PathBuf, PathBuf)> {
    let dir = engine_dir()?;
    let bin = dir.join(ENGINE_BIN);
    let models = dir.join("models");
    (bin.is_file() && models.join("realesrgan-x4plus.param").is_file()).then_some((bin, models))
}

fn cache_dir() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .ok_or_else(|| EngineError::Other("no home folder for the upscale cache".into()))?;
    // A per-process folder on disk (not /tmp, which is RAM here), removed when the job ends.
    let dir = base.join("arch-quilly/fotry/upscale").join(format!("{}-{}", std::process::id(), next_job()));
    std::fs::create_dir_all(&dir).map_err(|e| EngineError::Other(format!("upscale cache: {e}")))?;
    Ok(dir)
}

fn next_job() -> u64 {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone, Copy, PartialEq)]
enum Method {
    Lanczos,
    PreserveDetails,
    Ai(&'static str),
}

fn parse(p: &Value) -> Result<(u32, Method)> {
    let scale = match p.get("scale") {
        None | Some(Value::Null) => 2,
        Some(Value::String(s)) => s.trim_end_matches(['x', '×']).parse::<u32>().map_err(|_| bad("`scale` must be 2, 3 or 4"))?,
        Some(v) => v.as_u64().and_then(|n| u32::try_from(n).ok()).ok_or_else(|| bad("`scale` must be 2, 3 or 4"))?,
    };
    if !(2..=4).contains(&scale) {
        return Err(bad("`scale` must be 2, 3 or 4"));
    }
    let method = match p.get("method").and_then(Value::as_str).unwrap_or("aiPhoto") {
        "lanczos" => Method::Lanczos,
        "preserveDetails" => Method::PreserveDetails,
        "aiPhoto" | "ai" => Method::Ai("realesrgan-x4plus"),
        "aiArt" => Method::Ai("realesrgan-x4plus-anime"),
        other => return Err(bad(format!("unknown method `{other}` (aiPhoto, aiArt, lanczos or preserveDetails)"))),
    };
    Ok((scale, method))
}

/// The pixel layers of `doc` (groups searched too) with their content bounds.
fn pixel_layers(doc: &Document) -> Vec<(LayerId, Rect, Surface)> {
    fn walk(layers: &[photocraft_doc::Layer], out: &mut Vec<(LayerId, Rect, Surface)>) {
        for l in layers {
            if let LayerContent::Raster(s) = &l.content {
                let r = s.content_bounds();
                if !r.is_empty() {
                    out.push((l.id, r, s.clone()));
                }
            }
            if let Some(ch) = l.children() {
                walk(ch, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(&doc.layers, &mut out);
    out
}

fn write_png(path: &Path, surf: &Surface, r: Rect) -> Result<()> {
    let (w, h) = (r.width(), r.height());
    let n = (w as usize).checked_mul(h as usize).ok_or_else(|| bad("layer too large"))?;
    let mut px = vec![[0u8; 4]; n];
    surf.read_rgba8_into(r, &mut px);
    let img = photocraft_codecs::Image::from_u8(w, h, photocraft_codecs::ChannelLayout::Rgba, px.into_flattened())
        .map_err(|e| EngineError::Other(format!("upscale: {e}")))?;
    let bytes = photocraft_codecs::encode(&img, photocraft_codecs::Format::Png, &photocraft_codecs::EncodeOptions::default())
        .map_err(|e| EngineError::Other(format!("upscale: {e}")))?;
    std::fs::write(path, bytes).map_err(|e| EngineError::Other(format!("upscale: {e}")))
}

/// Runs the engine on one PNG, following its "12.34%" progress lines; kills it on cancel.
fn run_engine(bin: &Path, models: &Path, model: &str, scale: u32, input: &Path, output: &Path, ctx: &crate::jobs::JobCtx, lo: f32, hi: f32, label: &str) -> Result<()> {
    use std::io::Read;
    let mut child = Command::new(bin)
        .arg("-i").arg(input)
        .arg("-o").arg(output)
        .arg("-n").arg(model)
        .arg("-s").arg(scale.to_string())
        .arg("-m").arg(models)
        .arg("-f").arg("png")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| EngineError::Other(format!("could not start the AI engine: {e}")))?;
    let mut err = child.stderr.take().ok_or_else(|| EngineError::Other("AI engine: no output".into()))?;
    let mut buf = [0u8; 256];
    let mut line = String::new();
    loop {
        if ctx.cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(EngineError::Other("cancelled".into()));
        }
        let n = err.read(&mut buf).unwrap_or(0);
        if n == 0 {
            break;
        }
        for c in String::from_utf8_lossy(buf.get(..n).unwrap_or(&[])).chars() {
            if c == '\n' {
                if let Some(pc) = line.trim().strip_suffix('%').and_then(|v| v.parse::<f32>().ok()) {
                    ctx.progress(lo + (hi - lo) * (pc / 100.0).clamp(0.0, 1.0), label);
                }
                line.clear();
            } else {
                line.push(c);
            }
        }
    }
    let status = child.wait().map_err(|e| EngineError::Other(format!("AI engine: {e}")))?;
    if !status.success() || !output.is_file() {
        return Err(EngineError::Other(format!("the AI engine failed ({status})")));
    }
    Ok(())
}

/// Puts the AI result for one layer into the (already resized) layer: colour from the AI, alpha
/// from the AI when it kept one, otherwise from the resized layer.
fn paste_result(surf: &mut Surface, at: Rect, png: &Path) -> Result<()> {
    let bytes = std::fs::read(png).map_err(|e| EngineError::Other(format!("upscale: {e}")))?;
    let src = photocraft_codecs::decode(&bytes).map_err(|e| EngineError::Other(format!("upscale: {e}")))?;
    use photocraft_codecs::ChannelLayout as L;
    let had_alpha = matches!(src.layout(), L::GrayA | L::Rgba | L::CmykA);
    let src = src.convert(photocraft_codecs::ChannelLayout::Rgba, photocraft_codecs::SampleType::U8);
    let (w, h) = (src.width().min(at.width()), src.height().min(at.height()));
    let r = Rect::new(at.x0, at.y0, at.x0 + w as i32, at.y0 + h as i32);
    let n = (w as usize) * (h as usize);
    let mut base = vec![[0f32; 4]; n];
    surf.read_rgba_into(r, &mut base);
    let fmt = surf.format();
    let ch = surf.channels();
    let mut data = vec![0f32; n * ch];
    let sw = src.width() as usize;
    let px = src.data();
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * w as usize + x;
            let o = (y * sw + x) * 4;
            let Some(s) = px.get(o..o + 4) else { continue };
            let alpha = if had_alpha { f32::from(s[3]) / 255.0 } else { base.get(i).map_or(1.0, |b| b[3]) };
            let rgba = [f32::from(s[0]) / 255.0, f32::from(s[1]) / 255.0, f32::from(s[2]) / 255.0, alpha];
            if let Some(out) = data.get_mut(i * ch..(i + 1) * ch) {
                photocraft_raster::from_rgba_into(&fmt, rgba, out);
            }
        }
    }
    surf.write_region(r, &data);
    Ok(())
}

fn upscale(s: &mut Session, p: &Value) -> Result<Value> {
    let (scale, method) = parse(p)?;
    let d = s.active().ok_or(EngineError::NoDocument)?;
    let (ow, oh) = (d.doc.size.width, d.doc.size.height);
    let (nw, nh) = (ow.saturating_mul(scale), oh.saturating_mul(scale));
    crate::image_cmds::check_resample_budget("image.upscale", &d.doc, nw, nh, "")?;
    let filter = match method {
        Method::PreserveDetails => photocraft_algo::resample::Resample::PreserveDetails,
        _ => photocraft_algo::resample::Resample::Lanczos,
    };
    let ai = match method {
        Method::Ai(model) => Some((model, engine().ok_or_else(|| bad("the AI engine is not installed: use Image › Install AI Upscale Engine… first"))?)),
        _ => None,
    };
    let label = format!("Upscale {scale}×");
    crate::jobs::edit_job(
        s,
        "Upscale",
        move |doc, _active, ctx| {
            let originals = if ai.is_some() { pixel_layers(doc) } else { Vec::new() };
            ctx.progress(0.02, &label);
            crate::image_cmds::resample_doc(doc, nw, nh, filter);
            let Some((model, (bin, models))) = ai else { return Ok(()) };
            let dir = cache_dir()?;
            let total = originals.len().max(1) as f32;
            let result = (|| -> Result<()> {
                for (i, (id, r, surf)) in originals.iter().enumerate() {
                    ctx.check()?;
                    let lo = 0.05 + 0.95 * i as f32 / total;
                    let hi = 0.05 + 0.95 * (i + 1) as f32 / total;
                    let step = format!("{label}: AI, layer {} of {}", i + 1, originals.len());
                    ctx.progress(lo, &step);
                    let input = dir.join(format!("{i}-in.png"));
                    let output = dir.join(format!("{i}-out.png"));
                    write_png(&input, surf, *r)?;
                    run_engine(&bin, &models, model, scale, &input, &output, ctx, lo, hi, &step)?;
                    let at = Rect::new(r.x0 * scale as i32, r.y0 * scale as i32, r.x1 * scale as i32, r.y1 * scale as i32);
                    let mut done = false;
                    crate::image_cmds::for_each_layer(&mut doc.layers, &mut |l| {
                        if l.id == *id
                            && let LayerContent::Raster(target) = &mut l.content
                        {
                            done = paste_result(target, at, &output).is_ok();
                        }
                    });
                    if !done {
                        return Err(EngineError::Other("upscale: could not place the AI result".into()));
                    }
                    let _ = std::fs::remove_file(&input);
                    let _ = std::fs::remove_file(&output);
                }
                Ok(())
            })();
            let _ = std::fs::remove_dir_all(&dir);
            result
        },
        move |()| json!({"width": nw, "height": nh, "scale": scale}),
    )
}

fn engine_status(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"installed": engine().is_some(), "folder": engine_dir().map(|d| d.display().to_string()), "sizeMb": 45}))
}

/// Downloads the official Real-ESRGAN release (curl) and unpacks only the binary and the models
/// (bsdtar). Runs as a background job.
fn install_engine(s: &mut Session, _: &Value) -> Result<Value> {
    if engine().is_some() {
        return Ok(json!({"installed": true}));
    }
    let dir = engine_dir().ok_or_else(|| EngineError::Other("no home folder".into()))?;
    crate::jobs::run(
        s,
        "Install AI Engine",
        false,
        move |ctx| {
            ctx.progress(0.05, "Downloading the AI engine (45 MB)…");
            std::fs::create_dir_all(&dir).map_err(|e| EngineError::Other(format!("{}: {e}", dir.display())))?;
            let zip = dir.join("download.zip");
            let ok = Command::new("curl")
                .args(["-fsSL", "--retry", "2", "-A", "fotry/2.0", "-o"])
                .arg(&zip)
                .arg(ENGINE_URL)
                .status()
                .map_err(|e| EngineError::Other(format!("curl: {e}")))?;
            if !ok.success() {
                let _ = std::fs::remove_file(&zip);
                return Err(EngineError::Other("could not download the AI engine (check the internet connection)".into()));
            }
            ctx.check()?;
            ctx.progress(0.8, "Unpacking…");
            let ok = Command::new("bsdtar")
                .arg("-xf")
                .arg(&zip)
                .arg("-C")
                .arg(&dir)
                .args([ENGINE_BIN, "models/*.bin", "models/*.param"])
                .status()
                .map_err(|e| EngineError::Other(format!("bsdtar: {e}")))?;
            let _ = std::fs::remove_file(&zip);
            if !ok.success() {
                return Err(EngineError::Other("could not unpack the AI engine".into()));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(dir.join(ENGINE_BIN), std::fs::Permissions::from_mode(0o755));
            }
            Ok(())
        },
        |_, ()| Ok(json!({"installed": engine().is_some()})),
    )
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            id: "image.upscale",
            label: "Upscale…",
            menu: &["Image"],
            shortcut: Some("Cmd+Alt+U"),
            params: r##"{"scale":2|3|4=2,"method":"aiPhoto|aiArt|lanczos|preserveDetails"="aiPhoto"} → {width,height,scale} (the whole document, one undo step; aiPhoto/aiArt redraw every pixel layer with Real-ESRGAN, the others are Image Size with that filter)"##,
            enabled: has_doc,
            run: upscale,
            journal: true,
        },
        CommandSpec {
            id: "image.upscale.engineStatus",
            label: "AI Engine Status",
            menu: &[],
            shortcut: None,
            params: r##"{} → {installed, folder, sizeMb}"##,
            enabled: always,
            run: engine_status,
            journal: false,
        },
        CommandSpec {
            id: "image.upscale.installEngine",
            label: "Install AI Upscale Engine…",
            menu: &["Image"],
            shortcut: None,
            params: r##"{} → {installed} (downloads Real-ESRGAN, about 45 MB, from its official GitHub release)"##,
            enabled: not_installed,
            run: install_engine,
            journal: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_params_are_errors_not_panics() {
        for p in [json!({"scale": 9}), json!({"scale": "x"}), json!({"scale": -1}), json!({"method": "nope"}), json!({"scale": 1e300})] {
            assert!(parse(&p).is_err(), "{p}");
        }
        assert!(matches!(parse(&json!({"scale": "3x", "method": "lanczos"})), Ok((3, Method::Lanczos))));
        assert!(matches!(parse(&json!({})), Ok((2, Method::Ai(_)))));
    }

    #[test]
    fn lanczos_upscale_doubles_the_document() {
        let mut s = Session::new();
        s.execute("file.new", json!({"width": 40, "height": 30, "background": "white"})).unwrap();
        let r = s.execute("image.upscale", json!({"scale": 2, "method": "lanczos"})).unwrap();
        assert_eq!(r["width"], 80);
        let d = &s.active().unwrap().doc;
        assert_eq!((d.size.width, d.size.height), (80, 60));
    }
}
