use std::thread;
use std::time::Duration;
use enigo::{Enigo, Mouse, Settings};

const MOVE_PIXELS: i32 = 10;
const INTERVAL_SECONDS: u64 = 60;

fn main() {
    println!("Cursor Mover started!");
    println!("Moving cursor by {} pixels every {} seconds", MOVE_PIXELS, INTERVAL_SECONDS);
    println!("Press Ctrl+C to stop\n");

    let mut enigo = Enigo::new(&Settings::default()).expect("Failed to initialize Enigo");

    loop {
        // Move cursor relative to current position
        enigo.move_mouse(MOVE_PIXELS, MOVE_PIXELS, enigo::Coordinate::Rel)
            .expect("Failed to move cursor");
        
        println!("Cursor moved successfully");
        thread::sleep(Duration::from_secs(INTERVAL_SECONDS));
    }
}
