use std::collections::HashSet;
use minifb::{Window, WindowOptions, Key, KeyRepeat};
use super::font;
use super::disasm;
use super::{BG_COLOR, TEXT_COLOR, HEADER_COLOR, HIGHLIGHT_COLOR, BP_COLOR, DebugAction};
use crate::gameboy::GameBoy;

const WIN_W: usize = 320;
const WIN_H: usize = 440;

pub struct RegisterViewer {
    pub window: Window,
    buf: Vec<u32>,
    pub breakpoints: HashSet<u16>,
    // Breakpoint input state
    input_mode: bool,
    input_buf: String,
}

impl RegisterViewer {
    pub fn new() -> Self {
        let window = Window::new(
            "Registers",
            WIN_W,
            WIN_H,
            WindowOptions::default(),
        ).expect("Failed to create register viewer window");
        RegisterViewer {
            window,
            buf: vec![BG_COLOR; WIN_W * WIN_H],
            breakpoints: HashSet::new(),
            input_mode: false,
            input_buf: String::new(),
        }
    }

    pub fn update(&mut self, gb: &GameBoy, _palette: &[u32; 4]) -> Option<DebugAction> {
        self.buf.fill(BG_COLOR);

        let mut y = 4;

        // CPU Registers
        font::draw_string(&mut self.buf, WIN_W, 4, y, "CPU REGISTERS", HEADER_COLOR);
        y += 12;

        let af = gb.cpu.registers.get_af();
        let bc = gb.cpu.registers.get_bc();
        let de = gb.cpu.registers.get_de();
        let hl = gb.cpu.registers.get_hl();

        let line = format!("AF={:04X}  BC={:04X}", af, bc);
        font::draw_string(&mut self.buf, WIN_W, 4, y, &line, TEXT_COLOR);
        y += 10;

        let line = format!("DE={:04X}  HL={:04X}", de, hl);
        font::draw_string(&mut self.buf, WIN_W, 4, y, &line, TEXT_COLOR);
        y += 10;

        let line = format!("SP={:04X}  PC={:04X}", gb.cpu.sp, gb.cpu.pc);
        font::draw_string(&mut self.buf, WIN_W, 4, y, &line, TEXT_COLOR);
        y += 12;

        // Flags
        let f = &gb.cpu.registers.f;
        let flags_str = format!(
            "Z={} N={} H={} C={}",
            f.zero as u8, f.subtract as u8, f.half_carry as u8, f.carry as u8
        );
        font::draw_string(&mut self.buf, WIN_W, 4, y, &flags_str, TEXT_COLOR);
        y += 10;

        let line = format!(
            "IME={}  HALT={}",
            gb.cpu.ime as u8, gb.cpu.halted as u8
        );
        font::draw_string(&mut self.buf, WIN_W, 4, y, &line, TEXT_COLOR);
        y += 14;

        // IO Registers
        font::draw_string(&mut self.buf, WIN_W, 4, y, "IO REGISTERS", HEADER_COLOR);
        y += 12;

        let lcdc = gb.cpu.bus.ppu.lcdc;
        let stat = gb.cpu.bus.ppu.read_stat();
        let ly = gb.cpu.bus.ppu.ly;
        let line = format!("LCDC={:02X} STAT={:02X} LY={:02X}", lcdc, stat, ly);
        font::draw_string(&mut self.buf, WIN_W, 4, y, &line, TEXT_COLOR);
        y += 10;

        let scx = gb.cpu.bus.ppu.scx;
        let scy = gb.cpu.bus.ppu.scy;
        let wx = gb.cpu.bus.ppu.wx;
        let wy = gb.cpu.bus.ppu.wy;
        let line = format!("SCX={:02X} SCY={:02X} WX={:02X} WY={:02X}", scx, scy, wx, wy);
        font::draw_string(&mut self.buf, WIN_W, 4, y, &line, TEXT_COLOR);
        y += 10;

        let bgp = gb.cpu.bus.ppu.bgp;
        let obp0 = gb.cpu.bus.ppu.obp0;
        let obp1 = gb.cpu.bus.ppu.obp1;
        let line = format!("BGP={:02X} OBP0={:02X} OBP1={:02X}", bgp, obp0, obp1);
        font::draw_string(&mut self.buf, WIN_W, 4, y, &line, TEXT_COLOR);
        y += 10;

        let if_reg = gb.cpu.bus.if_register;
        let ie_reg = gb.cpu.bus.ie_register;
        let line = format!("IF={:02X}  IE={:02X}", if_reg, ie_reg);
        font::draw_string(&mut self.buf, WIN_W, 4, y, &line, TEXT_COLOR);
        y += 10;

        let div = gb.cpu.bus.timer.read(0xFF04);
        let tima = gb.cpu.bus.timer.read(0xFF05);
        let tma = gb.cpu.bus.timer.read(0xFF06);
        let tac = gb.cpu.bus.timer.read(0xFF07);
        let line = format!("DIV={:02X} TIMA={:02X} TMA={:02X} TAC={:02X}", div, tima, tma, tac);
        font::draw_string(&mut self.buf, WIN_W, 4, y, &line, TEXT_COLOR);
        y += 14;

        // Disassembly at PC
        font::draw_string(&mut self.buf, WIN_W, 4, y, "NEXT INSTRUCTION", HEADER_COLOR);
        y += 12;

        let (mnemonic, _size) = disasm::disassemble(gb.cpu.pc, |addr| {
            gb.cpu.bus.read_byte_no_tick(addr)
        });
        let line = format!("{:04X}: {}", gb.cpu.pc, mnemonic);
        font::draw_string(&mut self.buf, WIN_W, 4, y, &line, HIGHLIGHT_COLOR);
        y += 14;

        // Breakpoints
        font::draw_string(&mut self.buf, WIN_W, 4, y, "BREAKPOINTS", HEADER_COLOR);
        y += 12;

        if self.breakpoints.is_empty() {
            font::draw_string(&mut self.buf, WIN_W, 4, y, "(none)", TEXT_COLOR);
            y += 10;
        } else {
            let mut sorted: Vec<u16> = self.breakpoints.iter().copied().collect();
            sorted.sort();
            for bp in &sorted {
                let line = format!("  ${:04X}", bp);
                font::draw_string(&mut self.buf, WIN_W, 4, y, &line, BP_COLOR);
                y += 10;
            }
        }
        y += 4;

        // Input mode display
        if self.input_mode {
            let line = format!("BP addr> {}_", self.input_buf);
            font::draw_string(&mut self.buf, WIN_W, 4, y, &line, HIGHLIGHT_COLOR);
        }

        // Help
        let y = WIN_H - 20;
        font::draw_string(&mut self.buf, WIN_W, 4, y, "B:add bp  D:del bp  I:step", TEXT_COLOR);

        self.window.update_with_buffer(&self.buf, WIN_W, WIN_H).ok();

        // Handle keyboard input
        self.handle_input()
    }

