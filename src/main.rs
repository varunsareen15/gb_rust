mod cpu;
mod cartridge;
mod timer;
mod ppu;
mod joypad;
mod gameboy;
mod savestate;
mod apu;

use cartridge::Cartridge;
use gameboy::GameBoy;
use joypad::JoypadKey;

use minifb::{Key, Window, WindowOptions, Scale};
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const GB_COLORS: [u32; 4] = [
    0x00E0F8D0, // lightest (white)
    0x0088C070, // light
    0x00346856, // dark
    0x00081820, // darkest (black)
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let headless = args.iter().any(|a| a == "--headless");
    let rom_args: Vec<&String> = args.iter().skip(1).filter(|a| *a != "--headless").collect();

    if rom_args.is_empty() {
        eprintln!("Usage: {} [--headless] <rom.gb>", args[0]);
        std::process::exit(1);
    }

    let cartridge = Cartridge::from_file(rom_args[0]).unwrap_or_else(|e| {
        eprintln!("Error loading ROM: {}", e);
        std::process::exit(1);
    });

    println!("Title: {}", cartridge.title);
    println!("Type: 0x{:02X}", cartridge.cartridge_type);

    let mut gb = GameBoy::new(cartridge);

    if headless {
        run_headless(&mut gb);
    } else {
        run_windowed(&mut gb);
    }

    if let Err(e) = gb.cpu.bus.cartridge.save() {
        eprintln!("Error saving: {}", e);
    }
}

fn run_headless(gb: &mut GameBoy) {
    // No audio output in headless mode
    gb.cpu.bus.apu.set_sample_rate(0);

    // Run for up to ~60 seconds of emulated time (~3600 frames)
    // Stop early if Blargg memory-mapped result is available
    for _ in 0..3600 {
        gb.run_frame();
        // Clear sample buffer periodically (no audio output)
        gb.cpu.bus.apu.sample_buffer.clear();

        // Check for Blargg memory-mapped result signature at $A001-$A003
        let sig = [
            gb.cpu.bus.cartridge.read_byte(0xA001),
            gb.cpu.bus.cartridge.read_byte(0xA002),
            gb.cpu.bus.cartridge.read_byte(0xA003),
        ];
        if sig == [0xDE, 0xB0, 0x61] {
            let status = gb.cpu.bus.cartridge.read_byte(0xA000);
            if status != 0x80 {
                // Test finished — print result string from $A004
                let mut addr = 0xA004u16;
                loop {
                    let ch = gb.cpu.bus.cartridge.read_byte(addr);
                    if ch == 0 { break; }
                    eprint!("{}", ch as char);
                    addr += 1;
                    if addr > 0xBFFF { break; }
                }
                eprintln!();
                break;
            }
        }
    }

    // Dump VRAM tile map as ASCII (for screen-only test ROMs like halt_bug)
    // Blargg uses tiles where tile index maps to ASCII code
    let tilemap_base = 0x1800usize; // $9800 in VRAM
    let mut has_text = false;
    for row in 0..18 {
        let mut line = String::new();
        for col in 0..20 {
            let tile = gb.cpu.bus.vram[tilemap_base + row * 32 + col];
            if tile >= 0x20 && tile < 0x7F {
                line.push(tile as char);
                has_text = true;
            } else if tile == 0 {
                line.push(' ');
            } else {
                line.push(' ');
            }
        }
        if has_text {
            eprintln!("{}", line.trim_end());
        }
    }

    eprintln!();
}

fn run_windowed(gb: &mut GameBoy) {
    // Set up audio output via cpal
    let audio_buffer: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
    let _stream = setup_audio(gb, &audio_buffer);

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
        update_joypad(&window, gb);

        // Save states
        if window.is_key_pressed(Key::F5, minifb::KeyRepeat::No) {
            if let Err(e) = gb.save_state_to_slot(0) {
                eprintln!("Save state error: {}", e);
            }
        }
        if window.is_key_pressed(Key::F8, minifb::KeyRepeat::No) {
            if let Err(e) = gb.load_state_from_slot(0) {
                eprintln!("Load state error: {}", e);
            }
        }

        // Run one frame
        gb.run_frame();

        // Drain APU samples into audio buffer
        drain_audio_samples(gb, &audio_buffer);

        // Convert framebuffer to u32 colors
        let fb = gb.framebuffer();
        for (i, &pixel) in fb.iter().enumerate() {
            buffer[i] = GB_COLORS[(pixel & 0x03) as usize];
        }

        window.update_with_buffer(&buffer, 160, 144).unwrap();

        // Frame timing: sleep most of the wait, then spin-wait for precision
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            let remaining = frame_duration - elapsed;
            if remaining > Duration::from_millis(1) {
                std::thread::sleep(remaining - Duration::from_millis(1));
            }
            while frame_start.elapsed() < frame_duration {
                std::hint::spin_loop();
            }
        }
    }
}

fn setup_audio(gb: &mut GameBoy, audio_buffer: &Arc<Mutex<VecDeque<f32>>>) -> Option<cpal::Stream> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None => {
            eprintln!("No audio output device found");
            return None;
        }
    };

    let config = match device.default_output_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to get audio config: {}", e);
            return None;
        }
    };

    let sample_rate = config.sample_rate().0;
    gb.cpu.bus.apu.set_sample_rate(sample_rate);

    let buffer_clone = audio_buffer.clone();
    let last_sample: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
    let last_sample_clone = last_sample.clone();
    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let mut buffer = buffer_clone.lock().unwrap();
            let drain_count = data.len().min(buffer.len());
            for i in 0..drain_count {
                data[i] = buffer.pop_front().unwrap();
            }
            // On underrun, hold last sample to avoid pops
            let hold = if drain_count > 0 {
                let v = data[drain_count - 1];
                *last_sample_clone.lock().unwrap() = v;
                v
            } else {
                *last_sample_clone.lock().unwrap()
            };
            for sample in data[drain_count..].iter_mut() {
                *sample = hold;
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    );

    match stream {
        Ok(s) => {
            if let Err(e) = s.play() {
                eprintln!("Failed to start audio: {}", e);
                return None;
            }
            Some(s)
        }
        Err(e) => {
            eprintln!("Failed to build audio stream: {}", e);
            None
        }
    }
}

fn drain_audio_samples(gb: &mut GameBoy, audio_buffer: &Arc<Mutex<VecDeque<f32>>>) {
    if let Ok(mut buffer) = audio_buffer.lock() {
        buffer.extend(gb.cpu.bus.apu.sample_buffer.drain(..));
        // Cap at ~4 frames of audio to prevent latency buildup
        let sample_rate = gb.cpu.bus.apu.sample_rate as usize;
        let max_samples = (sample_rate * 2 * 4) / 60; // stereo, 4 frames
        if buffer.len() > max_samples {
            let excess = buffer.len() - max_samples;
            drop(buffer.drain(..excess));
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
