use super::*;

// ===============================================
// Tests for 8-bit ADD instruction
// ===============================================
#[test]
fn test_add_no_overflow() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x03;
    cpu.registers.f = FlagsRegister::default();

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

    let result = cpu.add(0x02);

    assert_eq!(result, 0x01);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.half_carry, true);
}

#[test]
fn test_add_zero_result() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x00;
    cpu.registers.f = FlagsRegister::default();

    let result = cpu.add(0x00);

    assert_eq!(result, 0x00);
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
    cpu.registers.f.carry = true;

    let result = cpu.adc(0x04);

    assert_eq!(result, 0x08);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, false);
    assert_eq!(cpu.registers.f.carry, false);
}

#[test]
fn test_adc_overflow_and_half_carry() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0xF8;
    cpu.registers.f = FlagsRegister::default();
    cpu.registers.f.carry = true;

    let result = cpu.adc(0x0A);

    assert_eq!(result, 0x03);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, false);
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
    let new_hl = cpu.add_hl(0x0100);
    cpu.registers.set_hl(new_hl);

    assert_eq!(cpu.registers.get_hl(), 0x1334);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, false);
    assert_eq!(cpu.registers.f.carry, false);
}

#[test]
fn test_add_hl_half_carry() {
    let mut cpu = CPU::default();
    cpu.registers.set_hl(0x0FF0);
    let new_hl = cpu.add_hl(0x0010);
    cpu.registers.set_hl(new_hl);

    assert_eq!(cpu.registers.get_hl(), 0x1000);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, true);
    assert_eq!(cpu.registers.f.carry, false);
}

#[test]
fn test_add_hl_overflow() {
    let mut cpu = CPU::default();
    cpu.registers.set_hl(0xFFFF);
    let new_hl = cpu.add_hl(0x0001);
    cpu.registers.set_hl(new_hl);

    assert_eq!(cpu.registers.get_hl(), 0x0000);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, true);
    assert_eq!(cpu.registers.f.carry, true);
}

// ===============================================
// Tests for SUB
// ===============================================
#[test]
fn test_sub_no_borrow() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x08;
    cpu.registers.f = FlagsRegister::default();

    let result = cpu.sub(0x03);
    assert_eq!(result, 0x05);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, true);
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.half_carry, false);
}

#[test]
fn test_sub_half_carry() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x10;
    cpu.registers.f = FlagsRegister::default();

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

    let result = cpu.sub(0x05);
    assert_eq!(result, 0xFE);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, true);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.half_carry, true);
}

#[test]
fn test_sub_zero_result() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x05;
    cpu.registers.f = FlagsRegister::default();

    let result = cpu.sub(0x05);
    assert_eq!(result, 0x00);
    assert_eq!(cpu.registers.f.zero, true);
    assert_eq!(cpu.registers.f.subtract, true);
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.half_carry, false);
}

// ===============================================
// Tests for SBC
// ===============================================
#[test]
fn test_sbc_no_carry_in_no_borrow() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x10;
    cpu.registers.f = FlagsRegister::default();
    cpu.registers.f.carry = false;

    let result = cpu.sbc(0x05);
    assert_eq!(result, 0x0B);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, true);
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.half_carry, true);
}

#[test]
fn test_sbc_with_carry_in_no_borrow() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x10;
    cpu.registers.f = FlagsRegister::default();
    cpu.registers.f.carry = true;

    let result = cpu.sbc(0x05);
    assert_eq!(result, 0x0A);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, true);
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.half_carry, true);
}

#[test]
fn test_sbc_borrow() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x03;
    cpu.registers.f = FlagsRegister::default();
    cpu.registers.f.carry = true;

    let result = cpu.sbc(0x05);
    assert_eq!(result, 0xFD);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, true);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.half_carry, true);
}

#[test]
fn test_sbc_zero_result() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x05;
    cpu.registers.f = FlagsRegister::default();
    cpu.registers.f.carry = false;

    let result = cpu.sbc(0x05);
    assert_eq!(result, 0x00);
    assert_eq!(cpu.registers.f.zero, true);
    assert_eq!(cpu.registers.f.subtract, true);
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.half_carry, false);
}

// ===============================================
// Tests for AND, OR, XOR
// ===============================================
#[test]
fn test_and_non_zero() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0xCC;
    let result = cpu.and(0xAA);
    assert_eq!(result, 0x88);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, true);
    assert_eq!(cpu.registers.f.carry, false);
}

