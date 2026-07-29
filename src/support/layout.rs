//! Hand-rolled window position/size persistence.
//!
//! This deliberately does NOT use imgui's built-in .ini save/load: reloading
//! saved window/dock settings through imgui's own serializer has been
//! observed to corrupt internal draw-data state on the *second* run with
//! this docking fork (crash in draw_data.rs, unaligned pointer in
//! slice::from_raw_parts). Instead we remember a simple name -> rectangle
//! map ourselves, in a plain text file next to the executable, and apply it
//! by hand with `.position()` / `.size()` on the first frame only.
//!
//! Usage in an app's main loop:
//!
//! ```ignore
//! let mut layout = support::WindowLayout::load();
//!
//! system.main_loop(move |_, ui| {
//!     layout.window(ui, "Settings", [20.0, 20.0], [340.0, 260.0], || {
//!         ui.text("Hello!");
//!     });
//!
//!     layout.window(ui, "Console", [380.0, 20.0], [520.0, 320.0], || {
//!         ui.text("Some log output");
//!     });
//!
//!     layout.end_frame();
//! });
//! ```

use imgui::{Condition, Ui};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
struct Rect {
    pos: [f32; 2],
    size: [f32; 2],
}

pub struct WindowLayout {
    rects: HashMap<String, Rect>,
    first_frame: bool,
    last_save: Instant,
    path: PathBuf,
}

impl WindowLayout {
    /// Loads any previously-saved layout from disk. Missing or malformed
    /// data just means windows fall back to whatever default you pass to
    /// `window()` - this never panics.
    pub fn load() -> Self {
        let path = layout_path();
        let rects = read(&path).unwrap_or_default();
        Self {
            rects,
            first_frame: true,
            last_save: Instant::now(),
            path,
        }
    }

    /// Declares a persisted window. On the very first frame after `load()`,
    /// the window is forced to its saved (or default) rectangle. On every
    /// later frame no position/size is passed at all, so imgui just keeps
    /// whatever is already in memory - which is how the user's live
    /// dragging/resizing shows up without fighting it every frame.
    ///
    /// `content` builds the window body exactly like a normal
    /// `.build(|| { ... })` closure would.
    pub fn window(
        &mut self,
        ui: &Ui,
        title: &str,
        default_pos: [f32; 2],
        default_size: [f32; 2],
        content: impl FnOnce(),
    ) {
        let rect = self.rects.get(title).copied().unwrap_or(Rect {
            pos: default_pos,
            size: default_size,
        });
        let condition = if self.first_frame {
            Condition::Always
        } else {
            Condition::FirstUseEver
        };

        ui.window(title)
            .position(rect.pos, condition)
            .size(rect.size, condition)
            .build(|| {
                content();
                self.rects.insert(
                    title.to_string(),
                    Rect {
                        pos: ui.window_pos(),
                        size: ui.window_size(),
                    },
                );
            });
    }

    /// Call once per frame, after every `window()` call for that frame.
    /// Flips off the first-frame forcing and throttles the on-disk save to
    /// roughly once a second (this is a real file write - no need to hit
    /// disk 60+ times a second while nothing has changed).
    pub fn end_frame(&mut self) {
        self.first_frame = false;
        if self.last_save.elapsed() >= Duration::from_secs(1) {
            write(&self.path, &self.rects);
            self.last_save = Instant::now();
        }
    }
}

fn layout_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.join("window_layout.txt");
        }
    }
    PathBuf::from("window_layout.txt")
}

fn read(path: &Path) -> Option<HashMap<String, Rect>> {
    let contents = std::fs::read_to_string(path).ok()?;
    let mut map = HashMap::new();
    for line in contents.lines() {
        let mut parts = line.split('\t');
        let name = parts.next()?;
        let x: f32 = parts.next()?.parse().ok()?;
        let y: f32 = parts.next()?.parse().ok()?;
        let w: f32 = parts.next()?.parse().ok()?;
        let h: f32 = parts.next()?.parse().ok()?;
        map.insert(
            name.to_string(),
            Rect {
                pos: [x, y],
                size: [w, h],
            },
        );
    }
    Some(map)
}

fn write(path: &Path, rects: &HashMap<String, Rect>) {
    let mut out = String::new();
    for (name, rect) in rects {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            name, rect.pos[0], rect.pos[1], rect.size[0], rect.size[1]
        ));
    }
    let _ = std::fs::write(path, out);
}
