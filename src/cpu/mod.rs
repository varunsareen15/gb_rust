pub mod registers;
pub mod memory;
pub mod instruction;

use registers::*;
use memory::*;
use instruction::*;
use crate::cartridge::Cartridge;

pub struct CPU {
    pub registers: Registers,
    pub pc: u16,
    pub sp: u16,
    pub bus: MemoryBus,
    pub ime: bool,
    pub halted: bool,
    ei_pending: bool,
    halt_bug: bool,
}

impl CPU {
    pub fn new(cartridge: Cartridge) -> Self {
        let mut cpu = CPU {
            registers: Registers::default(),
            pc: 0x0100,
            sp: 0xFFFE,
            bus: MemoryBus::new(cartridge),
            ime: false,
            halted: false,
            ei_pending: false,
            halt_bug: false,
        };
        // Post-boot register state (DMG)
        cpu.registers.a = 0x01;
        cpu.registers.f = FlagsRegister::from(0xB0);
        cpu.registers.b = 0x00;
        cpu.registers.c = 0x13;
        cpu.registers.d = 0x00;
        cpu.registers.e = 0xD8;
        cpu.registers.h = 0x01;
        cpu.registers.l = 0x4D;
        cpu
    }

    pub fn save_state(&self, buf: &mut Vec<u8>) {
        use crate::savestate::*;
        self.registers.save_state(buf);
        write_u16_le(buf, self.pc);
        write_u16_le(buf, self.sp);
        let flags: u8 = (if self.ime { 1 } else { 0 })
            | (if self.halted { 1 } else { 0 }) << 1
            | (if self.ei_pending { 1 } else { 0 }) << 2
            | (if self.halt_bug { 1 } else { 0 }) << 3;
        write_u8(buf, flags);
        self.bus.save_state(buf);
    }

    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) {
        use crate::savestate::*;
        self.registers.load_state(data, cursor);
        self.pc = read_u16_le(data, cursor);
        self.sp = read_u16_le(data, cursor);
        let flags = read_u8(data, cursor);
        self.ime = flags & 0x01 != 0;
        self.halted = flags & 0x02 != 0;
        self.ei_pending = flags & 0x04 != 0;
        self.halt_bug = flags & 0x08 != 0;
        self.bus.load_state(data, cursor);
    }

    pub fn step(&mut self) -> u8 {
        let interrupt_cycles = self.handle_interrupts();
        if interrupt_cycles > 0 {
            return interrupt_cycles;
        }

        if self.halted {
            return 4; // HALT consumes 4 T-cycles per tick
        }

        let halt_bug_active = self.halt_bug;
        if halt_bug_active {
            self.halt_bug = false;
        }

        // Handle delayed EI: IME becomes true after the instruction following EI
        if self.ei_pending {
            self.ei_pending = false;
            self.ime = true;
        }

        let mut instruction_byte = self.bus.read_byte(self.pc);
        let prefixed = instruction_byte == 0xCB;
        if prefixed {
            instruction_byte = self.bus.read_byte(self.pc + 1);
        }

        // HALT bug: PC failed to increment during HALT, so the byte after HALT
        // is fetched as the opcode but PC still points one behind. This causes
        // multi-byte instructions to re-read the opcode byte as their first operand.
        if halt_bug_active {
            self.pc = self.pc.wrapping_sub(1);
        }

        let (next_pc, cycles) = if let Some(instruction) = Instruction::from_byte(instruction_byte, prefixed) {
            self.execute(instruction)
        } else {
            let description = format!("0x{}{:02x}", if prefixed { "cb" } else { "" }, instruction_byte);
            panic!("Unknown instruction found for: {} at PC={:#06x}", description, self.pc)
        };

        self.pc = next_pc;
        cycles
    }

    fn handle_interrupts(&mut self) -> u8 {
        let pending = self.bus.if_register & self.bus.ie_register & 0x1F;
        if pending != 0 {
            self.halted = false;
        }
        if !self.ime || pending == 0 {
            return 0;
        }

        // Service the highest-priority (lowest bit) interrupt
        for bit in 0..5u8 {
            if pending & (1 << bit) != 0 {
                self.ime = false;
                self.bus.if_register &= !(1 << bit);
                self.push(self.pc);
                self.pc = match bit {
                    0 => 0x0040, // VBlank
                    1 => 0x0048, // LCD STAT
                    2 => 0x0050, // Timer
                    3 => 0x0058, // Serial
                    4 => 0x0060, // Joypad
                    _ => unreachable!(),
                };
                return 20;
            }
        }
        0
    }

    fn resolve_byte_target(&mut self, target: &ByteTarget) -> (u8, u16) {
        match target {
            ByteTarget::A => (self.registers.a, 1),
            ByteTarget::B => (self.registers.b, 1),
            ByteTarget::C => (self.registers.c, 1),
            ByteTarget::D => (self.registers.d, 1),
            ByteTarget::E => (self.registers.e, 1),
            ByteTarget::H => (self.registers.h, 1),
            ByteTarget::L => (self.registers.l, 1),
            ByteTarget::HL => (self.bus.read_byte(self.registers.get_hl()), 1),
            ByteTarget::Imm8 => (self.read_next_byte(), 2),
        }
    }

    fn read_prefix_target(&mut self, target: &PrefixTarget) -> u8 {
        match target {
            PrefixTarget::A => self.registers.a,
            PrefixTarget::B => self.registers.b,
            PrefixTarget::C => self.registers.c,
            PrefixTarget::D => self.registers.d,
            PrefixTarget::E => self.registers.e,
            PrefixTarget::H => self.registers.h,
            PrefixTarget::L => self.registers.l,
            PrefixTarget::HL => self.bus.read_byte(self.registers.get_hl()),
        }
    }

    fn write_prefix_target(&mut self, target: &PrefixTarget, value: u8) {
        match target {
            PrefixTarget::A => self.registers.a = value,
            PrefixTarget::B => self.registers.b = value,
            PrefixTarget::C => self.registers.c = value,
            PrefixTarget::D => self.registers.d = value,
            PrefixTarget::E => self.registers.e = value,
            PrefixTarget::H => self.registers.h = value,
            PrefixTarget::L => self.registers.l = value,
            PrefixTarget::HL => {
                let addr = self.registers.get_hl();
                self.bus.write_byte(addr, value);
            }
        }
    }

    fn execute(&mut self, instruction: Instruction) -> (u16, u8) {
        match instruction {
            Instruction::NOP => (self.pc.wrapping_add(1), 4),

            Instruction::ADD(target) => {
                let (value, pc_inc) = self.resolve_byte_target(&target);
                let cycles = if matches!(target, ByteTarget::HL | ByteTarget::Imm8) { 8 } else { 4 };
                self.registers.a = self.add(value);
                (self.pc.wrapping_add(pc_inc), cycles)
            }
            Instruction::ADC(target) => {
                let (value, pc_inc) = self.resolve_byte_target(&target);
                let cycles = if matches!(target, ByteTarget::HL | ByteTarget::Imm8) { 8 } else { 4 };
                self.registers.a = self.adc(value);
                (self.pc.wrapping_add(pc_inc), cycles)
            }
            Instruction::SUB(target) => {
                let (value, pc_inc) = self.resolve_byte_target(&target);
                let cycles = if matches!(target, ByteTarget::HL | ByteTarget::Imm8) { 8 } else { 4 };
                self.registers.a = self.sub(value);
                (self.pc.wrapping_add(pc_inc), cycles)
            }
            Instruction::SBC(target) => {
                let (value, pc_inc) = self.resolve_byte_target(&target);
                let cycles = if matches!(target, ByteTarget::HL | ByteTarget::Imm8) { 8 } else { 4 };
                self.registers.a = self.sbc(value);
                (self.pc.wrapping_add(pc_inc), cycles)
            }
            Instruction::AND(target) => {
                let (value, pc_inc) = self.resolve_byte_target(&target);
                let cycles = if matches!(target, ByteTarget::HL | ByteTarget::Imm8) { 8 } else { 4 };
                self.registers.a = self.and(value);
                (self.pc.wrapping_add(pc_inc), cycles)
            }
            Instruction::OR(target) => {
                let (value, pc_inc) = self.resolve_byte_target(&target);
                let cycles = if matches!(target, ByteTarget::HL | ByteTarget::Imm8) { 8 } else { 4 };
                self.registers.a = self.or(value);
                (self.pc.wrapping_add(pc_inc), cycles)
            }
            Instruction::XOR(target) => {
                let (value, pc_inc) = self.resolve_byte_target(&target);
                let cycles = if matches!(target, ByteTarget::HL | ByteTarget::Imm8) { 8 } else { 4 };
                self.registers.a = self.xor(value);
                (self.pc.wrapping_add(pc_inc), cycles)
            }
            Instruction::CP(target) => {
                let (value, pc_inc) = self.resolve_byte_target(&target);
                let cycles = if matches!(target, ByteTarget::HL | ByteTarget::Imm8) { 8 } else { 4 };
                self.cp(value);
                (self.pc.wrapping_add(pc_inc), cycles)
            }

            Instruction::ADDHL(target) => {
                let value = match target {
                    ArithmeticHLTarget::BC => self.registers.get_bc(),
                    ArithmeticHLTarget::DE => self.registers.get_de(),
                    ArithmeticHLTarget::HL => self.registers.get_hl(),
                    ArithmeticHLTarget::SP => self.sp,
                };
                let new_value = self.add_hl(value);
                self.registers.set_hl(new_value);
                (self.pc.wrapping_add(1), 8)
            }

            Instruction::INC(target) => {
                let cycles = match target {
                    IncDecTarget::BC | IncDecTarget::DE | IncDecTarget::HL | IncDecTarget::SP => 8,
                    IncDecTarget::HLREF => 12,
                    _ => 4,
                };
                match target {
                    IncDecTarget::A => { self.registers.a = self.inc(self.registers.a); }
                    IncDecTarget::B => { self.registers.b = self.inc(self.registers.b); }
                    IncDecTarget::C => { self.registers.c = self.inc(self.registers.c); }
                    IncDecTarget::D => { self.registers.d = self.inc(self.registers.d); }
                    IncDecTarget::E => { self.registers.e = self.inc(self.registers.e); }
                    IncDecTarget::H => { self.registers.h = self.inc(self.registers.h); }
                    IncDecTarget::L => { self.registers.l = self.inc(self.registers.l); }
                    IncDecTarget::BC => { let v = self.registers.get_bc().wrapping_add(1); self.registers.set_bc(v); }
                    IncDecTarget::DE => { let v = self.registers.get_de().wrapping_add(1); self.registers.set_de(v); }
                    IncDecTarget::HL => { let v = self.registers.get_hl().wrapping_add(1); self.registers.set_hl(v); }
                    IncDecTarget::SP => { self.sp = self.sp.wrapping_add(1); }
                    IncDecTarget::HLREF => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        let new_value = self.inc(value);
                        self.bus.write_byte(addr, new_value);
                    }
                }
                (self.pc.wrapping_add(1), cycles)
            }
            Instruction::DEC(target) => {
                let cycles = match target {
                    IncDecTarget::BC | IncDecTarget::DE | IncDecTarget::HL | IncDecTarget::SP => 8,
                    IncDecTarget::HLREF => 12,
                    _ => 4,
                };
                match target {
                    IncDecTarget::A => { self.registers.a = self.dec(self.registers.a); }
                    IncDecTarget::B => { self.registers.b = self.dec(self.registers.b); }
                    IncDecTarget::C => { self.registers.c = self.dec(self.registers.c); }
                    IncDecTarget::D => { self.registers.d = self.dec(self.registers.d); }
                    IncDecTarget::E => { self.registers.e = self.dec(self.registers.e); }
                    IncDecTarget::H => { self.registers.h = self.dec(self.registers.h); }
                    IncDecTarget::L => { self.registers.l = self.dec(self.registers.l); }
                    IncDecTarget::BC => { let v = self.registers.get_bc().wrapping_sub(1); self.registers.set_bc(v); }
                    IncDecTarget::DE => { let v = self.registers.get_de().wrapping_sub(1); self.registers.set_de(v); }
                    IncDecTarget::HL => { let v = self.registers.get_hl().wrapping_sub(1); self.registers.set_hl(v); }
                    IncDecTarget::SP => { self.sp = self.sp.wrapping_sub(1); }
                    IncDecTarget::HLREF => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        let new_value = self.dec(value);
                        self.bus.write_byte(addr, new_value);
                    }
                }
                (self.pc.wrapping_add(1), cycles)
            }

            Instruction::JP(test) => {
                match test {
                    JumpTest::HL => {
                        (self.registers.get_hl(), 4)
                    }
                    _ => {
                        let jump_condition = match test {
                            JumpTest::NotZero => !self.registers.f.zero,
                            JumpTest::NotCarry => !self.registers.f.carry,
                            JumpTest::Zero => self.registers.f.zero,
                            JumpTest::Carry => self.registers.f.carry,
                            JumpTest::Always => true,
                            JumpTest::HL => unreachable!(),
                        };
                        let next_pc = self.jump(jump_condition);
                        let cycles = if jump_condition { 16 } else { 12 };
                        (next_pc, cycles)
                    }
                }
            }
            Instruction::JR(test) => {
                let jump_condition = match test {
                    JumpTest::NotZero => !self.registers.f.zero,
                    JumpTest::Zero => self.registers.f.zero,
                    JumpTest::NotCarry => !self.registers.f.carry,
                    JumpTest::Carry => self.registers.f.carry,
                    JumpTest::Always => true,
                    _ => panic!("Invalid jump condition for JR instruction"),
                };
                let next_pc = self.jr(jump_condition);
                let cycles = if jump_condition { 12 } else { 8 };
                (next_pc, cycles)
            }

            Instruction::LD(load_type) => {
                match load_type {
                    LoadType::Byte(target, source) => {
                        let source_value = match source {
                            LoadByteSource::A => self.registers.a,
                            LoadByteSource::B => self.registers.b,
                            LoadByteSource::C => self.registers.c,
                            LoadByteSource::D => self.registers.d,
                            LoadByteSource::E => self.registers.e,
                            LoadByteSource::H => self.registers.h,
                            LoadByteSource::L => self.registers.l,
                            LoadByteSource::D8 => self.read_next_byte(),
                            LoadByteSource::HLI => {
                                let value = self.bus.read_byte(self.registers.get_hl());
                                self.registers.set_hl(self.registers.get_hl().wrapping_add(1));
                                value
                            }
                            LoadByteSource::HLD => {
                                let value = self.bus.read_byte(self.registers.get_hl());
                                self.registers.set_hl(self.registers.get_hl().wrapping_sub(1));
                                value
                            }
                            LoadByteSource::BC => self.bus.read_byte(self.registers.get_bc()),
                            LoadByteSource::DE => self.bus.read_byte(self.registers.get_de()),
                            LoadByteSource::A8 => {
                                let offset = self.read_next_byte() as u16;
                                self.bus.read_byte(0xFF00 + offset)
                            }
                            LoadByteSource::A16 => {
                                let addr = self.read_next_word();
                                self.bus.read_byte(addr)
                            }
                            LoadByteSource::HL => self.bus.read_byte(self.registers.get_hl()),
                            LoadByteSource::HiC => self.bus.read_byte(0xFF00 | self.registers.c as u16),
                        };
                        match target {
                            LoadByteTarget::A => self.registers.a = source_value,
                            LoadByteTarget::B => self.registers.b = source_value,
                            LoadByteTarget::C => self.registers.c = source_value,
                            LoadByteTarget::D => self.registers.d = source_value,
                            LoadByteTarget::E => self.registers.e = source_value,
                            LoadByteTarget::H => self.registers.h = source_value,
                            LoadByteTarget::L => self.registers.l = source_value,
                            LoadByteTarget::HL => {
                                let addr = self.registers.get_hl();
                                self.bus.write_byte(addr, source_value);
                            }
                            LoadByteTarget::HLI => {
                                let addr = self.registers.get_hl();
                                self.bus.write_byte(addr, source_value);
                                self.registers.set_hl(addr.wrapping_add(1));
                            }
                            LoadByteTarget::HLD => {
                                let addr = self.registers.get_hl();
                                self.bus.write_byte(addr, source_value);
                                self.registers.set_hl(addr.wrapping_sub(1));
                            }
                            LoadByteTarget::BC => {
                                let addr = self.registers.get_bc();
                                self.bus.write_byte(addr, source_value);
                            }
                            LoadByteTarget::DE => {
                                let addr = self.registers.get_de();
                                self.bus.write_byte(addr, source_value);
                            }
                            LoadByteTarget::A8 => {
                                let offset = self.read_next_byte() as u16;
                                self.bus.write_byte(0xFF00 + offset, source_value);
                            }
                            LoadByteTarget::A16 => {
                                let addr = self.read_next_word();
                                self.bus.write_byte(addr, source_value);
                            }
                            LoadByteTarget::HiC => {
                                self.bus.write_byte(0xFF00 | self.registers.c as u16, source_value);
                            }
                        };
                        let source_len: u16 = match source {
                            LoadByteSource::D8 | LoadByteSource::A8 => 1,
                            LoadByteSource::A16 => 2,
                            _ => 0,
                        };
                        let target_len: u16 = match target {
                            LoadByteTarget::A8 => 1,
                            LoadByteTarget::A16 => 2,
                            _ => 0,
                        };
                        let operand_bytes = std::cmp::max(source_len, target_len);
                        let cycles = self.ld_byte_cycles(&target, &source);
                        (self.pc.wrapping_add(1 + operand_bytes), cycles)
                    }
                    LoadType::Word(target, source) => {
                        let source_value = match source {
                            LoadWordSource::D16 => self.read_next_word(),
                            LoadWordSource::SP => self.sp,
                            LoadWordSource::HL => self.registers.get_hl(),
                        };
                        match target {
                            LoadWordTarget::BC => self.registers.set_bc(source_value),
                            LoadWordTarget::DE => self.registers.set_de(source_value),
                            LoadWordTarget::HL => self.registers.set_hl(source_value),
                            LoadWordTarget::SP => self.sp = source_value,
                            LoadWordTarget::A16 => {
                                let addr = self.read_next_word();
                                self.bus.write_byte(addr, (source_value & 0xFF) as u8);
                                self.bus.write_byte(addr.wrapping_add(1), (source_value >> 8) as u8);
                            }
                        };
                        let (pc_inc, cycles) = match (&target, &source) {
                            (LoadWordTarget::A16, LoadWordSource::SP) => (3, 20),
                            (LoadWordTarget::SP, LoadWordSource::HL) => (1, 8),
                            (_, LoadWordSource::D16) => (3, 12),
                            _ => (1, 8),
                        };
                        (self.pc.wrapping_add(pc_inc), cycles)
                    }
                }
            }

            Instruction::PUSH(target) => {
                let value = match target {
                    StackTarget::BC => self.registers.get_bc(),
                    StackTarget::DE => self.registers.get_de(),
                    StackTarget::HL => self.registers.get_hl(),
                    StackTarget::AF => self.registers.get_af(),
                };
                self.push(value);
                (self.pc.wrapping_add(1), 16)
            }
            Instruction::POP(target) => {
                let result = self.pop();
                match target {
                    StackTarget::BC => self.registers.set_bc(result),
                    StackTarget::DE => self.registers.set_de(result),
                    StackTarget::HL => self.registers.set_hl(result),
                    StackTarget::AF => self.registers.set_af(result),
                };
                (self.pc.wrapping_add(1), 12)
            }

            Instruction::CALL(test) => {
                let jump_condition = match test {
                    JumpTest::NotZero => !self.registers.f.zero,
                    JumpTest::Zero => self.registers.f.zero,
                    JumpTest::NotCarry => !self.registers.f.carry,
                    JumpTest::Carry => self.registers.f.carry,
                    JumpTest::Always => true,
                    _ => panic!("Invalid jump condition for CALL instruction"),
                };
                let next_pc = self.call(jump_condition);
                let cycles = if jump_condition { 24 } else { 12 };
                (next_pc, cycles)
            }
            Instruction::RET(test) => {
                let jump_condition = match test {
                    JumpTest::NotZero => !self.registers.f.zero,
                    JumpTest::Zero => self.registers.f.zero,
                    JumpTest::NotCarry => !self.registers.f.carry,
                    JumpTest::Carry => self.registers.f.carry,
                    JumpTest::Always => true,
                    _ => panic!("Invalid jump condition for RET instruction"),
                };
                let next_pc = self.return_(jump_condition);
                let cycles = match test {
                    JumpTest::Always => 16,
                    _ => if jump_condition { 20 } else { 8 },
                };
                (next_pc, cycles)
            }
            Instruction::RETI => {
                self.ime = true;
                (self.return_(true), 16)
            }

            Instruction::ADDSP => {
                let offset = self.read_next_byte() as i8;
                let new_sp = self.sp.wrapping_add(offset as u16);
                self.registers.f.zero = false;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.sp & 0xF) + (offset as u16 & 0xF) > 0xF;
                self.registers.f.carry = (self.sp & 0xFF) + (offset as u16 & 0xFF) > 0xFF;
                self.sp = new_sp;
                (self.pc.wrapping_add(2), 16)
            }
            Instruction::LDHL => {
                let offset = self.read_next_byte() as i8;
                let new_hl = self.sp.wrapping_add(offset as u16);
                self.registers.f.zero = false;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.sp & 0xF) + (offset as u16 & 0xF) > 0xF;
                self.registers.f.carry = (self.sp & 0xFF) + (offset as u16 & 0xFF) > 0xFF;
                self.registers.set_hl(new_hl);
                (self.pc.wrapping_add(2), 12)
            }

            Instruction::DI => {
                self.ime = false;
                self.ei_pending = false;
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::EI => {
                self.ei_pending = true;
                (self.pc.wrapping_add(1), 4)
            }

            Instruction::RLCA => {
                self.rlca();
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::RRCA => {
                self.rrca();
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::RLA => {
                self.rla();
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::RRA => {
                self.rra();
                (self.pc.wrapping_add(1), 4)
            }

            Instruction::DAA => {
                self.daa();
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::CPL => {
                self.cpl();
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::SCF => {
                self.scf();
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::CCF => {
                self.ccf();
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::HALT => {
                if !self.ime && (self.bus.if_register & self.bus.ie_register & 0x1F) != 0 {
                    self.halt_bug = true;
                } else {
                    self.halted = true;
                }
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::STOP => {
                (self.pc.wrapping_add(2), 4)
            }
            Instruction::RST(addr) => {
                self.push(self.pc.wrapping_add(1));
                (addr as u16, 16)
            }

            // CB-prefixed
            Instruction::RLC(ref target) => {
                let value = self.read_prefix_target(target);
                let result = self.rlc(value);
                self.write_prefix_target(target, result);
                let cycles = if matches!(target, PrefixTarget::HL) { 16 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::RRC(ref target) => {
                let value = self.read_prefix_target(target);
                let result = self.rrc(value);
                self.write_prefix_target(target, result);
                let cycles = if matches!(target, PrefixTarget::HL) { 16 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::RL(ref target) => {
                let value = self.read_prefix_target(target);
                let result = self.rl(value);
                self.write_prefix_target(target, result);
                let cycles = if matches!(target, PrefixTarget::HL) { 16 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::RR(ref target) => {
                let value = self.read_prefix_target(target);
                let result = self.rr(value);
                self.write_prefix_target(target, result);
                let cycles = if matches!(target, PrefixTarget::HL) { 16 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::SLA(ref target) => {
                let value = self.read_prefix_target(target);
                let result = self.sla(value);
                self.write_prefix_target(target, result);
                let cycles = if matches!(target, PrefixTarget::HL) { 16 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::SRA(ref target) => {
                let value = self.read_prefix_target(target);
                let result = self.sra(value);
                self.write_prefix_target(target, result);
                let cycles = if matches!(target, PrefixTarget::HL) { 16 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::SWAP(ref target) => {
                let value = self.read_prefix_target(target);
                let result = self.swap(value);
                self.write_prefix_target(target, result);
                let cycles = if matches!(target, PrefixTarget::HL) { 16 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::SRL(ref target) => {
                let value = self.read_prefix_target(target);
                let result = self.srl(value);
                self.write_prefix_target(target, result);
                let cycles = if matches!(target, PrefixTarget::HL) { 16 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::BIT(bit, ref target) => {
                let value = self.read_prefix_target(target);
                self.bit(bit, value);
                let cycles = if matches!(target, PrefixTarget::HL) { 12 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::RES(bit, ref target) => {
                let value = self.read_prefix_target(target);
                let result = value & !(1 << bit);
                self.write_prefix_target(target, result);
                let cycles = if matches!(target, PrefixTarget::HL) { 16 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::SET(bit, ref target) => {
                let value = self.read_prefix_target(target);
                let result = value | (1 << bit);
                self.write_prefix_target(target, result);
                let cycles = if matches!(target, PrefixTarget::HL) { 16 } else { 8 };
                (self.pc.wrapping_add(2), cycles)
            }
        }
    }

    fn ld_byte_cycles(&self, target: &LoadByteTarget, source: &LoadByteSource) -> u8 {
        match (target, source) {
            (LoadByteTarget::A8, _) | (_, LoadByteSource::A8) => 12,
            (LoadByteTarget::A16, _) | (_, LoadByteSource::A16) => 16,
            (LoadByteTarget::HiC, _) | (_, LoadByteSource::HiC) => 8,
            (LoadByteTarget::HL, LoadByteSource::D8) => 12,
            (_, LoadByteSource::D8) => 8,
            (LoadByteTarget::HL, _) => 8,
            (LoadByteTarget::HLI, _) | (LoadByteTarget::HLD, _) => 8,
            (_, LoadByteSource::HL) | (_, LoadByteSource::HLI) | (_, LoadByteSource::HLD) => 8,
            (LoadByteTarget::BC, _) | (LoadByteTarget::DE, _) => 8,
            (_, LoadByteSource::BC) | (_, LoadByteSource::DE) => 8,
            _ => 4,
        }
    }

    // --- Control flow helpers ---

    fn call(&mut self, should_jump: bool) -> u16 {
        let next_pc = self.pc.wrapping_add(3);
        if should_jump {
            self.push(next_pc);
            self.read_next_word()
        } else {
            next_pc
        }
    }

    fn return_(&mut self, should_jump: bool) -> u16 {
        if should_jump {
            self.pop()
        } else {
            self.pc.wrapping_add(1)
        }
    }

    fn pop(&mut self) -> u16 {
        let lsb = self.bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let msb = self.bus.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (msb << 8) | lsb
    }

    fn push(&mut self, value: u16) {
        self.sp = self.sp.wrapping_sub(1);
        self.bus.write_byte(self.sp, (value >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.bus.write_byte(self.sp, (value & 0xFF) as u8);
    }

    fn jump(&mut self, should_jump: bool) -> u16 {
        if should_jump {
            let least_significant_byte = self.bus.read_byte(self.pc + 1) as u16;
            let most_significant_byte = self.bus.read_byte(self.pc + 2) as u16;
            (most_significant_byte << 8) | least_significant_byte
        } else {
            self.pc.wrapping_add(3)
        }
    }

    fn jr(&mut self, should_jump: bool) -> u16 {
        if should_jump {
            let offset = self.read_next_byte() as i8;
            self.pc.wrapping_add(2).wrapping_add(offset as u16)
        } else {
            self.pc.wrapping_add(2)
        }
    }

    // --- 8-bit ALU operations ---

    fn add(&mut self, value: u8) -> u8 {
        let (new_value, did_overflow) = self.registers.a.overflowing_add(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = (self.registers.a & 0xF) + (value & 0xF) > 0xF;
        new_value
    }

    fn adc(&mut self, value: u8) -> u8 {
        let carry_in = if self.registers.f.carry { 1 } else { 0 };
        let (intermediate, did_overflow1) = self.registers.a.overflowing_add(value);
        let (result, did_overflow2) = intermediate.overflowing_add(carry_in);
        self.registers.f.zero = result == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = ((self.registers.a & 0xF) + (value & 0xF) + carry_in) > 0xF;
        self.registers.f.carry = did_overflow1 || did_overflow2;
        result
    }

    fn add_hl(&mut self, value: u16) -> u16 {
        let hl = self.registers.get_hl();
        let (new_hl, did_overflow) = hl.overflowing_add(value);
        self.registers.f.subtract = false;
        self.registers.f.half_carry = ((hl & 0xFFF) + (value & 0xFFF)) > 0xFFF;
        self.registers.f.carry = did_overflow;
        new_hl
    }

    fn sub(&mut self, value: u8) -> u8 {
        let (new_value, did_overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (self.registers.a & 0xF) < (value & 0xF);
        self.registers.f.carry = did_overflow;
        new_value
    }

    fn sbc(&mut self, value: u8) -> u8 {
        let carry_in = if self.registers.f.carry { 1 } else { 0 };
        let (intermediate, did_overflow1) = self.registers.a.overflowing_sub(value);
        let (result, did_overflow2) = intermediate.overflowing_sub(carry_in);
        self.registers.f.zero = result == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (self.registers.a & 0xF) < ((value & 0xF) + carry_in);
        self.registers.f.carry = did_overflow1 || did_overflow2;
        result
    }

    fn and(&mut self, value: u8) -> u8 {
        let result = self.registers.a & value;
        self.registers.f.zero = result == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = true;
        self.registers.f.carry = false;
        result
    }

    fn or(&mut self, value: u8) -> u8 {
        let result = self.registers.a | value;
        self.registers.f.zero = result == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = false;
        result
    }

    fn xor(&mut self, value: u8) -> u8 {
        let result = self.registers.a ^ value;
        self.registers.f.zero = result == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = false;
        result
    }

    fn cp(&mut self, value: u8) {
        let (result, did_overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.zero = result == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (self.registers.a & 0xF) < (value & 0xF);
        self.registers.f.carry = did_overflow;
    }

    fn inc(&mut self, value: u8) -> u8 {
        let new_value = value.wrapping_add(1);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = (value & 0xF) + 1 > 0xF;
        new_value
    }

    fn dec(&mut self, value: u8) -> u8 {
        let new_value = value.wrapping_sub(1);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = (value & 0xF) == 0;
        new_value
    }

    // --- CB-prefixed rotate/shift operations ---

    fn rlc(&mut self, value: u8) -> u8 {
        let carry = value >> 7;
        let new_value = (value << 1) | carry;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = carry == 1;
        new_value
    }

    fn rrc(&mut self, value: u8) -> u8 {
        let carry = value & 1;
        let new_value = (value >> 1) | (carry << 7);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = carry == 1;
        new_value
    }

    fn rl(&mut self, value: u8) -> u8 {
        let old_carry = if self.registers.f.carry { 1u8 } else { 0 };
        let new_carry = value >> 7;
        let new_value = (value << 1) | old_carry;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = new_carry == 1;
        new_value
    }

    fn rr(&mut self, value: u8) -> u8 {
        let old_carry = if self.registers.f.carry { 1u8 } else { 0 };
        let new_carry = value & 1;
        let new_value = (value >> 1) | (old_carry << 7);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = new_carry == 1;
        new_value
    }

    fn sla(&mut self, value: u8) -> u8 {
        let carry = value >> 7;
        let new_value = value << 1;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = carry == 1;
        new_value
    }

    fn sra(&mut self, value: u8) -> u8 {
        let carry = value & 1;
        let new_value = (value >> 1) | (value & 0x80);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = carry == 1;
        new_value
    }

    fn swap(&mut self, value: u8) -> u8 {
        let new_value = (value >> 4) | (value << 4);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = false;
        new_value
    }

    fn srl(&mut self, value: u8) -> u8 {
        let carry = value & 1;
        let new_value = value >> 1;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = carry == 1;
        new_value
    }

    fn bit(&mut self, bit: u8, value: u8) {
        self.registers.f.zero = (value >> bit) & 1 == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = true;
    }

    // --- Accumulator rotates (unprefixed, always clear zero flag) ---

    fn rlca(&mut self) {
        let value = self.registers.a;
        let carry = value >> 7;
        self.registers.a = (value << 1) | carry;
        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = carry == 1;
    }

    fn rrca(&mut self) {
        let value = self.registers.a;
        let carry = value & 1;
        self.registers.a = (value >> 1) | (carry << 7);
        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = carry == 1;
    }

    fn rla(&mut self) {
        let value = self.registers.a;
        let old_carry = if self.registers.f.carry { 1u8 } else { 0 };
        let new_carry = value >> 7;
        self.registers.a = (value << 1) | old_carry;
        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = new_carry == 1;
    }

    fn rra(&mut self) {
        let value = self.registers.a;
        let old_carry = if self.registers.f.carry { 1u8 } else { 0 };
        let new_carry = value & 1;
        self.registers.a = (value >> 1) | (old_carry << 7);
        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = new_carry == 1;
    }

    // --- Misc operations ---

    fn daa(&mut self) {
        let mut a = self.registers.a;
        let mut adjust = 0u8;
        if self.registers.f.half_carry || (!self.registers.f.subtract && (a & 0xF) > 9) {
            adjust |= 0x06;
        }
        if self.registers.f.carry || (!self.registers.f.subtract && a > 0x99) {
            adjust |= 0x60;
            self.registers.f.carry = true;
        }
        if self.registers.f.subtract {
            a = a.wrapping_sub(adjust);
        } else {
            a = a.wrapping_add(adjust);
        }
        self.registers.f.zero = a == 0;
        self.registers.f.half_carry = false;
        self.registers.a = a;
    }

    fn cpl(&mut self) {
        self.registers.a = !self.registers.a;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = true;
    }

    fn scf(&mut self) {
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = true;
    }

    fn ccf(&mut self) {
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = !self.registers.f.carry;
    }

    // --- Memory read helpers ---

    fn read_next_word(&mut self) -> u16 {
        let least_significant_byte = self.bus.read_byte(self.pc + 1) as u16;
        let most_significant_byte = self.bus.read_byte(self.pc + 2) as u16;
        (most_significant_byte << 8) | least_significant_byte
    }

    fn read_next_byte(&mut self) -> u8 {
        self.bus.read_byte(self.pc + 1)
    }
}

impl Default for CPU {
    fn default() -> Self {
        CPU {
            registers: Registers::default(),
            pc: 0,
            sp: 0,
            bus: MemoryBus::default(),
            ime: false,
            halted: false,
            ei_pending: false,
            halt_bug: false,
        }
    }
}

#[cfg(test)]
mod tests;