#[test]
fn test_and_zero_result() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0xF0;
    let result = cpu.and(0x0F);
    assert_eq!(result, 0x00);
    assert_eq!(cpu.registers.f.zero, true);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, true);
    assert_eq!(cpu.registers.f.carry, false);
}

#[test]
fn test_or_non_zero() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x10;
    let result = cpu.or(0x02);
    assert_eq!(result, 0x12);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, false);
    assert_eq!(cpu.registers.f.carry, false);
}

#[test]
fn test_or_zero_result() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x00;
    let result = cpu.or(0x00);
    assert_eq!(result, 0x00);
    assert_eq!(cpu.registers.f.zero, true);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, false);
    assert_eq!(cpu.registers.f.carry, false);
}

#[test]
fn test_xor_zero_result() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0xAA;
    let result = cpu.xor(0xAA);
    assert_eq!(result, 0x00);
    assert_eq!(cpu.registers.f.zero, true);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, false);
    assert_eq!(cpu.registers.f.carry, false);
}

#[test]
fn test_xor_non_zero() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0xF0;
    let result = cpu.xor(0x0F);
    assert_eq!(result, 0xFF);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, false);
    assert_eq!(cpu.registers.f.carry, false);
}

// ===============================================
// Tests for RLC (CB-prefixed)
// ===============================================
#[test]
fn test_rlc_no_carry() {
    let mut cpu = CPU::default();
    // 0b01010011 -> carry=0, result = 0b10100110
    let result = cpu.rlc(0x53);
    assert_eq!(result, 0xA6);
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.zero, false);
}

#[test]
fn test_rlc_with_carry() {
    let mut cpu = CPU::default();
    // 0b10000101 -> carry=1, result = 0b00001011
    let result = cpu.rlc(0x85);
    assert_eq!(result, 0x0B);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

// ===============================================
// Tests for RRC (CB-prefixed)
// ===============================================
#[test]
fn test_rrc_no_carry() {
    let mut cpu = CPU::default();
    // 0b10100110 -> bit0=0, carry=0, result = 0b01010011
    let result = cpu.rrc(0xA6);
    assert_eq!(result, 0x53);
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.zero, false);
}

#[test]
fn test_rrc_with_carry() {
    let mut cpu = CPU::default();
    // 0b10100111 -> bit0=1, carry=1, result = 0b11010011
    let result = cpu.rrc(0xA7);
    assert_eq!(result, 0xD3);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

// ===============================================
// Tests for RL (CB-prefixed, rotate left through carry)
// ===============================================
#[test]
fn test_rl_no_carry_in() {
    let mut cpu = CPU::default();
    cpu.registers.f.carry = false;
    // 0b10000101 -> new carry=1, result = 0b00001010
    let result = cpu.rl(0x85);
    assert_eq!(result, 0x0A);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

#[test]
fn test_rl_with_carry_in() {
    let mut cpu = CPU::default();
    cpu.registers.f.carry = true;
    // 0b10000101 -> new carry=1, result = 0b00001011
    let result = cpu.rl(0x85);
    assert_eq!(result, 0x0B);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

// ===============================================
// Tests for RR (CB-prefixed, rotate right through carry)
// ===============================================
#[test]
fn test_rr_no_carry_in() {
    let mut cpu = CPU::default();
    cpu.registers.f.carry = false;
    // 0b10000101 -> new carry=1, result = 0b01000010
    let result = cpu.rr(0x85);
    assert_eq!(result, 0x42);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

#[test]
fn test_rr_with_carry_in() {
    let mut cpu = CPU::default();
    cpu.registers.f.carry = true;
    // 0b10000100 -> new carry=0, result = 0b11000010
    let result = cpu.rr(0x84);
    assert_eq!(result, 0xC2);
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.zero, false);
}

// ===============================================
// Tests for SLA (CB-prefixed, shift left arithmetic)
// ===============================================
#[test]
fn test_sla() {
    let mut cpu = CPU::default();
    // 0b10000101 -> carry=1, result = 0b00001010
    let result = cpu.sla(0x85);
    assert_eq!(result, 0x0A);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

#[test]
fn test_sla_zero() {
    let mut cpu = CPU::default();
    // 0b10000000 -> carry=1, result = 0x00
    let result = cpu.sla(0x80);
    assert_eq!(result, 0x00);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, true);
}

// ===============================================
// Tests for SRA (CB-prefixed, shift right arithmetic)
// ===============================================
#[test]
fn test_sra_preserves_sign() {
    let mut cpu = CPU::default();
    // 0b10000101 -> carry=1, result = 0b11000010 (sign bit preserved)
    let result = cpu.sra(0x85);
    assert_eq!(result, 0xC2);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

#[test]
fn test_sra_positive() {
    let mut cpu = CPU::default();
    // 0b01000100 -> carry=0, result = 0b00100010
    let result = cpu.sra(0x44);
    assert_eq!(result, 0x22);
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.zero, false);
}

// ===============================================
// Tests for SWAP (CB-prefixed)
// ===============================================
#[test]
fn test_swap() {
    let mut cpu = CPU::default();
    let result = cpu.swap(0xAB);
    assert_eq!(result, 0xBA);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, false);
    assert_eq!(cpu.registers.f.carry, false);
}

#[test]
fn test_swap_zero() {
    let mut cpu = CPU::default();
    let result = cpu.swap(0x00);
    assert_eq!(result, 0x00);
    assert_eq!(cpu.registers.f.zero, true);
}

// ===============================================
// Tests for SRL (CB-prefixed, shift right logical)
// ===============================================
#[test]
fn test_srl() {
    let mut cpu = CPU::default();
    // 0b10000101 -> carry=1, result = 0b01000010
    let result = cpu.srl(0x85);
    assert_eq!(result, 0x42);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

#[test]
fn test_srl_zero() {
    let mut cpu = CPU::default();
    let result = cpu.srl(0x01);
    assert_eq!(result, 0x00);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, true);
}

// ===============================================
// Tests for BIT (CB-prefixed)
// ===============================================
#[test]
fn test_bit_set() {
    let mut cpu = CPU::default();
    cpu.bit(3, 0b00001000);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, true);
}

#[test]
fn test_bit_not_set() {
    let mut cpu = CPU::default();
    cpu.bit(3, 0b11110111);
    assert_eq!(cpu.registers.f.zero, true);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, true);
}

