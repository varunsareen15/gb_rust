# GB Emulator Roadmap

## Phase 1: Correctness
- [x] 1.1 — HALT bug (IME=0 with pending interrupts: PC fails to increment)
- [x] 1.2 — Serial stub (0xFF01/0xFF02 — many games poll this)
- [x] 1.3 — Pass Blargg's cpu_instrs test ROM
- [x] 1.4 — Pass Blargg's instr_timing test ROM
- [x] 1.5 — Pass Blargg's mem_timing test ROM

## Phase 2: MBC Support
- [x] 2.1 — MBC1 (most common — Zelda, Metroid, Mega Man, etc.)
- [x] 2.2 — MBC3 + RTC (Pokemon Gold/Silver)
- [x] 2.3 — MBC5 (Pokemon Red/Blue JP rev, many later titles)

## Phase 3: Sound
- [ ] 3.1 — APU framework + channel 1 (square wave with sweep)
- [ ] 3.2 — Channel 2 (square wave, no sweep)
- [ ] 3.3 — Channel 3 (wave)
- [ ] 3.4 — Channel 4 (noise)
- [ ] 3.5 — Mixer + audio output (cpal or rodio)

## Phase 4: Persistence
- [ ] 4.1 — Battery saves (persist external RAM to .sav file)
- [ ] 4.2 — Save states (serialize full emulator state)

## Phase 5: Quality of Life
- [ ] 5.1 — Speed controls (fast-forward, slow-motion, frame step)
- [ ] 5.2 — FPS counter in title bar
- [ ] 5.3 — Configurable controls (config file)
- [ ] 5.4 — Window scaling / fullscreen toggle
- [ ] 5.5 — Shader/filters (CRT scanlines, DMG green)
- [ ] 5.6 — Gamepad support (gilrs)

## Phase 6: Debug Tools
- [ ] 6.1 — Headless mode (run without window, for testing)
- [ ] 6.2 — Tile map / VRAM viewer
- [ ] 6.3 — OAM / sprite viewer
- [ ] 6.4 — Register inspector + breakpoints

## Phase 7: Accurate PPU
- [ ] 7.1 — Pixel FIFO renderer (replaces scanline renderer)
- [ ] 7.2 — Variable mode 3 timing

## Phase 8: Game Boy Color
- [ ] 8.1 — CGB palette registers + color rendering
- [ ] 8.2 — Double-speed mode
- [ ] 8.3 — VRAM bank 1
- [ ] 8.4 — WRAM banks 1-7

## Phase 9: Architecture
- [ ] 9.1 — Separate emulator core into library crate
- [ ] 9.2 — Frontend trait (minifb as one impl)
