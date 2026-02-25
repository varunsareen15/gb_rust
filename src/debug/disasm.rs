/// Disassemble one instruction at `addr` using `read_fn` for side-effect-free reads.
/// Returns (mnemonic_string, byte_count).
pub fn disassemble<F: Fn(u16) -> u8>(addr: u16, read_fn: F) -> (String, u8) {
    let opcode = read_fn(addr);

    if opcode == 0xCB {
        let cb = read_fn(addr.wrapping_add(1));
        let s = disassemble_cb(cb);
        return (s, 2);
    }

    match opcode {
        0x00 => ("NOP".into(), 1),
        0x01 => { let w = read_word(addr, &read_fn); (format!("LD BC,${:04X}", w), 3) }
        0x02 => ("LD (BC),A".into(), 1),
        0x03 => ("INC BC".into(), 1),
        0x04 => ("INC B".into(), 1),
        0x05 => ("DEC B".into(), 1),
        0x06 => { let b = read_fn(addr.wrapping_add(1)); (format!("LD B,${:02X}", b), 2) }
        0x07 => ("RLCA".into(), 1),
        0x08 => { let w = read_word(addr, &read_fn); (format!("LD (${:04X}),SP", w), 3) }
        0x09 => ("ADD HL,BC".into(), 1),
        0x0A => ("LD A,(BC)".into(), 1),
        0x0B => ("DEC BC".into(), 1),
        0x0C => ("INC C".into(), 1),
        0x0D => ("DEC C".into(), 1),
        0x0E => { let b = read_fn(addr.wrapping_add(1)); (format!("LD C,${:02X}", b), 2) }
        0x0F => ("RRCA".into(), 1),

        0x10 => ("STOP".into(), 2),
        0x11 => { let w = read_word(addr, &read_fn); (format!("LD DE,${:04X}", w), 3) }
        0x12 => ("LD (DE),A".into(), 1),
        0x13 => ("INC DE".into(), 1),
        0x14 => ("INC D".into(), 1),
        0x15 => ("DEC D".into(), 1),
        0x16 => { let b = read_fn(addr.wrapping_add(1)); (format!("LD D,${:02X}", b), 2) }
        0x17 => ("RLA".into(), 1),
        0x18 => { let b = read_fn(addr.wrapping_add(1)); (format!("JR ${:02X}", b), 2) }
        0x19 => ("ADD HL,DE".into(), 1),
        0x1A => ("LD A,(DE)".into(), 1),
        0x1B => ("DEC DE".into(), 1),
        0x1C => ("INC E".into(), 1),
        0x1D => ("DEC E".into(), 1),
        0x1E => { let b = read_fn(addr.wrapping_add(1)); (format!("LD E,${:02X}", b), 2) }
        0x1F => ("RRA".into(), 1),

        0x20 => { let b = read_fn(addr.wrapping_add(1)); (format!("JR NZ,${:02X}", b), 2) }
        0x21 => { let w = read_word(addr, &read_fn); (format!("LD HL,${:04X}", w), 3) }
        0x22 => ("LD (HL+),A".into(), 1),
        0x23 => ("INC HL".into(), 1),
        0x24 => ("INC H".into(), 1),
        0x25 => ("DEC H".into(), 1),
        0x26 => { let b = read_fn(addr.wrapping_add(1)); (format!("LD H,${:02X}", b), 2) }
        0x27 => ("DAA".into(), 1),
        0x28 => { let b = read_fn(addr.wrapping_add(1)); (format!("JR Z,${:02X}", b), 2) }
        0x29 => ("ADD HL,HL".into(), 1),
        0x2A => ("LD A,(HL+)".into(), 1),
        0x2B => ("DEC HL".into(), 1),
        0x2C => ("INC L".into(), 1),
        0x2D => ("DEC L".into(), 1),
        0x2E => { let b = read_fn(addr.wrapping_add(1)); (format!("LD L,${:02X}", b), 2) }
        0x2F => ("CPL".into(), 1),

        0x30 => { let b = read_fn(addr.wrapping_add(1)); (format!("JR NC,${:02X}", b), 2) }
        0x31 => { let w = read_word(addr, &read_fn); (format!("LD SP,${:04X}", w), 3) }
        0x32 => ("LD (HL-),A".into(), 1),
        0x33 => ("INC SP".into(), 1),
        0x34 => ("INC (HL)".into(), 1),
        0x35 => ("DEC (HL)".into(), 1),
        0x36 => { let b = read_fn(addr.wrapping_add(1)); (format!("LD (HL),${:02X}", b), 2) }
        0x37 => ("SCF".into(), 1),
        0x38 => { let b = read_fn(addr.wrapping_add(1)); (format!("JR C,${:02X}", b), 2) }
        0x39 => ("ADD HL,SP".into(), 1),
        0x3A => ("LD A,(HL-)".into(), 1),
        0x3B => ("DEC SP".into(), 1),
        0x3C => ("INC A".into(), 1),
        0x3D => ("DEC A".into(), 1),
        0x3E => { let b = read_fn(addr.wrapping_add(1)); (format!("LD A,${:02X}", b), 2) }
        0x3F => ("CCF".into(), 1),

        // LD r,r block 0x40-0x7F
        0x40 => ("LD B,B".into(), 1), 0x41 => ("LD B,C".into(), 1),
        0x42 => ("LD B,D".into(), 1), 0x43 => ("LD B,E".into(), 1),
        0x44 => ("LD B,H".into(), 1), 0x45 => ("LD B,L".into(), 1),
        0x46 => ("LD B,(HL)".into(), 1), 0x47 => ("LD B,A".into(), 1),
        0x48 => ("LD C,B".into(), 1), 0x49 => ("LD C,C".into(), 1),
        0x4A => ("LD C,D".into(), 1), 0x4B => ("LD C,E".into(), 1),
        0x4C => ("LD C,H".into(), 1), 0x4D => ("LD C,L".into(), 1),
        0x4E => ("LD C,(HL)".into(), 1), 0x4F => ("LD C,A".into(), 1),

        0x50 => ("LD D,B".into(), 1), 0x51 => ("LD D,C".into(), 1),
        0x52 => ("LD D,D".into(), 1), 0x53 => ("LD D,E".into(), 1),
        0x54 => ("LD D,H".into(), 1), 0x55 => ("LD D,L".into(), 1),
        0x56 => ("LD D,(HL)".into(), 1), 0x57 => ("LD D,A".into(), 1),
        0x58 => ("LD E,B".into(), 1), 0x59 => ("LD E,C".into(), 1),
        0x5A => ("LD E,D".into(), 1), 0x5B => ("LD E,E".into(), 1),
        0x5C => ("LD E,H".into(), 1), 0x5D => ("LD E,L".into(), 1),
        0x5E => ("LD E,(HL)".into(), 1), 0x5F => ("LD E,A".into(), 1),

        0x60 => ("LD H,B".into(), 1), 0x61 => ("LD H,C".into(), 1),
        0x62 => ("LD H,D".into(), 1), 0x63 => ("LD H,E".into(), 1),
        0x64 => ("LD H,H".into(), 1), 0x65 => ("LD H,L".into(), 1),
        0x66 => ("LD H,(HL)".into(), 1), 0x67 => ("LD H,A".into(), 1),
        0x68 => ("LD L,B".into(), 1), 0x69 => ("LD L,C".into(), 1),
        0x6A => ("LD L,D".into(), 1), 0x6B => ("LD L,E".into(), 1),
        0x6C => ("LD L,H".into(), 1), 0x6D => ("LD L,L".into(), 1),
        0x6E => ("LD L,(HL)".into(), 1), 0x6F => ("LD L,A".into(), 1),

        0x70 => ("LD (HL),B".into(), 1), 0x71 => ("LD (HL),C".into(), 1),
        0x72 => ("LD (HL),D".into(), 1), 0x73 => ("LD (HL),E".into(), 1),
        0x74 => ("LD (HL),H".into(), 1), 0x75 => ("LD (HL),L".into(), 1),
        0x76 => ("HALT".into(), 1),
        0x77 => ("LD (HL),A".into(), 1),
        0x78 => ("LD A,B".into(), 1), 0x79 => ("LD A,C".into(), 1),
        0x7A => ("LD A,D".into(), 1), 0x7B => ("LD A,E".into(), 1),
        0x7C => ("LD A,H".into(), 1), 0x7D => ("LD A,L".into(), 1),
        0x7E => ("LD A,(HL)".into(), 1), 0x7F => ("LD A,A".into(), 1),

        // ALU block 0x80-0xBF
        0x80..=0x87 => (format!("ADD A,{}", alu_reg(opcode & 7)), 1),
        0x88..=0x8F => (format!("ADC A,{}", alu_reg(opcode & 7)), 1),
        0x90..=0x97 => (format!("SUB {}", alu_reg(opcode & 7)), 1),
        0x98..=0x9F => (format!("SBC A,{}", alu_reg(opcode & 7)), 1),
        0xA0..=0xA7 => (format!("AND {}", alu_reg(opcode & 7)), 1),
        0xA8..=0xAF => (format!("XOR {}", alu_reg(opcode & 7)), 1),
        0xB0..=0xB7 => (format!("OR {}", alu_reg(opcode & 7)), 1),
        0xB8..=0xBF => (format!("CP {}", alu_reg(opcode & 7)), 1),

        0xC0 => ("RET NZ".into(), 1),
        0xC1 => ("POP BC".into(), 1),
        0xC2 => { let w = read_word(addr, &read_fn); (format!("JP NZ,${:04X}", w), 3) }
        0xC3 => { let w = read_word(addr, &read_fn); (format!("JP ${:04X}", w), 3) }
        0xC4 => { let w = read_word(addr, &read_fn); (format!("CALL NZ,${:04X}", w), 3) }
        0xC5 => ("PUSH BC".into(), 1),
        0xC6 => { let b = read_fn(addr.wrapping_add(1)); (format!("ADD A,${:02X}", b), 2) }
        0xC7 => ("RST $00".into(), 1),
        0xC8 => ("RET Z".into(), 1),
        0xC9 => ("RET".into(), 1),
        0xCA => { let w = read_word(addr, &read_fn); (format!("JP Z,${:04X}", w), 3) }
        // 0xCB handled above
        0xCC => { let w = read_word(addr, &read_fn); (format!("CALL Z,${:04X}", w), 3) }
        0xCD => { let w = read_word(addr, &read_fn); (format!("CALL ${:04X}", w), 3) }
        0xCE => { let b = read_fn(addr.wrapping_add(1)); (format!("ADC A,${:02X}", b), 2) }
        0xCF => ("RST $08".into(), 1),

        0xD0 => ("RET NC".into(), 1),
        0xD1 => ("POP DE".into(), 1),
        0xD2 => { let w = read_word(addr, &read_fn); (format!("JP NC,${:04X}", w), 3) }
        0xD4 => { let w = read_word(addr, &read_fn); (format!("CALL NC,${:04X}", w), 3) }
        0xD5 => ("PUSH DE".into(), 1),
        0xD6 => { let b = read_fn(addr.wrapping_add(1)); (format!("SUB ${:02X}", b), 2) }
        0xD7 => ("RST $10".into(), 1),
        0xD8 => ("RET C".into(), 1),
        0xD9 => ("RETI".into(), 1),
        0xDA => { let w = read_word(addr, &read_fn); (format!("JP C,${:04X}", w), 3) }
        0xDC => { let w = read_word(addr, &read_fn); (format!("CALL C,${:04X}", w), 3) }
        0xDE => { let b = read_fn(addr.wrapping_add(1)); (format!("SBC A,${:02X}", b), 2) }
        0xDF => ("RST $18".into(), 1),

        0xE0 => { let b = read_fn(addr.wrapping_add(1)); (format!("LDH (${:02X}),A", b), 2) }
        0xE1 => ("POP HL".into(), 1),
        0xE2 => ("LD ($FF00+C),A".into(), 1),
        0xE5 => ("PUSH HL".into(), 1),
        0xE6 => { let b = read_fn(addr.wrapping_add(1)); (format!("AND ${:02X}", b), 2) }
        0xE7 => ("RST $20".into(), 1),
        0xE8 => { let b = read_fn(addr.wrapping_add(1)); (format!("ADD SP,${:02X}", b), 2) }
        0xE9 => ("JP (HL)".into(), 1),
        0xEA => { let w = read_word(addr, &read_fn); (format!("LD (${:04X}),A", w), 3) }
        0xEE => { let b = read_fn(addr.wrapping_add(1)); (format!("XOR ${:02X}", b), 2) }
        0xEF => ("RST $28".into(), 1),

        0xF0 => { let b = read_fn(addr.wrapping_add(1)); (format!("LDH A,(${:02X})", b), 2) }
        0xF1 => ("POP AF".into(), 1),
        0xF2 => ("LD A,($FF00+C)".into(), 1),
        0xF3 => ("DI".into(), 1),
        0xF5 => ("PUSH AF".into(), 1),
        0xF6 => { let b = read_fn(addr.wrapping_add(1)); (format!("OR ${:02X}", b), 2) }
        0xF7 => ("RST $30".into(), 1),
        0xF8 => { let b = read_fn(addr.wrapping_add(1)); (format!("LD HL,SP+${:02X}", b), 2) }
        0xF9 => ("LD SP,HL".into(), 1),
        0xFA => { let w = read_word(addr, &read_fn); (format!("LD A,(${:04X})", w), 3) }
        0xFB => ("EI".into(), 1),
        0xFE => { let b = read_fn(addr.wrapping_add(1)); (format!("CP ${:02X}", b), 2) }
        0xFF => ("RST $38".into(), 1),

        _ => (format!("DB ${:02X}", opcode), 1),
    }
}

fn read_word<F: Fn(u16) -> u8>(addr: u16, read_fn: &F) -> u16 {
    let lo = read_fn(addr.wrapping_add(1)) as u16;
    let hi = read_fn(addr.wrapping_add(2)) as u16;
    (hi << 8) | lo
}

fn alu_reg(r: u8) -> &'static str {
    match r {
        0 => "B", 1 => "C", 2 => "D", 3 => "E",
        4 => "H", 5 => "L", 6 => "(HL)", 7 => "A",
        _ => "?",
    }
}

fn disassemble_cb(byte: u8) -> String {
    let reg = alu_reg(byte & 7);
    match byte >> 3 {
        0 => format!("RLC {}", reg),
        1 => format!("RRC {}", reg),
        2 => format!("RL {}", reg),
        3 => format!("RR {}", reg),
        4 => format!("SLA {}", reg),
        5 => format!("SRA {}", reg),
        6 => format!("SWAP {}", reg),
        7 => format!("SRL {}", reg),
        n @ 8..=15 => format!("BIT {},{}",  n - 8, reg),
        n @ 16..=23 => format!("RES {},{}", n - 16, reg),
        n @ 24..=31 => format!("SET {},{}", n - 24, reg),
        _ => unreachable!(),
    }
}