    fn handle_input(&mut self) -> Option<DebugAction> {
        if self.input_mode {
            // Hex digit input
            for &(key, ch) in &[
                (Key::Key0, '0'), (Key::Key1, '1'), (Key::Key2, '2'), (Key::Key3, '3'),
                (Key::Key4, '4'), (Key::Key5, '5'), (Key::Key6, '6'), (Key::Key7, '7'),
                (Key::Key8, '8'), (Key::Key9, '9'),
                (Key::A, 'A'), (Key::B, 'B'), (Key::C, 'C'),
                (Key::D, 'D'), (Key::E, 'E'), (Key::F, 'F'),
            ] {
                if self.window.is_key_pressed(key, KeyRepeat::No) && self.input_buf.len() < 4 {
                    self.input_buf.push(ch);
                }
            }

            if self.window.is_key_pressed(Key::Backspace, KeyRepeat::No) {
                self.input_buf.pop();
            }

            if self.window.is_key_pressed(Key::Enter, KeyRepeat::No) {
                if let Ok(addr) = u16::from_str_radix(&self.input_buf, 16) {
                    self.breakpoints.insert(addr);
                }
                self.input_buf.clear();
                self.input_mode = false;
            }

            if self.window.is_key_pressed(Key::Escape, KeyRepeat::No) {
                self.input_buf.clear();
                self.input_mode = false;
            }

            return None;
        }

        // Normal mode
        if self.window.is_key_pressed(Key::B, KeyRepeat::No) {
            self.input_mode = true;
            self.input_buf.clear();
            return None;
        }

        if self.window.is_key_pressed(Key::D, KeyRepeat::No) {
            // Delete most recently added breakpoint (last in sorted order)
            if let Some(&bp) = self.breakpoints.iter().next() {
                self.breakpoints.remove(&bp);
            }
            return None;
        }

        if self.window.is_key_pressed(Key::I, KeyRepeat::No) {
            return Some(DebugAction::Step);
        }

        None
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }
}