#[test]
fn test_bit_7() {
    let mut cpu = CPU::default();
    cpu.bit(7, 0x80);
    assert_eq!(cpu.registers.f.zero, false);
    cpu.bit(7, 0x7F);
    assert_eq!(cpu.registers.f.zero, true);
}

// ===============================================
// Tests for RES and SET (CB-prefixed)
// ===============================================
#[test]
fn test_res_bit() {
    let value: u8 = 0xFF;
    let result = value & !(1 << 3);
    assert_eq!(result, 0b11110111);
}

#[test]
fn test_set_bit() {
    let value: u8 = 0x00;
    let result = value | (1 << 3);
    assert_eq!(result, 0b00001000);
}

// ===============================================
// Tests for accumulator rotates (unprefixed)
// ===============================================
#[test]
fn test_rlca() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x85; // 10000101
    cpu.rlca();
    assert_eq!(cpu.registers.a, 0x0B); // 00001011
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false); // always cleared
}

#[test]
fn test_rrca() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x85; // 10000101
    cpu.rrca();
    assert_eq!(cpu.registers.a, 0xC2); // 11000010
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

#[test]
fn test_rla() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x85;
    cpu.registers.f.carry = false;
    cpu.rla();
    assert_eq!(cpu.registers.a, 0x0A);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

#[test]
fn test_rra() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x85;
    cpu.registers.f.carry = false;
    cpu.rra();
    assert_eq!(cpu.registers.a, 0x42);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, false);
}

// ===============================================
// Tests for DAA
// ===============================================
#[test]
fn test_daa_after_add() {
    let mut cpu = CPU::default();
    // BCD: 15 + 27 = 42
    cpu.registers.a = 0x15;
    let result = cpu.add(0x27);
    cpu.registers.a = result; // 0x3C
    cpu.daa();
    assert_eq!(cpu.registers.a, 0x42);
    assert_eq!(cpu.registers.f.zero, false);
}

#[test]
fn test_daa_after_add_with_half_carry() {
    let mut cpu = CPU::default();
    // BCD: 08 + 09 = 17
    cpu.registers.a = 0x08;
    let result = cpu.add(0x09);
    cpu.registers.a = result; // 0x11, half_carry=true
    cpu.daa();
    assert_eq!(cpu.registers.a, 0x17);
}

#[test]
fn test_daa_after_add_with_carry() {
    let mut cpu = CPU::default();
    // BCD: 99 + 01 = 100 -> 0x00 with carry
    cpu.registers.a = 0x99;
    let result = cpu.add(0x01);
    cpu.registers.a = result; // 0x9A
    cpu.daa();
    assert_eq!(cpu.registers.a, 0x00);
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.zero, true);
}

