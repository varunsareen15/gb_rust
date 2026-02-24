# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Game Boy (LR35902) CPU emulator written in Rust. Currently focused on implementing the full CPU instruction set with test coverage. No external dependencies — uses only the Rust standard library.

## Build Commands

```bash
cargo build                    # Compile
cargo test                     # Run all tests (60 currently)
cargo test <test_name>         # Run a single test, e.g. cargo test test_add_no_overflow
```

## Architecture

```
src/
  main.rs              — Entry point, imports the cpu module
  cpu/
    mod.rs             — CPU struct, step(), execute(), all ALU/rotate/misc helpers
    registers.rs       — Registers, FlagsRegister, 16-bit pair access, From/Into conversions
    instruction.rs     — Instruction enum, target enums, from_byte() decoding (unprefixed + CB-prefixed)
    memory.rs          — MemoryBus (read_byte / write_byte)
    tests.rs           — All tests (#[cfg(test)])
```

### Key structures

- **`CPU`** — Holds registers, program counter (`pc`), stack pointer (`sp`), and a `MemoryBus`.
- **`Registers`** — Eight 8-bit registers (`a`–`l`, `f`) with helper methods for 16-bit pair access (`get_bc`/`set_bc`, `get_de`/`set_de`, `get_hl`/`set_hl`, `get_af`/`set_af`).
- **`FlagsRegister`** — Individual booleans (`zero`, `subtract`, `half_carry`, `carry`) with `From<u8>` / `Into<u8>` conversions.
- **`MemoryBus`** — Wraps a `[u8; 0xFFFF]` array with `read_byte`/`write_byte`.
- **`Instruction`** enum — Decoded instructions with operand info embedded in variants. Decoded via `Instruction::from_byte()` (main opcodes) and `from_byte_prefixed()` (0xCB-prefixed opcodes).

### Target enums

- **`ByteTarget`** — Unified enum (A, B, C, D, E, H, L, HL, Imm8) used by ADD, ADC, SUB, SBC, AND, OR, XOR, CP.
- **`PrefixTarget`** — (A, B, C, D, E, H, L, HL) used by all CB-prefixed instructions.
- **`IncDecTarget`** — 8-bit registers + 16-bit pairs + (HL) for INC/DEC.

### Execution flow

`CPU::step()` → reads opcode at PC → decodes via `Instruction::from_byte()` → `CPU::execute()` dispatches on the instruction enum → `resolve_byte_target()` maps target to value (eliminating per-register duplication) → ALU helpers update registers and flags → returns new PC value.

## Current State

- **Arithmetic/logic**: ADD, ADC, SUB, SBC, AND, OR, XOR, CP — fully implemented with `resolve_byte_target()` helper.
- **INC/DEC**: 8-bit and 16-bit variants including (HL).
- **Accumulator rotates**: RLCA, RRCA, RLA, RRA (unprefixed, always clear zero flag).
- **Misc**: DAA, CPL, SCF, CCF, HALT, STOP, RST.
- **CB-prefixed**: All 256 opcodes decoded and implemented — RLC, RRC, RL, RR, SLA, SRA, SWAP, SRL, BIT, RES, SET.
- **Control flow**: JP, JR, CALL, RET, RETI, PUSH, POP, ADDSP, LDHL.
- **Loads**: LD byte and word variants.
- **Interrupts**: DI/EI decoded but not yet functional (TODO stubs).

## Testing

Tests are in `src/cpu/tests.rs` (included via `#[cfg(test)] mod tests`). Each test constructs a `CPU::default()`, sets up register state, calls the operation method directly, and asserts register values and flags. Tests cover edge cases like half-carry, overflow, zero results, carry-in behavior, sign preservation, and BCD correction.
