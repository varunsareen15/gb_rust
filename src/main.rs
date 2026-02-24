mod cpu;
mod cartridge;
mod timer;
mod ppu;
mod joypad;
mod gameboy;

use cartridge::Cartridge;
use gameboy::GameBoy;
use joypad::JoypadKey;

use minifb::{Key, Window, WindowOptions, Scale};
use std::time::{Duration, Instant};

const GB_COLORS: [u32; 4] = [
    0x00E0F8D0, // lightest (white)
    0x0088C070, // light
    0x00346856, // dark
    0x00081820, // darkest (black)
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <rom.gb>", args[0]);
        std::process::exit(1);
    }

    let cartridge = Cartridge::from_file(&args[1]).unwrap_or_else(|e| {
        eprintln!("Error loading ROM: {}", e);
        std::process::exit(1);
    });

    println!("Title: {}", cartridge.title);
    println!("Type: 0x{:02X}", cartridge.cartridge_type);

    let mut gb = GameBoy::new(cartridge);

    let mut window = Window::new(
        "GB Emulator",
        160,
        144,
        WindowOptions {
            scale: Scale::X4,
            ..WindowOptions::default()
        },
    ).expect("Failed to create window");

    let frame_duration = Duration::from_nanos(16_742_706); // ~59.7 Hz
    let mut buffer = vec![0u32; 160 * 144];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();

        // Handle input
        update_joypad(&window, &mut gb);

        // Run one frame
        gb.run_frame();

        // Convert framebuffer to u32 colors
        let fb = gb.framebuffer();
        for (i, &pixel) in fb.iter().enumerate() {
            buffer[i] = GB_COLORS[(pixel & 0x03) as usize];
        }

        window.update_with_buffer(&buffer, 160, 144).unwrap();

        // Frame timing
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }
}

fn update_joypad(window: &Window, gb: &mut GameBoy) {
    let key_map: &[(Key, JoypadKey)] = &[
        (Key::Right, JoypadKey::Right),
        (Key::Left, JoypadKey::Left),
        (Key::Up, JoypadKey::Up),
        (Key::Down, JoypadKey::Down),
        (Key::Z, JoypadKey::A),
        (Key::X, JoypadKey::B),
        (Key::Backspace, JoypadKey::Select),
        (Key::Enter, JoypadKey::Start),
    ];

    for &(key, joypad_key) in key_map {
        if window.is_key_down(key) {
            gb.cpu.bus.joypad.key_down(joypad_key);
        } else {
            gb.cpu.bus.joypad.key_up(joypad_key);
        }
    }
}