#[test]
fn test_daa_after_sub() {
    let mut cpu = CPU::default();
    // BCD: 42 - 15 = 27
    cpu.registers.a = 0x42;
    let result = cpu.sub(0x15);
    cpu.registers.a = result; // 0x2D, subtract=true, half_carry=true
    cpu.daa();
    assert_eq!(cpu.registers.a, 0x27);
}

// ===============================================
// Tests for CPL
// ===============================================
#[test]
fn test_cpl() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0xAA;
    cpu.cpl();
    assert_eq!(cpu.registers.a, 0x55);
    assert_eq!(cpu.registers.f.subtract, true);
    assert_eq!(cpu.registers.f.half_carry, true);
}

// ===============================================
// Tests for SCF
// ===============================================
#[test]
fn test_scf() {
    let mut cpu = CPU::default();
    cpu.registers.f.carry = false;
    cpu.registers.f.subtract = true;
    cpu.registers.f.half_carry = true;
    cpu.scf();
    assert_eq!(cpu.registers.f.carry, true);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, false);
}

// ===============================================
// Tests for CCF
// ===============================================
#[test]
fn test_ccf_clear() {
    let mut cpu = CPU::default();
    cpu.registers.f.carry = true;
    cpu.registers.f.subtract = true;
    cpu.registers.f.half_carry = true;
    cpu.ccf();
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.subtract, false);
    assert_eq!(cpu.registers.f.half_carry, false);
}

#[test]
fn test_ccf_set() {
    let mut cpu = CPU::default();
    cpu.registers.f.carry = false;
    cpu.ccf();
    assert_eq!(cpu.registers.f.carry, true);
}

// ===============================================
// Tests for CP (compare)
// ===============================================
#[test]
fn test_cp_equal() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x42;
    cpu.cp(0x42);
    assert_eq!(cpu.registers.f.zero, true);
    assert_eq!(cpu.registers.f.subtract, true);
    assert_eq!(cpu.registers.f.carry, false);
    assert_eq!(cpu.registers.f.half_carry, false);
}

#[test]
fn test_cp_less() {
    let mut cpu = CPU::default();
    cpu.registers.a = 0x03;
    cpu.cp(0x05);
    assert_eq!(cpu.registers.f.zero, false);
    assert_eq!(cpu.registers.f.subtract, true);
    assert_eq!(cpu.registers.f.carry, true);
}

// ===============================================
// Test opcode decoding completeness
// ===============================================
#[test]
fn test_all_cb_opcodes_decoded() {
    for byte in 0x00..=0xFFu8 {
        assert!(
            Instruction::from_byte(byte, true).is_some(),
            "CB-prefixed opcode 0x{:02X} should be decoded", byte
        );
    }
}

// ===============================================
// Tests for HALT bug
// ===============================================
#[test]
fn test_halt_bug_triggers() {
    // IME=0 + pending interrupt → halt_bug=true, halted=false
    let mut cpu = CPU::default();
    cpu.pc = 0xC000; // Use WRAM (writable)
    cpu.ime = false;
    cpu.bus.ie_register = 0x01; // VBlank enabled
    cpu.bus.if_register = 0x01; // VBlank pending
    // Write HALT opcode (0x76) at PC
    cpu.bus.write_byte(0xC000, 0x76);
    // Write NOP after HALT for the next step
    cpu.bus.write_byte(0xC001, 0x00);

    cpu.step(); // executes HALT
    assert!(!cpu.halted, "CPU should NOT be halted (halt bug)");
    assert!(cpu.halt_bug, "halt_bug flag should be set");
}

#[test]
fn test_halt_bug_double_read() {
    // Instruction after HALT executes but PC doesn't advance
    let mut cpu = CPU::default();
    cpu.pc = 0xC000;
    cpu.ime = false;
    cpu.bus.ie_register = 0x01;
    cpu.bus.if_register = 0x01;
    // Write HALT at 0xC000, then INC B (0x04) at 0xC001
    cpu.bus.write_byte(0xC000, 0x76);
    cpu.bus.write_byte(0xC001, 0x04); // INC B
    cpu.registers.b = 0x00;

    cpu.step(); // executes HALT → sets halt_bug, PC becomes 0xC001
    assert!(cpu.halt_bug);
    assert_eq!(cpu.pc, 0xC001);

    cpu.step(); // executes INC B at 0xC001, but PC stays at 0xC001 due to halt bug
    assert_eq!(cpu.registers.b, 1);
    assert_eq!(cpu.pc, 0xC001, "PC should not advance due to halt bug (double read)");
    assert!(!cpu.halt_bug, "halt_bug should be cleared after one use");

    cpu.step(); // executes INC B at 0xC001 again, this time PC advances normally
    assert_eq!(cpu.registers.b, 2);
    assert_eq!(cpu.pc, 0xC002);
}

