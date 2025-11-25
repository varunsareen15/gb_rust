use std::ptr;

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

#[derive(Clone)]
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

    fn write_byte(&self, _address: u16, _byte: u8) {}
}

impl Registers {
    fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (u8::from(self.f.clone()) as u16)
    }

    fn set_af(&mut self, value: u16) {
        self.a = ((value & 0xFF00) >> 8) as u8;
        self.f = FlagsRegister::from((value & 0xFF) as u8);
    }

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
    NOP,
    ADD(ArithmeticTarget),
    JP(JumpTest),
    LD(LoadType),
    PUSH(StackTarget),
    POP(StackTarget),
    CALL(JumpTest),
    RET(JumpTest),
    RETI,
    INC(IncDecTarget),
    DEC(IncDecTarget),
    RLC(PrefixTarget),
    ADDHL(ArithmeticHLTarget),
    ADC(ArithmeticTarget),
    SUB(SubtractionTarget),
    SBC(SubtractionTarget),
    AND(LogicalTarget),
    OR(LogicalTarget),
    XOR(LogicalTarget),
    CP(SubtractionTarget),
    JR(JumpTest),
    ADDSP,
    DI,
    EI,
    LDHL,
}

enum ArithmeticTarget {
    A, B, C, D, E, H, L, HL, Imm8
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
    Always,
    HL,
}

enum LoadByteTarget {
    A, B, C, D, E, H, L, HLI, HLD, BC, DE, A8, A16, HL
}

enum LoadByteSource {
    A, B, C, D, E, H, L, D8, HLI, HLD, BC, DE, A8, A16, HL
}

enum LoadWordTarget {
    BC, DE, HL, SP, A16
}

enum LoadWordSource {
    D16, SP, HL
}

enum LoadType {
    Byte(LoadByteTarget, LoadByteSource),
    Word(LoadWordTarget, LoadWordSource),
}

enum StackTarget {
    BC,
    DE,
    HL,
    AF,
}

enum IncDecTarget {
    A, B, C, D, E, H, L, BC, DE, HL, SP, HLREF
}

