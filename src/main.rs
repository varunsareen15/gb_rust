mod cpu;
mod cartridge;
mod timer;
mod ppu;
mod joypad;
mod gameboy;
mod savestate;
mod apu;
mod filters;
mod config;
mod debug;

use cartridge::Cartridge;
use gameboy::GameBoy;
use joypad::JoypadKey;

use minifb::{Key, Window, WindowOptions, Scale};
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(PartialEq, Clone, Copy)]
enum SpeedMode {
    Normal,
    FastForward,
    Paused,
}

use filters::PALETTES;

const SCALE_STEPS: [(Scale, &str); 3] = [
    (Scale::X1, "2x"),
    (Scale::X2, "4x"),
    (Scale::X4, "8x"),
];

fn create_window(scale: Scale) -> Window {
    Window::new(
        "GB Emulator",
        320,
        288,
        WindowOptions {
            scale,
            ..WindowOptions::default()
        },
    ).expect("Failed to create window")
}

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
        let config = config::Config::load();
        run_windowed(&mut gb, &config);
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

fn run_windowed(gb: &mut GameBoy, config: &config::Config) {
    // Set up audio output via cpal
    let audio_buffer: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
    let _stream = setup_audio(gb, &audio_buffer);

    let mut scale_idx: usize = config.scale_index();
    let mut window = create_window(SCALE_STEPS[scale_idx].0);

    let frame_duration = Duration::from_nanos(16_742_706); // ~59.7 Hz
    let ff_multiplier = config.speed.fast_forward_multiplier;
    let mut native_buf = vec![0u32; 160 * 144];
    let mut buffer = vec![0u32; 320 * 288];

    // Palette and scanline state (from config)
    let mut palette_idx: usize = config.palette_index();
    let mut scanlines = config.display.scanlines;

    // Build joypad key map from config
    let joypad_map = config.joypad_key_map();

    // FPS tracking
    let mut frame_count: u32 = 0;
    let mut fps_timer = Instant::now();
    #[allow(unused_assignments)]
    let mut fps_display: f64 = 0.0;

    // Speed mode
    let mut speed_mode = SpeedMode::Normal;
    let mut was_paused = false;
    let mut ff_locked = false; // Shift+Tab toggle for persistent fast-forward

    // Debug windows
    let mut debug = debug::DebugWindows::new();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let frame_start = Instant::now();

        // Handle input
        update_joypad(&window, gb, &joypad_map);

        // Debug window toggles (F1/F2/F3)
        debug.handle_toggles(&window);

        // Speed controls
        let shift_held = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let tab_held = window.is_key_down(Key::Tab);
        // Shift+Tab toggles persistent fast-forward
        if shift_held && window.is_key_pressed(Key::Tab, minifb::KeyRepeat::No) {
            ff_locked = !ff_locked;
        }
        if window.is_key_pressed(Key::Space, minifb::KeyRepeat::No) {
            speed_mode = if speed_mode == SpeedMode::Paused {
                SpeedMode::Normal
            } else {
                SpeedMode::Paused
            };
            ff_locked = false;
        }
        if speed_mode != SpeedMode::Paused {
            speed_mode = if ff_locked || tab_held { SpeedMode::FastForward } else { SpeedMode::Normal };
        }

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

        // Palette / scanline controls
        if window.is_key_pressed(Key::P, minifb::KeyRepeat::No) {
            palette_idx = (palette_idx + 1) % PALETTES.len();
            eprintln!("Palette: {}", PALETTES[palette_idx].0);
        }
        if window.is_key_pressed(Key::F10, minifb::KeyRepeat::No) {
            scanlines = !scanlines;
            eprintln!("Scanlines: {}", if scanlines { "ON" } else { "OFF" });
        }

        // Window scaling
        if window.is_key_pressed(Key::F11, minifb::KeyRepeat::No) {
            scale_idx = (scale_idx + 1) % SCALE_STEPS.len();
            window = create_window(SCALE_STEPS[scale_idx].0);
            eprintln!("Scale: {}", SCALE_STEPS[scale_idx].1);
            continue;
        }

        // Determine whether to run a frame
        let run_frame = match speed_mode {
            SpeedMode::Normal | SpeedMode::FastForward => true,
            SpeedMode::Paused => {
                // Frame step: N advances one frame while paused
                window.is_key_pressed(Key::N, minifb::KeyRepeat::No)
            }
        };

        if run_frame {
            // Check if we have breakpoints to watch
            let has_breakpoints = debug.breakpoints()
                .map_or(false, |bps| !bps.is_empty());

            if has_breakpoints {
                let bps = debug.breakpoints().unwrap().clone();
                let hit = gb.run_frame_with_breakpoints(&bps);
                if hit {
                    speed_mode = SpeedMode::Paused;
                    eprintln!("Breakpoint hit at ${:04X}", gb.cpu.pc);
                }
            } else {
                gb.run_frame();
            }

            if speed_mode == SpeedMode::FastForward {
                // Mute audio during fast-forward: discard samples
                gb.cpu.bus.apu.sample_buffer.clear();
                if let Ok(mut buf) = audio_buffer.lock() {
                    buf.clear();
                }
            } else {
                drain_audio_samples(gb, &audio_buffer);
            }
        } else if !was_paused {
            // Just entered pause — clear audio buffer to silence output
            if let Ok(mut buf) = audio_buffer.lock() {
                buf.clear();
            }
        }
        was_paused = speed_mode == SpeedMode::Paused;

        // Convert framebuffer to u32 colors with current palette
        let fb = gb.framebuffer();
        let palette = &PALETTES[palette_idx].1;
        for (i, &pixel) in fb.iter().enumerate() {
            native_buf[i] = palette[(pixel & 0x03) as usize];
        }

        // Upscale 2x and optionally apply scanlines
        filters::upscale_nearest(&native_buf, &mut buffer, 160, 144);
        if scanlines {
            filters::apply_scanlines(&mut buffer, 320, 288);
        }

        window.update_with_buffer(&buffer, 320, 288).unwrap();

        // Update debug windows
        let debug_action = debug.update(gb, palette);
        match debug_action {
            Some(debug::DebugAction::Step) => {
                gb.run_step();
                speed_mode = SpeedMode::Paused;
            }
            Some(debug::DebugAction::BreakpointHit) => {
                speed_mode = SpeedMode::Paused;
            }
            None => {}
        }

        // FPS counter
        frame_count += 1;
        let fps_elapsed = fps_timer.elapsed();
        if fps_elapsed >= Duration::from_secs(1) {
            fps_display = frame_count as f64 / fps_elapsed.as_secs_f64();
            frame_count = 0;
            fps_timer = Instant::now();
            let mode_str = match speed_mode {
                SpeedMode::Normal => "",
                SpeedMode::FastForward => " [FAST]",
                SpeedMode::Paused => " [PAUSED]",
            };
            window.set_title(&format!("GB Emulator — {:.1} FPS{}", fps_display, mode_str));
        }

        // Frame timing
        match speed_mode {
            SpeedMode::FastForward => {
                if ff_multiplier > 0 {
                    let ff_duration = frame_duration / ff_multiplier;
                    let elapsed = frame_start.elapsed();
                    if elapsed < ff_duration {
                        let remaining = ff_duration - elapsed;
                        if remaining > Duration::from_millis(1) {
                            std::thread::sleep(remaining - Duration::from_millis(1));
                        }
                        while frame_start.elapsed() < ff_duration {
                            std::hint::spin_loop();
                        }
                    }
                }
            }
            SpeedMode::Paused => {
                // Sleep briefly to avoid burning CPU while paused
                std::thread::sleep(Duration::from_millis(16));
            }
            SpeedMode::Normal => {
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

fn update_joypad(window: &Window, gb: &mut GameBoy, key_map: &[(Key, JoypadKey)]) {
    for &(key, joypad_key) in key_map {
        if window.is_key_down(key) {
            gb.cpu.bus.joypad.key_down(joypad_key);
        } else {
            gb.cpu.bus.joypad.key_up(joypad_key);
        }
    }
}