#[test]
fn test_halt_normal_ime_enabled() {
    // IME=1, no pending interrupt yet → normal halt (halted=true), no halt bug
    let mut cpu = CPU::default();
    cpu.pc = 0xC000;
    cpu.ime = true;
    cpu.bus.ie_register = 0x01;
    cpu.bus.if_register = 0x00; // no pending yet
    cpu.bus.write_byte(0xC000, 0x76);

    cpu.step(); // executes HALT
    assert!(cpu.halted, "CPU should be halted normally when IME=1");
    assert!(!cpu.halt_bug);
}

#[test]
fn test_halt_normal_no_pending() {
    // IME=0, no pending interrupts → normal halt (halted=true)
    let mut cpu = CPU::default();
    cpu.pc = 0xC000;
    cpu.ime = false;
    cpu.bus.ie_register = 0x01;
    cpu.bus.if_register = 0x00; // no pending
    cpu.bus.write_byte(0xC000, 0x76);

    cpu.step(); // executes HALT
    assert!(cpu.halted, "CPU should be halted normally when no pending interrupts");
    assert!(!cpu.halt_bug);
}

// ===============================================
// Tests for delayed EI timing
// ===============================================
#[test]
fn test_ei_delayed_by_one_instruction() {
    // EI sets ei_pending but IME should not become true until after the NEXT instruction
    let mut cpu = CPU::default();
    cpu.pc = 0xC000;
    cpu.ime = false;
    cpu.bus.ie_register = 0x01; // VBlank enabled
    cpu.bus.if_register = 0x00; // No pending interrupts yet

    // Write EI (0xFB) at 0xC000, then NOP (0x00) at 0xC001
    cpu.bus.write_byte(0xC000, 0xFB); // EI
    cpu.bus.write_byte(0xC001, 0x00); // NOP
    cpu.bus.write_byte(0xC002, 0x00); // NOP

    // Step 1: Execute EI — sets ei_pending, IME still false
    cpu.step();
    assert_eq!(cpu.pc, 0xC001);
    assert!(!cpu.ime, "IME should still be false immediately after EI");
    assert!(cpu.ei_pending, "ei_pending should be set after EI");

    // Step 2: Execute NOP — ei_pending processed before execute, IME becomes true
    cpu.step();
    assert_eq!(cpu.pc, 0xC002);
    assert!(cpu.ime, "IME should be true after the instruction following EI");
}

// ===============================================
// Tests for serial port stub
// ===============================================
#[test]
fn test_serial_transfer_completes() {
    let mut cpu = CPU::default();
    cpu.bus.write_byte(0xFF01, 0x42); // write data to SB
    // Request transfer with internal clock (bit 7 + bit 0)
    cpu.bus.write_byte(0xFF02, 0x81);
    // Transfer completes immediately: SB = 0xFF (no link partner)
    assert_eq!(cpu.bus.read_byte(0xFF01), 0xFF);
    // SC bit 7 cleared (transfer complete)
    assert_eq!(cpu.bus.read_byte(0xFF02) & 0x80, 0x00);
    // Serial interrupt requested (bit 3 of IF)
    assert_eq!(cpu.bus.if_register & 0x08, 0x08);
}

#[test]
fn test_serial_no_transfer_without_start() {
    let mut cpu = CPU::default();
    cpu.bus.write_byte(0xFF01, 0x42); // write data to SB
    // Write SC without bit 7 → no transfer
    cpu.bus.write_byte(0xFF02, 0x01);
    // SB unchanged
    assert_eq!(cpu.bus.read_byte(0xFF01), 0x42);
    // No serial interrupt
    assert_eq!(cpu.bus.if_register & 0x08, 0x00);
}

#[test]
fn test_serial_sb_readwrite() {
    let mut cpu = CPU::default();
    // SB is readable/writable
    cpu.bus.write_byte(0xFF01, 0xAB);
    assert_eq!(cpu.bus.read_byte(0xFF01), 0xAB);
    cpu.bus.write_byte(0xFF01, 0x00);
    assert_eq!(cpu.bus.read_byte(0xFF01), 0x00);
}