enum PrefixTarget {
    A, B, C, D, E, H, L, HL,
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
            Instruction::NOP => self.pc.wrapping_add(1),
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
                    ArithmeticTarget::HL => {
                        let value = self.bus.read_byte(self.registers.get_hl());
                        let new_value = self.add(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::Imm8 => {
                        let value = self.read_next_byte();
                        let new_value = self.add(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(2)
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
                    ArithmeticTarget::HL => {
                        let value = self.bus.read_byte(self.registers.get_hl());
                        let new_value = self.adc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    ArithmeticTarget::Imm8 => {
                        let value = self.read_next_byte();
                        let new_value = self.adc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(2)
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
            Instruction::XOR(target) => { // Finished XOR
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
            Instruction::CP(target) => { // FInished CP 
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
            Instruction::INC(target) => {
                match target {
                    IncDecTarget::A => {
                        let value = self.registers.a;
                        let new_value = self.inc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::B => {
                        let value = self.registers.b;
                        let new_value = self.inc(value);
                        self.registers.b = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::C => {
                        let value = self.registers.c;
                        let new_value = self.inc(value);
                        self.registers.c = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::D => {
                        let value = self.registers.d;
                        let new_value = self.inc(value);
                        self.registers.d = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::E => {
                        let value = self.registers.e;
                        let new_value = self.inc(value);
                        self.registers.e = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::H => {
                        let value = self.registers.h;
                        let new_value = self.inc(value);
                        self.registers.h = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::L => {
                        let value = self.registers.l;
                        let new_value = self.inc(value);
                        self.registers.l = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::BC => {
                        let value = self.registers.get_bc();
                        let new_value = value.wrapping_add(1);
                        self.registers.set_bc(new_value);
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::DE => {
                        let value = self.registers.get_de();
                        let new_value = value.wrapping_add(1);
                        self.registers.set_de(new_value);
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::HL => {
                        let value = self.registers.get_hl();
                        let new_value = value.wrapping_add(1);
                        self.registers.set_hl(new_value);
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::SP => {
                        self.sp = self.sp.wrapping_add(1);
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::HLREF => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        let new_value = self.inc(value);
                        self.bus.write_byte(addr, new_value);
                        self.pc.wrapping_add(1)
                    }
                }
            }
            Instruction::DEC(target) => {
                match target {
                    IncDecTarget::A => {
                        let value = self.registers.a;
                        let new_value = self.dec(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::B => {
                        let value = self.registers.b;
                        let new_value = self.dec(value);
                        self.registers.b = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::C => {
                        let value = self.registers.c;
                        let new_value = self.dec(value);
                        self.registers.c = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::D => {
                        let value = self.registers.d;
                        let new_value = self.dec(value);
                        self.registers.d = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::E => {
                        let value = self.registers.e;
                        let new_value = self.dec(value);
                        self.registers.e = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::H => {
                        let value = self.registers.h;
                        let new_value = self.dec(value);
                        self.registers.h = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::L => {
                        let value = self.registers.l;
                        let new_value = self.dec(value);
                        self.registers.l = new_value;
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::BC => {
                        let value = self.registers.get_bc();
                        let new_value = value.wrapping_sub(1);
                        self.registers.set_bc(new_value);
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::DE => {
                        let value = self.registers.get_de();
                        let new_value = value.wrapping_sub(1);
                        self.registers.set_de(new_value);
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::HL => {
                        let value = self.registers.get_hl();
                        let new_value = value.wrapping_sub(1);
                        self.registers.set_hl(new_value);
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::SP => {
                        self.sp = self.sp.wrapping_sub(1);
                        self.pc.wrapping_add(1)
                    }
                    IncDecTarget::HLREF => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        let new_value = self.dec(value);
                        self.bus.write_byte(addr, new_value);
                        self.pc.wrapping_add(1)
                    }
                }
            }
            Instruction::JP(test) => {
                let jump_condition = match test {
                    JumpTest::NotZero => !self.registers.f.zero,
                    JumpTest::NotCarry => !self.registers.f.carry,
                    JumpTest::Zero => self.registers.f.zero,
                    JumpTest::Carry => self.registers.f.carry,
                    JumpTest::Always => true,
                    JumpTest::HL => {
                        self.pc = self.registers.get_hl();
                        return self.pc;
                    }
                };
                self.jump(jump_condition)
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
                            },
                            LoadByteSource::HLD => {
                                let value = self.bus.read_byte(self.registers.get_hl());
                                self.registers.set_hl(self.registers.get_hl().wrapping_sub(1));
                                value
                            },
                            LoadByteSource::BC => self.bus.read_byte(self.registers.get_bc()),
                            LoadByteSource::DE => self.bus.read_byte(self.registers.get_de()),
                            LoadByteSource::A8 => self.bus.read_byte(0xFF00 + self.read_next_byte() as u16),
                            LoadByteSource::A16 => self.bus.read_byte(self.read_next_word()),
                            LoadByteSource::HL => self.bus.read_byte(self.registers.get_hl()),
                        };
                        match target {
                            LoadByteTarget::A => self.registers.a = source_value,
                            LoadByteTarget::B => self.registers.b = source_value,
                            LoadByteTarget::C => self.registers.c = source_value,
                            LoadByteTarget::D => self.registers.d = source_value,
                            LoadByteTarget::E => self.registers.e = source_value,
                            LoadByteTarget::H => self.registers.h = source_value,
                            LoadByteTarget::L => self.registers.l = source_value,
                            LoadByteTarget::HL => self.bus.write_byte(self.registers.get_hl(), source_value),
                            LoadByteTarget::HLI => {
                                self.bus.write_byte(self.registers.get_hl(), source_value);
                                self.registers.set_hl(self.registers.get_hl().wrapping_add(1));
                            },
                            LoadByteTarget::HLD => {
                                self.bus.write_byte(self.registers.get_hl(), source_value);
                                self.registers.set_hl(self.registers.get_hl().wrapping_sub(1));
                            },
                            LoadByteTarget::BC => self.bus.write_byte(self.registers.get_bc(), source_value),
                            LoadByteTarget::DE => self.bus.write_byte(self.registers.get_de(), source_value),
                            LoadByteTarget::A8 => self.bus.write_byte(0xFF00 + self.read_next_byte() as u16, source_value),
                            LoadByteTarget::A16 => self.bus.write_byte(self.read_next_word(), source_value),
                        };
                        match source {
                            LoadByteSource::D8 | LoadByteSource::A8 => self.pc.wrapping_add(2),
                            LoadByteSource::A16 => self.pc.wrapping_add(3),
                            _                  => self.pc.wrapping_add(1),
                        }
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
                                self.bus.write_byte(self.read_next_word(), (source_value & 0xFF) as u8);
                                self.bus.write_byte(self.read_next_word().wrapping_add(1), (source_value >> 8) as u8);
                            }
                        };
                        match source {
                            LoadWordSource::D16 => self.pc.wrapping_add(3),
                            _ => self.pc.wrapping_add(1),
                        }
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
                self.pc.wrapping_add(1)
            }
            Instruction::POP(target) => {
                let result = self.pop();
                match target {
                    StackTarget::BC => self.registers.set_bc(result),
                    StackTarget::DE => self.registers.set_de(result),
                    StackTarget::HL => self.registers.set_hl(result),
                    StackTarget::AF => self.registers.set_af(result),
                };
                self.pc.wrapping_add(1)
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
                self.call(jump_condition)
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
                self.return_(jump_condition)
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
                self.jr(jump_condition)
            }
            Instruction::ADDSP => {
                let offset = self.read_next_byte() as i8;
                let new_sp = self.sp.wrapping_add(offset as u16);
                self.registers.f.zero = false;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.sp & 0xF) + (offset as u16 & 0xF) > 0xF;
                self.registers.f.carry = (self.sp & 0xFF) + (offset as u16 & 0xFF) > 0xFF;
                self.sp = new_sp;
                self.pc.wrapping_add(2)
            }
            Instruction::DI => {
                // TODO: implement interrupts
                self.pc.wrapping_add(1)
            }
            Instruction::EI => {
                // TODO: implement interrupts
                self.pc.wrapping_add(1)
            }
            Instruction::LDHL => {
                let offset = self.read_next_byte() as i8;
                let new_hl = self.sp.wrapping_add(offset as u16);
                self.registers.f.zero = false;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = (self.sp & 0xF) + (offset as u16 & 0xF) > 0xF;
                self.registers.f.carry = (self.sp & 0xFF) + (offset as u16 & 0xFF) > 0xFF;
                self.registers.set_hl(new_hl);
                self.pc.wrapping_add(2)
            }
            Instruction::RETI => {
                // TODO: implement interrupts
                self.return_(true)
            }
            Instruction::RLC(target) => {
                match target {
                    PrefixTarget::A => {
                        let value = self.registers.a;
                        let new_value = self.rlc(value);
                        self.registers.a = new_value;
                        self.pc.wrapping_add(2)
                    }
                    PrefixTarget::B => {
                        let value = self.registers.b;
                        let new_value = self.rlc(value);
                        self.registers.b = new_value;
                        self.pc.wrapping_add(2)
                    }
                    PrefixTarget::C => {
                        let value = self.registers.c;
                        let new_value = self.rlc(value);
                        self.registers.c = new_value;
                        self.pc.wrapping_add(2)
                    }
                    PrefixTarget::D => {
                        let value = self.registers.d;
                        let new_value = self.rlc(value);
                        self.registers.d = new_value;
                        self.pc.wrapping_add(2)
                    }
                    PrefixTarget::E => {
                        let value = self.registers.e;
                        let new_value = self.rlc(value);
                        self.registers.e = new_value;
                        self.pc.wrapping_add(2)
                    }
                    PrefixTarget::H => {
                        let value = self.registers.h;
                        let new_value = self.rlc(value);
                        self.registers.h = new_value;
                        self.pc.wrapping_add(2)
                    }
                    PrefixTarget::L => {
                        let value = self.registers.l;
                        let new_value = self.rlc(value);
                        self.registers.l = new_value;
                        self.pc.wrapping_add(2)
                    }
                    PrefixTarget::HL => {
                        let addr = self.registers.get_hl();
                        let value = self.bus.read_byte(addr);
                        let new_value = self.rlc(value);
                        self.bus.write_byte(addr, new_value);
                        self.pc.wrapping_add(2)
                    }
                }
            }
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
        self.bus.write_byte(self.sp, (value >> 8) as u8);

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

    fn jr(&self, should_jump: bool) -> u16 {
        if should_jump {
            let offset = self.read_next_byte() as i8;
            self.pc.wrapping_add(2).wrapping_add(offset as u16)
        } else {
            self.pc.wrapping_add(2)
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

    fn rlc(&mut self, value: u8) -> u8 {
        let carry = value >> 7;
        let new_value = (value << 1) | carry;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = carry == 1;
        new_value
    }

    fn read_next_word(&self) -> u16 {
        // Game Boy is little endian so read pc + 2 as most significant bit
        // and pc + 1 as least significant bit
        let least_significant_byte = self.bus.read_byte(self.pc + 1) as u16;
        let most_significant_byte = self.bus.read_byte(self.pc + 2) as u16;
        (most_significant_byte << 8) | least_significant_byte
    }

    fn read_next_byte(&self) -> u8 {
        self.bus.read_byte(self.pc + 1)
    }
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
            0x01 => Some(Instruction::RLC(PrefixTarget::C)),
            0x02 => Some(Instruction::RLC(PrefixTarget::D)),
            0x03 => Some(Instruction::RLC(PrefixTarget::E)),
            0x04 => Some(Instruction::RLC(PrefixTarget::H)),
            0x05 => Some(Instruction::RLC(PrefixTarget::L)),
            0x06 => Some(Instruction::RLC(PrefixTarget::HL)),
            0x07 => Some(Instruction::RLC(PrefixTarget::A)),
            _ => None
        }
    }

    fn from_byte_not_prefixed(byte: u8) -> Option<Instruction> {
        match byte {
            0x00 => Some(Instruction::NOP),
            0x01 => Some(Instruction::LD(LoadType::Word(LoadWordTarget::BC, LoadWordSource::D16))),
            0x02 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::BC, LoadByteSource::A))),
            0x03 => Some(Instruction::INC(IncDecTarget::BC)),
            0x04 => Some(Instruction::INC(IncDecTarget::B)),
            0x05 => Some(Instruction::DEC(IncDecTarget::B)),
            0x06 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::B, LoadByteSource::D8))),
            0x08 => Some(Instruction::LD(LoadType::Word(LoadWordTarget::A16, LoadWordSource::SP))),
            0x09 => Some(Instruction::ADDHL(ArithmeticHLTarget::BC)),
            0x0A => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::BC))),
            0x0B => Some(Instruction::DEC(IncDecTarget::BC)),
            0x0C => Some(Instruction::INC(IncDecTarget::C)),
            0x0D => Some(Instruction::DEC(IncDecTarget::C)),
            0x0E => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::C, LoadByteSource::D8))),

            0x11 => Some(Instruction::LD(LoadType::Word(LoadWordTarget::DE, LoadWordSource::D16))),
            0x12 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::DE, LoadByteSource::A))),
            0x13 => Some(Instruction::INC(IncDecTarget::DE)),
            0x14 => Some(Instruction::INC(IncDecTarget::D)),
            0x15 => Some(Instruction::DEC(IncDecTarget::D)),
            0x16 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::D, LoadByteSource::D8))),
            0x18 => Some(Instruction::JR(JumpTest::Always)),
            0x19 => Some(Instruction::ADDHL(ArithmeticHLTarget::DE)),
            0x1A => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::DE))),
            0x1B => Some(Instruction::DEC(IncDecTarget::DE)),
            0x1C => Some(Instruction::INC(IncDecTarget::E)),
            0x1D => Some(Instruction::DEC(IncDecTarget::E)),
            0x1E => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::E, LoadByteSource::D8))),

            0x20 => Some(Instruction::JR(JumpTest::NotZero)),
            0x21 => Some(Instruction::LD(LoadType::Word(LoadWordTarget::HL, LoadWordSource::D16))),
            0x22 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::HLI, LoadByteSource::A))),
            0x23 => Some(Instruction::INC(IncDecTarget::HL)),
            0x24 => Some(Instruction::INC(IncDecTarget::H)),
            0x25 => Some(Instruction::DEC(IncDecTarget::H)),
            0x26 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::H, LoadByteSource::D8))),
            0x28 => Some(Instruction::JR(JumpTest::Zero)),
            0x29 => Some(Instruction::ADDHL(ArithmeticHLTarget::HL)),
            0x2A => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::HLI))),
            0x2B => Some(Instruction::DEC(IncDecTarget::HL)),
            0x2C => Some(Instruction::INC(IncDecTarget::L)),
            0x2D => Some(Instruction::DEC(IncDecTarget::L)),
            0x2E => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::L, LoadByteSource::D8))),

            0x30 => Some(Instruction::JR(JumpTest::NotCarry)),
            0x31 => Some(Instruction::LD(LoadType::Word(LoadWordTarget::SP, LoadWordSource::D16))),
            0x32 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::HLD, LoadByteSource::A))),
            0x33 => Some(Instruction::INC(IncDecTarget::SP)),
            0x34 => Some(Instruction::INC(IncDecTarget::HLREF)),
            0x35 => Some(Instruction::DEC(IncDecTarget::HLREF)),
            0x36 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::HL, LoadByteSource::D8))),
            0x38 => Some(Instruction::JR(JumpTest::Carry)),
            0x39 => Some(Instruction::ADDHL(ArithmeticHLTarget::SP)),
            0x3A => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::HLD))),
            0x3B => Some(Instruction::DEC(IncDecTarget::SP)),
            0x3C => Some(Instruction::INC(IncDecTarget::A)),
            0x3D => Some(Instruction::DEC(IncDecTarget::A)),
            0x3E => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::D8))),

            0x40 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::B, LoadByteSource::B))),
            0x41 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::B, LoadByteSource::C))),
            0x42 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::B, LoadByteSource::D))),
            0x43 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::B, LoadByteSource::E))),
            0x44 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::B, LoadByteSource::H))),
            0x45 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::B, LoadByteSource::L))),
            0x46 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::B, LoadByteSource::HL))),
            0x47 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::B, LoadByteSource::A))),
            0x48 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::C, LoadByteSource::B))),
            0x49 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::C, LoadByteSource::C))),
            0x4A => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::C, LoadByteSource::D))),
            0x4B => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::C, LoadByteSource::E))),
            0x4C => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::C, LoadByteSource::H))),
            0x4D => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::C, LoadByteSource::L))),
            0x4E => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::C, LoadByteSource::HL))),
            0x4F => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::C, LoadByteSource::A))),

            0x50 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::D, LoadByteSource::B))),
            0x51 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::D, LoadByteSource::C))),
            0x52 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::D, LoadByteSource::D))),
            0x53 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::D, LoadByteSource::E))),
            0x54 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::D, LoadByteSource::H))),
            0x55 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::D, LoadByteSource::L))),
            0x56 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::D, LoadByteSource::HL))),
            0x57 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::D, LoadByteSource::A))),
            0x58 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::E, LoadByteSource::B))),
            0x59 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::E, LoadByteSource::C))),
            0x5A => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::E, LoadByteSource::D))),
            0x5B => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::E, LoadByteSource::E))),
            0x5C => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::E, LoadByteSource::H))),
            0x5D => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::E, LoadByteSource::L))),
            0x5E => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::E, LoadByteSource::HL))),
            0x5F => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::E, LoadByteSource::A))),

            0x60 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::H, LoadByteSource::B))),
            0x61 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::H, LoadByteSource::C))),
            0x62 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::H, LoadByteSource::D))),
            0x63 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::H, LoadByteSource::E))),
            0x64 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::H, LoadByteSource::H))),
            0x65 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::H, LoadByteSource::L))),
            0x66 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::H, LoadByteSource::HL))),
            0x67 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::H, LoadByteSource::A))),
            0x68 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::L, LoadByteSource::B))),
            0x69 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::L, LoadByteSource::C))),
            0x6A => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::L, LoadByteSource::D))),
            0x6B => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::L, LoadByteSource::E))),
            0x6C => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::L, LoadByteSource::H))),
            0x6D => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::L, LoadByteSource::L))),
            0x6E => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::L, LoadByteSource::HL))),
            0x6F => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::L, LoadByteSource::A))),

            0x70 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::HL, LoadByteSource::B))),
            0x71 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::HL, LoadByteSource::C))),
            0x72 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::HL, LoadByteSource::D))),
            0x73 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::HL, LoadByteSource::E))),
            0x74 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::HL, LoadByteSource::H))),
            0x75 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::HL, LoadByteSource::L))),
            0x77 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::HL, LoadByteSource::A))),
            0x78 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::B))),
            0x79 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::C))),
            0x7A => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::D))),
            0x7B => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::E))),
            0x7C => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::H))),
            0x7D => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::L))),
            0x7E => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::HL))),
            0x7F => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::A))),

            0x80 => Some(Instruction::ADD(ArithmeticTarget::B)),
            0x81 => Some(Instruction::ADD(ArithmeticTarget::C)),
            0x82 => Some(Instruction::ADD(ArithmeticTarget::D)),
            0x83 => Some(Instruction::ADD(ArithmeticTarget::E)),
            0x84 => Some(Instruction::ADD(ArithmeticTarget::H)),
            0x85 => Some(Instruction::ADD(ArithmeticTarget::L)),
            0x86 => Some(Instruction::ADD(ArithmeticTarget::HL)),
            0x87 => Some(Instruction::ADD(ArithmeticTarget::A)),
            0x88 => Some(Instruction::ADC(ArithmeticTarget::B)),
            0x89 => Some(Instruction::ADC(ArithmeticTarget::C)),
            0x8A => Some(Instruction::ADC(ArithmeticTarget::D)),
            0x8B => Some(Instruction::ADC(ArithmeticTarget::E)),
            0x8C => Some(Instruction::ADC(ArithmeticTarget::H)),
            0x8D => Some(Instruction::ADC(ArithmeticTarget::L)),
            0x8E => Some(Instruction::ADC(ArithmeticTarget::HL)),
            0x8F => Some(Instruction::ADC(ArithmeticTarget::A)),

            0x90 => Some(Instruction::SUB(SubtractionTarget::B)),
            0x91 => Some(Instruction::SUB(SubtractionTarget::C)),
            0x92 => Some(Instruction::SUB(SubtractionTarget::D)),
            0x93 => Some(Instruction::SUB(SubtractionTarget::E)),
            0x94 => Some(Instruction::SUB(SubtractionTarget::H)),
            0x95 => Some(Instruction::SUB(SubtractionTarget::L)),
            0x96 => Some(Instruction::SUB(SubtractionTarget::HL)),
            0x97 => Some(Instruction::SUB(SubtractionTarget::A)),
            0x98 => Some(Instruction::SBC(SubtractionTarget::B)),
            0x99 => Some(Instruction::SBC(SubtractionTarget::C)),
            0x9A => Some(Instruction::SBC(SubtractionTarget::D)),
            0x9B => Some(Instruction::SBC(SubtractionTarget::E)),
            0x9C => Some(Instruction::SBC(SubtractionTarget::H)),
            0x9D => Some(Instruction::SBC(SubtractionTarget::L)),
            0x9E => Some(Instruction::SBC(SubtractionTarget::HL)),
            0x9F => Some(Instruction::SBC(SubtractionTarget::A)),

            0xA0 => Some(Instruction::AND(LogicalTarget::B)),
            0xA1 => Some(Instruction::AND(LogicalTarget::C)),
            0xA2 => Some(Instruction::AND(LogicalTarget::D)),
            0xA3 => Some(Instruction::AND(LogicalTarget::E)),
            0xA4 => Some(Instruction::AND(LogicalTarget::H)),
            0xA5 => Some(Instruction::AND(LogicalTarget::L)),
            0xA6 => Some(Instruction::AND(LogicalTarget::HL)),
            0xA7 => Some(Instruction::AND(LogicalTarget::A)),
            0xA8 => Some(Instruction::XOR(LogicalTarget::B)),
            0xA9 => Some(Instruction::XOR(LogicalTarget::C)),
            0xAA => Some(Instruction::XOR(LogicalTarget::D)),
            0xAB => Some(Instruction::XOR(LogicalTarget::E)),
            0xAC => Some(Instruction::XOR(LogicalTarget::H)),
            0xAD => Some(Instruction::XOR(LogicalTarget::L)),
            0xAE => Some(Instruction::XOR(LogicalTarget::HL)),
            0xAF => Some(Instruction::XOR(LogicalTarget::A)),

            0xB0 => Some(Instruction::OR(LogicalTarget::B)),
            0xB1 => Some(Instruction::OR(LogicalTarget::C)),
            0xB2 => Some(Instruction::OR(LogicalTarget::D)),
            0xB3 => Some(Instruction::OR(LogicalTarget::E)),
            0xB4 => Some(Instruction::OR(LogicalTarget::H)),
            0xB5 => Some(Instruction::OR(LogicalTarget::L)),
            0xB6 => Some(Instruction::OR(LogicalTarget::HL)),
            0xB7 => Some(Instruction::OR(LogicalTarget::A)),
            0xB8 => Some(Instruction::CP(SubtractionTarget::B)),
            0xB9 => Some(Instruction::CP(SubtractionTarget::C)),
            0xBA => Some(Instruction::CP(SubtractionTarget::D)),
            0xBB => Some(Instruction::CP(SubtractionTarget::E)),
            0xBC => Some(Instruction::CP(SubtractionTarget::H)),
            0xBD => Some(Instruction::CP(SubtractionTarget::L)),
            0xBE => Some(Instruction::CP(SubtractionTarget::HL)),
            0xBF => Some(Instruction::CP(SubtractionTarget::A)),

            0xC0 => Some(Instruction::RET(JumpTest::NotZero)),
            0xC1 => Some(Instruction::POP(StackTarget::BC)),
            0xC2 => Some(Instruction::JP(JumpTest::NotZero)),
            0xC3 => Some(Instruction::JP(JumpTest::Always)),
            0xC4 => Some(Instruction::CALL(JumpTest::NotZero)),
            0xC5 => Some(Instruction::PUSH(StackTarget::BC)),
            0xC6 => Some(Instruction::ADD(ArithmeticTarget::Imm8)),
            0xC8 => Some(Instruction::RET(JumpTest::Zero)),
            0xC9 => Some(Instruction::RET(JumpTest::Always)),
            0xCA => Some(Instruction::JP(JumpTest::Zero)),
            0xCC => Some(Instruction::CALL(JumpTest::Zero)),
            0xCD => Some(Instruction::CALL(JumpTest::Always)),
            0xCE => Some(Instruction::ADC(ArithmeticTarget::Imm8)),

            0xD0 => Some(Instruction::RET(JumpTest::NotCarry)),
            0xD1 => Some(Instruction::POP(StackTarget::DE)),
            0xD2 => Some(Instruction::JP(JumpTest::NotCarry)),
            0xD4 => Some(Instruction::CALL(JumpTest::NotCarry)),
            0xD5 => Some(Instruction::PUSH(StackTarget::DE)),
            0xD6 => Some(Instruction::SUB(SubtractionTarget::Imm8)),
            0xD8 => Some(Instruction::RET(JumpTest::Carry)),
            0xD9 => Some(Instruction::RETI),
            0xDA => Some(Instruction::JP(JumpTest::Carry)),
            0xDC => Some(Instruction::CALL(JumpTest::Carry)),
            0xDE => Some(Instruction::SBC(SubtractionTarget::Imm8)),

            0xE0 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A8, LoadByteSource::A))),
            0xE1 => Some(Instruction::POP(StackTarget::HL)),
            0xE2 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::C, LoadByteSource::A))),
            0xE5 => Some(Instruction::PUSH(StackTarget::HL)),
            0xE6 => Some(Instruction::AND(LogicalTarget::Imm8)),
            0xE8 => Some(Instruction::ADDSP),
            0xE9 => Some(Instruction::JP(JumpTest::HL)),
            0xEA => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A16, LoadByteSource::A))),

            0xF0 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::A8))),
            0xF1 => Some(Instruction::POP(StackTarget::AF)),
            0xF2 => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::C))),
            0xF3 => Some(Instruction::DI),
            0xF5 => Some(Instruction::PUSH(StackTarget::AF)),
            0xF6 => Some(Instruction::OR(LogicalTarget::Imm8)),
            0xF8 => Some(Instruction::LDHL),
            0xF9 => Some(Instruction::LD(LoadType::Word(LoadWordTarget::SP, LoadWordSource::HL))),
            0xFA => Some(Instruction::LD(LoadType::Byte(LoadByteTarget::A, LoadByteSource::A16))),
            0xFB => Some(Instruction::EI),
            0xFE => Some(Instruction::CP(SubtractionTarget::Imm8)),

            _ => None
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


