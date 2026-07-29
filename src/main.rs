mod support;

use enigo::{Coordinate, Enigo, Mouse, Settings as EnigoSettings};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// settings shared between the UI thread and the background jiggler thread.
// plain atomics are enough here (no locking needed) since every field is a
// single primitive value read/written independently.
struct SharedSettings {
  move_pixels: AtomicI32,
  interval_secs: AtomicU64,
  running: AtomicBool,
}

impl Default for SharedSettings {
  fn default() -> Self {
    Self {
      move_pixels: AtomicI32::new(10),
      interval_secs: AtomicU64::new(60),
      running: AtomicBool::new(true),
    }
  }
}

// spawns the background thread that actually moves the mouse.
// all former `println!` calls are now sent down `log_tx` instead, so the
// UI thread can pick them up and render them in the console window.
fn spawn_jiggler(settings: Arc<SharedSettings>, log_tx: Sender<String>) {
  thread::spawn(move || {
    let mut enigo = match Enigo::new(&EnigoSettings::default()) {
      Ok(e) => e,
      Err(e) => {
        let _ = log_tx.send(format!("[error] Failed to initialize Enigo: {e}"));
        return;
      }
    };

    let _ = log_tx.send("Cursor Mover started!".to_string());

    loop {
      if settings.running.load(Ordering::Relaxed) {
        let px = settings.move_pixels.load(Ordering::Relaxed);
        match enigo.move_mouse(px, px, Coordinate::Rel) {
          Ok(_) => {
            let _ = log_tx.send(format!("Cursor moved by ({px}, {px})"));
          }
          Err(e) => {
            let _ = log_tx.send(format!("[error] Failed to move cursor: {e}"));
          }
        }
      }

      let interval = settings.interval_secs.load(Ordering::Relaxed).max(1);
      thread::sleep(Duration::from_secs(interval));
    }
  });
}

fn main() {
  let system = support::init(file!());

  let settings = Arc::new(SharedSettings::default());
  let (log_tx, log_rx): (Sender<String>, Receiver<String>) = mpsc::channel();

  spawn_jiggler(settings.clone(), log_tx.clone());

  let mut log_lines: Vec<String> = Vec::new();
  let mut autoscroll = true;

  // local mirrors so the input widgets have somewhere to write to each frame.
  let mut move_pixels_input = settings.move_pixels.load(Ordering::Relaxed);
  let mut interval_input = settings.interval_secs.load(Ordering::Relaxed) as i32;

  let mut layout = support::WindowLayout::load();

  system.main_loop(move |_, ui| {
    // drain whatever log messages have piled up since the last frame.
    // try_recv() never blocks, so this is safe to call every frame.
    while let Ok(line) = log_rx.try_recv() {
      log_lines.push(line);
      if log_lines.len() > 1000 {
        log_lines.remove(0);
      }
    }

    ui.dockspace_over_main_viewport();

    layout.window(ui, "Settings", [20.0, 20.0], [340.0, 260.0], || {
      let is_running = settings.running.load(Ordering::Relaxed);
      ui.text(format!(
          "Status: {}",
          if is_running { "Running" } else { "Paused" }
      ));
      ui.separator();

      if ui.input_int("Move pixels", &mut move_pixels_input).build() {
        settings
          .move_pixels
          .store(move_pixels_input, Ordering::Relaxed);
      }

      if ui
        .input_int("Interval (seconds)", &mut interval_input)
          .build()
      {
        if interval_input < 1 {
          interval_input = 1;
        }
        settings
          .interval_secs
          .store(interval_input as u64, Ordering::Relaxed);
        }

      ui.separator();

      if ui.button(if is_running { "Pause" } else { "Resume" }) {
        settings.running.store(!is_running, Ordering::Relaxed);
      }
      ui.same_line();
      if ui.button("Clear log") {
        log_lines.clear();
      }

      ui.separator();
      let mouse_pos = ui.io().mouse_pos;
      ui.text(format!(
          "Mouse Position: ({:.1}, {:.1})",
          mouse_pos[0], mouse_pos[1]
      ));
    });

    layout.window(ui, "Console", [380.0, 20.0], [520.0, 320.0], || {
      ui.checkbox("Autoscroll", &mut autoscroll);
      ui.separator();

      let bg_token =
        ui.push_style_color(imgui::StyleColor::ChildBg, [0.05, 0.05, 0.05, 1.0]);

      ui.child_window("console_output")
        .size([0.0, 0.0])
        .build(|| {
          for line in &log_lines {
            ui.text_colored([0.4, 0.9, 0.4, 1.0], line);
          }
          if autoscroll && ui.scroll_y() >= ui.scroll_max_y() {
            ui.set_scroll_here_y();
          }
        });

      bg_token.pop();
    });

    layout.end_frame();
  });
}
