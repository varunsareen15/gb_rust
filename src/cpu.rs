use std::{collections::btree_map::Values, ptr::addr_of, result};

const ZERO_FLAG_BYTE_POSITION: u8 = 7;
const SUBTRACT_FLAG_BYTE_POSITION: u8 = 6;
const HALF_CARRY_FLAG_BYTE_POSITION: u8 = 5;
const CARRY_FLAG_BYTE_POSITION: u8 = 4;

// Game Boy registers, each register holds 8 bits
struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: FlagsRegister,
    h: u8,
    l: u8,
}

struct FlagsRegister {
    zero: bool,
    subtract: bool,
    half_carry: bool,
    carry: bool,
}

struct CPU {
    registers: Registers,
    pc: u16,
    sp: u16,
    bus: MemoryBus,
}

struct MemoryBus {
    memory: [u8; 0xFFFF]
}

impl MemoryBus {
    fn read_byte(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    fn write_byte(&self, address: u16, byte: u8) {}
}

impl Registers {
    fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    fn set_bc(&mut self, value: u16) {
        self.b = ((value & 0xFF00) >> 8) as u8;
        self.c = (value & 0xFF) as u8;
    }

    fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    fn set_de(&mut self, value: u16) {
        self.d = ((value & 0xFF00) >> 8) as u8;
        self.e = (value & 0xFF) as u8;
    }

    fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    fn set_hl(&mut self, value: u16) {
        self.h = ((value & 0xFF00) >> 8) as u8;
        self.l = (value & 0xFF) as u8;
    }
}

// Helps to easily convert FlagsRegister from u8 and back
impl std::convert::From<FlagsRegister> for u8  {
    fn from(flag: FlagsRegister) -> u8 {
        (if flag.zero       { 1 } else { 0 }) << ZERO_FLAG_BYTE_POSITION |
        (if flag.subtract   { 1 } else { 0 }) << SUBTRACT_FLAG_BYTE_POSITION |
        (if flag.half_carry { 1 } else { 0 }) << HALF_CARRY_FLAG_BYTE_POSITION |
        (if flag.carry      { 1 } else { 0 }) << CARRY_FLAG_BYTE_POSITION
    }
}

impl std::convert::From<u8> for FlagsRegister {
    fn from(byte: u8) -> Self {
        let zero = ((byte >> ZERO_FLAG_BYTE_POSITION) & 0b1) != 0;
        let subtract = ((byte >> SUBTRACT_FLAG_BYTE_POSITION) & 0b1) != 0;
        let half_carry = ((byte >> HALF_CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;
        let carry = ((byte >> CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;

        FlagsRegister {
            zero,
            subtract,
            half_carry,
            carry
        }
    }
}

enum Instruction {
    ADD(ArithmeticTarget),
    JP(JumpTest),
    LD(LoadType),
    PUSH(StackTarget),
    POP(StackTarget),
    CALL(JumpTest),
    RET(JumpTest),
    INC(IncDecTarget),
    RLC(PrefixTarget),
    ADDHL(ArithmeticHLTarget),
    ADC(ArithmeticTarget),
    SUB(SubtractionTarget),
    SBC(SubtractionTarget),
    AND(LogicalTarget),
    OR(LogicalTarget),
    XOR(LogicalTarget),
    CP(SubtractionTarget),
}

enum ArithmeticTarget {
    A, B, C, D, E, H, L,
}

enum ArithmeticHLTarget {
    BC, DE, HL, SP,
}

enum SubtractionTarget {
    A, B, C, D, E, H, L, HL, Imm8,
}

enum LogicalTarget {
    A, B, C, D, E, H, L, HL, Imm8,
}

enum JumpTest {
    NotZero,
    Zero,
    NotCarry,
    Carry,
    Always
}

enum LoadByteTarget {
    A, B, C, D, E, H, L, HLI 
}

enum LoadByteSource {
    A, B, C, D, E, H, L, D8, HLI 
}

enum LoadType {
    Byte(LoadByteTarget, LoadByteSource),
}

enum StackTarget {
    BC,
    DE,
}

enum IncDecTarget {
    BC,
}

enum PrefixTarget {
    B,
}

impl CPU {
    fn step(&mut self) {
        let mut instruction_byte = self.bus.read_byte(self.pc);
        let prefixed = instruction_byte == 0xCB;
        if prefixed {
            instruction_byte = self.bus.read_byte(self.pc + 1);
        }
        let next_pc = if let Some(instruction) = Instruction::from_byte(instruction_byte, prefixed) {
            self.execute(instruction)
        } else {
            let description = format!("0x{}{:x}", if prefixed { "cb" } else { "" }, instruction_byte);
            panic!("Unkown instruction found for: {}", description)    
        };
        self.pc = next_pc;
    }

    fn execute(&mut self, instruction: Instruction) -> u16 {
        match instruction {
            Instruction::ADD(target) => { // Completed ADD
                match target {
                    ArithmeticTarget::C => {
                        let value = self.registers.c;
                        let new_value = self.add(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::B => {
                        let value = self.registers.b;
                        let new_value = self.add(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::A => {
                        let value = self.registers.a;
                        let new_value = self.add(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::D => {
                        let value = self.registers.d;
                        let new_value = self.add(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::E => {
                        let value = self.registers.e;
                        let new_value = self.add(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::H => {
                        let value = self.registers.h;
                        let new_value = self.add(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::L => {
                        let value = self.registers.l;
                        let new_value = self.add(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                }
            }
            Instruction::ADC(target) => { // Finished ADC
                match target {
                    ArithmeticTarget::A => {
                        let value = self.registers.a;
                        let new_value = self.adc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::B => {
                        let value = self.registers.b;
                        let new_value = self.adc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::C => {
                        let value = self.registers.c;
                        let new_value = self.adc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::D => {
                        let value = self.registers.d;
                        let new_value = self.adc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::E => {
                        let value = self.registers.e;
                        let new_value = self.adc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::H => {
                        let value = self.registers.h;
                        let new_value = self.adc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::L => {
                        let value = self.registers.l;
                        let new_value = self.adc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                }
            }
            Instruction::ADDHL(target) => {
                match target { // Finished ADDHL
                    ArithmeticHLTarget::BC => {
                        let value = self.registers.get_bc();
                        let new_value = self.add_hl(value);
                        self.registers.set_hl(new_value);
                        self.pc = self.pc.wrapping_add(1);
                        self.pc
                    }
                    ArithmeticHLTarget::DE => {
                        let value = self.registers.get_de();
                        let new_value = self.add_hl(value);
                        self.registers.set_hl(new_value);
                        self.pc = self.pc.wrapping_add(1);
                        self.pc
                    }
                    ArithmeticHLTarget::HL => {
                        let value = self.registers.get_hl();
                        let new_value = self.add_hl(value);
                        self.registers.set_hl(new_value);
                        self.pc = self.pc.wrapping_add(1);
                        self.pc
                    }
                    ArithmeticHLTarget::SP => {
                        let value = self.sp;
                        let new_value = self.add_hl(value);
                        self.registers.set_hl(new_value);
                        self.pc = self.pc.wrapping_add(1);
                        self.pc
                    }
                }     
            }
            Instruction::SUB(target) => { // Finished SUB
                match target {
                    SubtractionTarget::A => {
                        let value = self.registers.a;
                        let new_value = self.sub(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::B => {
                        let value = self.registers.b;
                        let new_value = self.sub(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::C => {
                        let value = self.registers.c;
                        let new_value = self.sub(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::D => {
                        let value = self.registers.d;
                        let new_value = self.sub(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::E => {
                        let value = self.registers.e;
                        let new_value = self.sub(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::H => {
                        let value = self.registers.h;
                        let new_value = self.sub(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::L => {
                        let value = self.registers.l;
                        let new_value = self.sub(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::HL => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        let new_value = self.sub(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::Imm8 => {
                        let value = self.read_next_byte();
                        let new_value = self.sub(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(2)
                    }
                }
            }
            Instruction::SBC(target) => { // Finished SBC
                match target {
                    SubtractionTarget::A => {
                        let value = self.registers.a;
                        let new_value = self.sbc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::B => {
                        let value = self.registers.b;
                        let new_value = self.sbc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::C => {
                        let value = self.registers.c;
                        let new_value = self.sbc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::D => {
                        let value = self.registers.d;
                        let new_value = self.sbc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::E => {
                        let value = self.registers.e;
                        let new_value = self.sbc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::H => {
                        let value = self.registers.h;
                        let new_value = self.sbc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::L => {
                        let value = self.registers.l;
                        let new_value = self.sbc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::HL => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        let new_value = self.sbc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::Imm8 => {
                        let value = self.read_next_byte();
                        let new_value = self.sbc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(2)
                    }
                }
            }
            Instruction::AND(target) => { // Finished AND
                match target {
                    LogicalTarget::A => {
                        let value = self.registers.a;
                        let new_value = self.and(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::B => {
                        let value = self.registers.b;
                        let new_value = self.and(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::C => {
                        let value = self.registers.c;
                        let new_value = self.and(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::D => {
                        let value = self.registers.d;
                        let new_value = self.and(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::E => {
                        let value = self.registers.l;
                        let new_value = self.and(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::H => {
                        let value = self.registers.h;
                        let new_value = self.and(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::L => {
                        let value = self.registers.l;
                        let new_value = self.and(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::HL => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        let new_value = self.and(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::Imm8 => {
                        let value = self.read_next_byte();
                        let new_value = self.and(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(2)
                    }
                }
            }
            Instruction::OR(target) => { // Finished OR
                match target {
                    LogicalTarget::A => {
                        let value = self.registers.a;
                        let new_value = self.or(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::B => {
                        let value = self.registers.a;
                        let new_value = self.or(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::C => {
                        let value = self.registers.c;
                        let new_value = self.or(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::D => {
                        let value = self.registers.d;
                        let new_value = self.or(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::E => {
                        let value = self.registers.e;
                        let new_value = self.or(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::H => {
                        let value = self.registers.h;
                        let new_value = self.or(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::L => {
                        let value = self.registers.l;
                        let new_value = self.or(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::HL => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        let new_value = self.or(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::Imm8 => {
                        let value = self.read_next_byte();
                        let new_value = self.or(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(2)
                    }
                }
            }
            Instruction::XOR(target) => {
                match target {
                    LogicalTarget::A => {
                        let value = self.registers.a;
                        let new_value = self.xor(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::B => {
                        let value = self.registers.b;
                        let new_value = self.xor(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::C => {
                        let value = self.registers.c;
                        let new_value = self.xor(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::D => {
                        let value = self.registers.d;
                        let new_value = self.xor(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::E => {
                        let value = self.registers.e;
                        let new_value = self.xor(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::H => {
                        let value = self.registers.h;
                        let new_value = self.xor(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::L => {
                        let value = self.registers.l;
                        let new_value = self.xor(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::HL => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        let new_value = self.xor(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    LogicalTarget::Imm8 => {
                        let value = self.read_next_byte();
                        let new_value = self.xor(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(2)
                    }
                }
            }
            Instruction::CP(target) => {
                match target {
                    SubtractionTarget::A => {
                        let value = self.registers.a;
                        self.cp(value);
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::B => {
                        let value = self.registers.b;
                        self.cp(value);
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::C => {
                        let value = self.registers.c;
                        self.cp(value);
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::D => {
                        let value = self.registers.d;
                        self.cp(value);
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::E => {
                        let value = self.registers.e;
                        self.cp(value);
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::H => {
                        let value = self.registers.h;
                        self.cp(value);
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::L => {
                        let value = self.registers.l;
                        self.cp(value);
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::HL => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        self.cp(value);
                        self.pc.wrapping_add(1)
                    }
                    SubtractionTarget::Imm8 => {
                        let value = self.read_next_byte();
                        self.cp(value);
                        self.pc.wrapping_add(2)
                    }
                }
            }
            Instruction::JP(test) => {
                let jump_condition = match test {
                    JumpTest::NotZero => !self.registers.f.zero,
                    JumpTest::NotCarry => !self.registers.f.carry,
                    JumpTest::Zero => self.registers.f.zero,
                    JumpTest::Carry => self.registers.f.carry,
                    JumpTest::Always => true
                };
                self.jump(jump_condition)
            }
            Instruction::LD(load_type) => {
                match load_type {
                    LoadType::Byte(target, source) => {
                        let source_value = match source {
                            LoadByteSource::A => self.registers.a,
                            LoadByteSource::D8 => self.read_next_byte(),
                            LoadByteSource::HLI => self.bus.read_byte(self.registers.get_hl()),
                            _ => { panic!("TODO: implement other sources") }
                        };
                        match target {
                            LoadByteTarget::A => self.registers.a = source_value,
                            LoadByteTarget::HLI => self.bus.write_byte(self.registers.get_hl(), source_value),
                            _ => { panic!("TODO: implement other targets") }
                        };
                        match source {
                            LoadByteSource::D8 => self.pc.wrapping_add(2),
                            _                  => self.pc.wrapping_add(1),
                        }
                    }
                    _ => { panic!("TODO: implement other load types") }
                }
            }
            Instruction::PUSH(target) => {
                let value = match target {
                    StackTarget::BC => self.registers.get_bc(),
                    _ => { panic!("TODO: support more targets") }
                };
                self.push(value);
                self.pc.wrapping_add(1)
            }
            Instruction::POP(target) => {
                let result = self.pop();
                match target {
                    StackTarget::BC => self.registers.set_bc(result),
                    _ => { panic!("TODO: support more targets") }
                };
                self.pc.wrapping_add(1)
            }
            Instruction::CALL(test) => {
                let jump_condition = match test {
                    JumpTest::NotZero => !self.registers.f.zero,
                    _ => { panic!("TODO: support more conditions") }
                };
                self.call(jump_condition)
            }
            Instruction::RET(test) => {
                let jump_condition = match test {
                    JumpTest::NotZero => !self.registers.f.zero,
                    _ => { panic!("TODO: support more conditions") }
                };
                self.return_(jump_condition)
            }
            _ => { /* TODO: support more instructions */ self.pc }
        } 
    }

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
        self.bus.write_byte(self.sp, (value & 0xFF00 >> 8) as u8);

        self.sp = self.sp.wrapping_sub(1);
        self.bus.write_byte(self.sp, (value & 0xFF) as u8);
    }

    fn jump(&self, should_jump: bool) -> u16 {
            if should_jump {
                // Game Boy is little endian so read pc + 2 as most significant bit 
                // and pc + 1 as least significant bit
                let least_significant_byte = self.bus.read_byte(self.pc + 1) as u16;
                let most_significant_byte = self.bus.read_byte(self.pc + 2) as u16;
                (most_significant_byte << 8) | least_significant_byte
            } else {
                // If we don't jump we need to still move the program
                // counter forward by 3 since the jump instruction is 
                // 3 bytes wide (1 byte for tag and 2 bytes for jump address)
                self.pc.wrapping_add(3)
            }
    }

    fn add(&mut self, value: u8) -> u8 {
        let (new_value, did_overflow) = self.registers.a.overflowing_add(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = did_overflow;
        // Half Carry is set if adding the lower nibbles of the value and register A
        // together result in a value bigger than 0xF. If the result is larger than 0xF
        // than the addition caused a carry from the lower nibble to the upper nibble.
        self.registers.f.half_carry = (self.registers.a & 0xF) + (value & 0xF) > 0xF;
        new_value
    }

    fn adc(&mut self, value: u8) -> u8 {
        let carry_in = if self.registers.f.carry { 1 } else { 0 };
        let (intermediate, did_overflow1) = self.registers.a.overflowing_add(value);
        let (result, did_overflow2) = intermediate.overflowing_add(carry_in);

        self.registers.f.zero = result == 0;
        self.registers.f.subtract = false;
        // Hald carry: check if the lower nibbles plus the incoming carry exceed 0xF.
        self.registers.f.half_carry = ((self.registers.a & 0xF) + (value & 0xF) + carry_in) > 0xF;
        // Carry flag is set if either addition overflowed.
        self.registers.f.carry = did_overflow1 || did_overflow2;
        result
    }
    
    fn add_hl(&mut self, value: u16) -> u16 {
        let hl = self.registers.get_hl();
        let (new_hl, did_overflow) = hl.overflowing_add(value);
        // Zero flag remains unchanged for ADDHL
        self.registers.f.subtract = false;
        self.registers.f.half_carry = ((hl & 0xFFF) + (value & 0xFFF)) > 0xFFF;
        self.registers.f.carry = did_overflow;
        new_hl
    }

    fn sub(&mut self, value: u8) -> u8 {
        let (new_value, did_overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.zero = new_value ==  0;
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
        // Half-carry: if the lower nibble of A is less than the sum of the lower nibble of value and carry.
        self.registers.f.half_carry = (self.registers.a & 0xF) < ((value & 0xF) + carry_in);
        self.registers.f.carry = did_overflow1 || did_overflow2;
        result
    }
    
    fn and(&mut self, value: u8) -> u8 {
        let result = self.registers.a & value;
        self.registers.f.zero = result == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = true; // AND always sets the half-carry flag.
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

    fn read_next_word(&self) -> u16 { 0 }
    fn read_next_byte(&self) -> u8 { 0 }
}

impl Instruction {
    fn from_byte(byte: u8, prefixed: bool) -> Option<Instruction> {
        if prefixed {
            Instruction::from_byte_prefixed(byte)
        } else {
            Instruction::from_byte_not_prefixed(byte)
        }
    }

    fn from_byte_prefixed(byte: u8) -> Option<Instruction> {
        match byte {
            0x00 => Some(Instruction::RLC(PrefixTarget::B)),
            _ => /* TODO: Add mapping for rest of instructions */ None
        }
    }

    fn from_byte_not_prefixed(byte: u8) -> Option<Instruction> {
        match byte {
            0x02 => Some(Instruction::INC(IncDecTarget::BC)),
            _ => /* TODO: Add mapping for rest of instructions */ None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Provide default implementations so that our tests can create a CPU in a known state.
    impl Default for FlagsRegister {
        fn default() -> Self {
            FlagsRegister {
                zero: false,
                subtract: false,
                half_carry: false,
                carry: false,
            }
        }
    }

    impl Default for Registers {
        fn default() -> Self {
            Registers {
                a: 0,
                b: 0,
                c: 0,
                d: 0,
                e: 0,
                f: FlagsRegister::default(),
                h: 0,
                l: 0,
            }
        }
    }

    impl Default for MemoryBus {
        fn default() -> Self {
            MemoryBus {
                memory: [0; 0xFFFF],
            }
        }
    }

    impl Default for CPU {
        fn default() -> Self {
            CPU {
                registers: Registers::default(),
                pc: 0,
                sp: 0,
                bus: MemoryBus::default(),
            }
        }
    }

    // ===============================================
    // Tests for 8-bit ADD instruction
    // ===============================================
    #[test]
    fn test_add_no_overflow() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x03;
        cpu.registers.f = FlagsRegister::default(); // ensure flags start in a known state

        // 0x03 + 0x04 = 0x07
        let result = cpu.add(0x04);

        assert_eq!(result, 0x07);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, false);
        assert_eq!(cpu.registers.f.carry, false);
        assert_eq!(cpu.registers.f.half_carry, false);
    }

    #[test]
    fn test_add_half_carry() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x08;
        cpu.registers.f = FlagsRegister::default();

        // 0x08 + 0x09 = 0x11 (0x8 + 0x9 = 0x11 > 0xF, so half carry)
        let result = cpu.add(0x09);

        assert_eq!(result, 0x11);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, false);
        assert_eq!(cpu.registers.f.carry, false);
        assert_eq!(cpu.registers.f.half_carry, true);
    }

    #[test]
    fn test_add_overflow() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0xFF;
        cpu.registers.f = FlagsRegister::default();

        // 0xFF + 0x02 = 0x101 -> result 0x01 (overflow occurs)
        let result = cpu.add(0x02);

        assert_eq!(result, 0x01);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, false);
        assert_eq!(cpu.registers.f.carry, true);
        // Lower nibble: 0xF + 0x2 = 0x11 > 0xF
        assert_eq!(cpu.registers.f.half_carry, true);
    }

    #[test]
    fn test_add_zero_result() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x00;
        cpu.registers.f = FlagsRegister::default();

        let result = cpu.add(0x00);

        assert_eq!(result, 0x00);
        // Result is zero so zero flag must be set.
        assert_eq!(cpu.registers.f.zero, true);
        assert_eq!(cpu.registers.f.subtract, false);
        assert_eq!(cpu.registers.f.carry, false);
        assert_eq!(cpu.registers.f.half_carry, false);
    }

    // ===============================================
    // Tests for ADC (8-bit add with carry)
    // ===============================================
    #[test]
    fn test_adc_no_carry_in_no_overflow() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x03;
        cpu.registers.f = FlagsRegister::default();
        cpu.registers.f.carry = false;

        // 0x03 + 0x04 + 0 = 0x07
        let result = cpu.adc(0x04);

        assert_eq!(result, 0x07);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, false);
        assert_eq!(cpu.registers.f.carry, false);
        assert_eq!(cpu.registers.f.half_carry, false);
    }

    #[test]
    fn test_adc_with_carry_in() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x03;
        cpu.registers.f = FlagsRegister::default();
        cpu.registers.f.carry = true; // carry in = 1

        // 0x03 + 0x04 + 1 = 0x08
        let result = cpu.adc(0x04);

        assert_eq!(result, 0x08);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, false);
        // For 0x03 + 0x04 + 1, lower nibble: 0x3 + 0x4 + 1 = 0x8, so no half carry.
        assert_eq!(cpu.registers.f.half_carry, false);
        assert_eq!(cpu.registers.f.carry, false);
    }

    #[test]
    fn test_adc_overflow_and_half_carry() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0xF8;
        cpu.registers.f = FlagsRegister::default();
        cpu.registers.f.carry = true; // carry in = 1

        // 0xF8 + 0x0A + 1 = 0x103 -> result 0x03 with overflow.
        let result = cpu.adc(0x0A);

        assert_eq!(result, 0x03);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, false);
        // Lower nibble: (0x8 + 0xA + 1 = 0x13) exceeds 0xF, so half carry should be set.
        assert_eq!(cpu.registers.f.half_carry, true);
        assert_eq!(cpu.registers.f.carry, true);
    }

    // ===============================================
    // Tests for ADDHL (16-bit add to HL)
    // ===============================================
    #[test]
    fn test_add_hl_no_overflow() {
        let mut cpu = CPU::default();
        cpu.registers.set_hl(0x1234);
        // 0x1234 + 0x0100 = 0x1334
        let new_hl = cpu.add_hl(0x0100);
        cpu.registers.set_hl(new_hl);

        assert_eq!(cpu.registers.get_hl(), 0x1334);
        // In 16-bit addition, the subtract flag is cleared.
        assert_eq!(cpu.registers.f.subtract, false);
        // No half-carry or carry should occur.
        assert_eq!(cpu.registers.f.half_carry, false);
        assert_eq!(cpu.registers.f.carry, false);
    }

    #[test]
    fn test_add_hl_half_carry() {
        let mut cpu = CPU::default();
        // For example, HL = 0x0FF0 and adding 0x0010 should yield 0x1000.
        cpu.registers.set_hl(0x0FF0);
        let new_hl = cpu.add_hl(0x0010);
        cpu.registers.set_hl(new_hl);

        assert_eq!(cpu.registers.get_hl(), 0x1000);
        assert_eq!(cpu.registers.f.subtract, false);
        // Expect half-carry because (0xFF0 & 0xFFF) + (0x0010 & 0xFFF) > 0xFFF.
        assert_eq!(cpu.registers.f.half_carry, true);
        // No full 16-bit carry.
        assert_eq!(cpu.registers.f.carry, false);
    }

    #[test]
    fn test_add_hl_overflow() {
        let mut cpu = CPU::default();
        // HL = 0xFFFF and adding 0x0001 wraps to 0x0000.
        cpu.registers.set_hl(0xFFFF);
        let new_hl = cpu.add_hl(0x0001);
        cpu.registers.set_hl(new_hl);

        assert_eq!(cpu.registers.get_hl(), 0x0000);
        assert_eq!(cpu.registers.f.subtract, false);
        // Both half-carry and carry should be set.
        assert_eq!(cpu.registers.f.half_carry, true);
        assert_eq!(cpu.registers.f.carry, true);
    }

    // --- SUB Tests ---

    #[test]
    fn test_sub_no_borrow() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x08;
        cpu.registers.f = FlagsRegister::default();

        // 0x08 - 0x03 = 0x05, no borrow.
        let result = cpu.sub(0x03);
        assert_eq!(result, 0x05);
        assert_eq!(cpu.registers.f.zero, false);
        // SUB always sets subtract flag.
        assert_eq!(cpu.registers.f.subtract, true);
        assert_eq!(cpu.registers.f.carry, false);
        // No borrow from lower nibble: (0x08 & 0xF)=8 is not less than (0x03 & 0xF)=3.
        assert_eq!(cpu.registers.f.half_carry, false);
    }

    #[test]
    fn test_sub_half_carry() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x10;
        cpu.registers.f = FlagsRegister::default();

        // 0x10 - 0x01 = 0x0F.
        // Lower nibble: (0x10 & 0xF)=0, (0x01 & 0xF)=1, so a borrow occurs in the lower nibble.
        let result = cpu.sub(0x01);
        assert_eq!(result, 0x0F);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, true);
        assert_eq!(cpu.registers.f.carry, false);
        assert_eq!(cpu.registers.f.half_carry, true);
    }

    #[test]
    fn test_sub_borrow() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x03;
        cpu.registers.f = FlagsRegister::default();

        // 0x03 - 0x05 wraps around to 0xFE with borrow.
        let result = cpu.sub(0x05);
        assert_eq!(result, 0xFE);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, true);
        assert_eq!(cpu.registers.f.carry, true);
        // Lower nibble: 0x03 (3) < 0x05 (5) so half-carry true.
        assert_eq!(cpu.registers.f.half_carry, true);
    }

    #[test]
    fn test_sub_zero_result() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x05;
        cpu.registers.f = FlagsRegister::default();

        // 0x05 - 0x05 = 0.
        let result = cpu.sub(0x05);
        assert_eq!(result, 0x00);
        assert_eq!(cpu.registers.f.zero, true);
        assert_eq!(cpu.registers.f.subtract, true);
        assert_eq!(cpu.registers.f.carry, false);
        assert_eq!(cpu.registers.f.half_carry, false);
    }

    // --- SBC Tests ---
    #[test]
    fn test_sbc_no_carry_in_no_borrow() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x10;
        cpu.registers.f = FlagsRegister::default();
        cpu.registers.f.carry = false;

        // 0x10 - 0x05 = 0x0B; then subtract carry (0) = 0x0B.
        let result = cpu.sbc(0x05);
        assert_eq!(result, 0x0B);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, true);
        assert_eq!(cpu.registers.f.carry, false);
        // Lower nibble: (0x10 & 0xF)=0, (0x05 & 0xF)=5, 0 < 5 so half-carry true.
        assert_eq!(cpu.registers.f.half_carry, true);
    }

    #[test]
    fn test_sbc_with_carry_in_no_borrow() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x10;
        cpu.registers.f = FlagsRegister::default();
        cpu.registers.f.carry = true; // carry in = 1

        // 0x10 - 0x05 = 0x0B, then subtract carry: 0x0B - 1 = 0x0A.
        let result = cpu.sbc(0x05);
        assert_eq!(result, 0x0A);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, true);
        assert_eq!(cpu.registers.f.carry, false);
        // Lower nibble: (0x10 & 0xF)=0, (0x05 & 0xF) + 1 = 6, so 0 < 6 → half-carry true.
        assert_eq!(cpu.registers.f.half_carry, true);
    }

    #[test]
    fn test_sbc_borrow() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x03;
        cpu.registers.f = FlagsRegister::default();
        cpu.registers.f.carry = true; // carry in = 1

        // 0x03 - 0x05 = 0xFE with borrow; then subtract carry: 0xFE - 1 = 0xFD.
        let result = cpu.sbc(0x05);
        assert_eq!(result, 0xFD);
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, true);
        // Borrow occurred, so carry flag should be set.
        assert_eq!(cpu.registers.f.carry, true);
        // Lower nibble: (0x03 & 0xF)=3, (0x05 & 0xF)+1 = 6, and 3 < 6.
        assert_eq!(cpu.registers.f.half_carry, true);
    }

    #[test]
    fn test_sbc_zero_result() {
        let mut cpu = CPU::default();
        cpu.registers.a = 0x05;
        cpu.registers.f = FlagsRegister::default();
        cpu.registers.f.carry = false;

        // 0x05 - 0x05 = 0.
        let result = cpu.sbc(0x05);
        assert_eq!(result, 0x00);
        assert_eq!(cpu.registers.f.zero, true);
        assert_eq!(cpu.registers.f.subtract, true);
        assert_eq!(cpu.registers.f.carry, false);
        assert_eq!(cpu.registers.f.half_carry, false);
    }

    // AND, OR, XOR TESTS

    #[test]
    fn test_and_non_zero() {
        let mut cpu = CPU::default();
        // Set A to a value where ANDing with 0xAA produces a nonzero result.
        // For example, A = 0xCC (11001100) and 0xAA (10101010) gives 0x88 (10001000)
        cpu.registers.a = 0xCC;
        let result = cpu.and(0xAA);
        assert_eq!(result, 0x88);
        // The result is nonzero so zero flag is false.
        assert_eq!(cpu.registers.f.zero, false);
        // Subtract flag should be false.
        assert_eq!(cpu.registers.f.subtract, false);
        // AND always sets the half-carry flag to true.
        assert_eq!(cpu.registers.f.half_carry, true);
        // Carry flag is cleared.
        assert_eq!(cpu.registers.f.carry, false);
    }

    #[test]
    fn test_and_zero_result() {
        let mut cpu = CPU::default();
        // A = 0xF0 and operand 0x0F will yield 0xF0 & 0x0F = 0x00.
        cpu.registers.a = 0xF0;
        let result = cpu.and(0x0F);
        assert_eq!(result, 0x00);
        // Result is zero so zero flag is set.
        assert_eq!(cpu.registers.f.zero, true);
        assert_eq!(cpu.registers.f.subtract, false);
        // AND always sets half-carry.
        assert_eq!(cpu.registers.f.half_carry, true);
        assert_eq!(cpu.registers.f.carry, false);
    }

    #[test]
    fn test_or_non_zero() {
        let mut cpu = CPU::default();
        // For OR, choose A = 0x10 and operand 0x02 so that 0x10 | 0x02 = 0x12.
        cpu.registers.a = 0x10;
        let result = cpu.or(0x02);
        assert_eq!(result, 0x12);
        // Nonzero result: zero flag should be false.
        assert_eq!(cpu.registers.f.zero, false);
        // OR clears subtract flag.
        assert_eq!(cpu.registers.f.subtract, false);
        // For OR, half-carry flag is cleared.
        assert_eq!(cpu.registers.f.half_carry, false);
        // Carry flag is also cleared.
        assert_eq!(cpu.registers.f.carry, false);
    }

    #[test]
    fn test_or_zero_result() {
        let mut cpu = CPU::default();
        // Setting both A and the operand to 0 gives a result of 0.
        cpu.registers.a = 0x00;
        let result = cpu.or(0x00);
        assert_eq!(result, 0x00);
        // Result is zero so the zero flag should be set.
        assert_eq!(cpu.registers.f.zero, true);
        assert_eq!(cpu.registers.f.subtract, false);
        assert_eq!(cpu.registers.f.half_carry, false);
        assert_eq!(cpu.registers.f.carry, false);
    }

    #[test]
    fn test_xor_zero_result() {
        let mut cpu = CPU::default();
        // XORing a value with itself yields zero.
        cpu.registers.a = 0xAA;
        let result = cpu.xor(0xAA);
        assert_eq!(result, 0x00);
        // Result is zero so zero flag is set.
        assert_eq!(cpu.registers.f.zero, true);
        // Subtract, half-carry, and carry flags should be false.
        assert_eq!(cpu.registers.f.subtract, false);
        assert_eq!(cpu.registers.f.half_carry, false);
        assert_eq!(cpu.registers.f.carry, false);
    }

    #[test]
    fn test_xor_non_zero() {
        let mut cpu = CPU::default();
        // For example, A = 0xF0 and operand = 0x0F gives 0xF0 ^ 0x0F = 0xFF.
        cpu.registers.a = 0xF0;
        let result = cpu.xor(0x0F);
        assert_eq!(result, 0xFF);
        // Nonzero result: zero flag should be false.
        assert_eq!(cpu.registers.f.zero, false);
        assert_eq!(cpu.registers.f.subtract, false);
        assert_eq!(cpu.registers.f.half_carry, false);
        assert_eq!(cpu.registers.f.carry, false);
    }
}


