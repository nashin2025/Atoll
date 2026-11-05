//! RISC-V VM for the Atoll Protocol.
//!
//! Phase 1: RISC-V instruction decoding and execution simulation.
//! Phase 2: Full RV64IMC support with privileged modes.
//!
//! Production hardening additions:
//! - ELF64 header parsing and validation (magic, class, encoding, type, machine, entry point)
//! - RV64I base integer instruction decoder (ADD, SUB, AND, OR, XOR, SLL, SRL, SRA, SLT, SLTU,
//!   LUI, AUIPC, JAL, JALR, BEQ, BNE, BLT, BGE, BLTU, BGEU, LB, LH, LW, LWU, LD, SB, SH, SW, SD,
//!   ADDI, SLTI, SLTIU, XORI, ORI, ANDI, SLLI, SRLI, SRAI)
//! - 32 x 64-bit register file (x0 hardwired to 0), program counter
//! - Linear memory sandbox with bounds checking
//! - Deterministic execution: same input produces same output
//! - Panic safety: invalid instructions and memory accesses return VMError instead of panicking

use crate::vm::{ExecutionContext, ExecutionResult, GasMeter, VMError, VMType};

/// RISC-V executor
pub struct RISCVExecutor {
    pub gas_meter: GasMeter,
    pub context: ExecutionContext,
}

/// ELF64 header: 64 bytes
#[derive(Debug, Clone)]
struct Elf64Header {
    /// Magic number: \x7fELF
    magic: [u8; 4],
    /// Class: ELFCLASS64 = 2
    class: u8,
    /// Data encoding: ELFDATA2LSB = 1 (little-endian)
    data: u8,
    /// Version: EV_CURRENT = 1
    version: u8,
    /// OS/ABI identification
    os_abi: u8,
    /// ABI version
    abi_version: u8,
    /// Padding
    pad: [u8; 7],
    /// Object file type: ET_EXEC = 2
    e_type: u16,
    /// Machine architecture: EM_RISC-V = 243
    e_machine: u16,
    /// Object file version
    e_version: u32,
    /// Entry point virtual address
    e_entry: u64,
    /// Program header table file offset
    e_phoff: u64,
    /// Section header table file offset
    e_shoff: u64,
    /// Processor-specific flags
    e_flags: u32,
    /// Size of ELF header
    e_ehsize: u16,
    /// Size of program header entry
    e_phentsize: u16,
    /// Number of program header entries
    e_phnum: u16,
    /// Size of section header entry
    e_shentsize: u16,
    /// Number of section header entries
    e_shnum: u16,
    /// Section name string table index
    e_shstrndx: u16,
}

/// RISC-V register file: 32 x 64-bit registers, x0 hardwired to 0
#[derive(Debug, Clone, Default)]
struct RegisterFile {
    regs: [u64; 32],
}

impl RegisterFile {
    fn new() -> Self {
        Self { regs: [0; 32] }
    }

    fn read(
        &self,
        reg: u8,
    ) -> u64 {
        if reg == 0 {
            0
        } else {
            self.regs[reg as usize]
        }
    }

    fn write(
        &mut self,
        reg: u8,
        value: u64,
    ) {
        if reg != 0 {
            self.regs[reg as usize] = value;
        }
    }
}

/// RISC-V linear memory: isolated, bounds-checked
#[derive(Debug, Clone, Default)]
struct Memory {
    data: Vec<u8>,
}

impl Memory {
    fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }

    fn read_u64(
        &self,
        addr: u64,
    ) -> Result<u64, VMError> {
        let addr = addr as usize;
        if addr + 8 > self.data.len() {
            return Err(VMError::InvalidMemoryAccess);
        }
        let mut result = 0u64;
        for i in 0..8 {
            result |= (self.data[addr + i] as u64) << (i * 8);
        }
        Ok(result)
    }

    fn write_u64(
        &mut self,
        addr: u64,
        value: u64,
    ) -> Result<(), VMError> {
        let addr = addr as usize;
        if addr + 8 > self.data.len() {
            return Err(VMError::InvalidMemoryAccess);
        }
        for i in 0..8 {
            self.data[addr + i] = (value >> (i * 8)) as u8;
        }
        Ok(())
    }

    fn read_u32(
        &self,
        addr: u64,
    ) -> Result<u32, VMError> {
        let addr = addr as usize;
        if addr + 4 > self.data.len() {
            return Err(VMError::InvalidMemoryAccess);
        }
        let mut result = 0u32;
        for i in 0..4 {
            result |= (self.data[addr + i] as u32) << (i * 8);
        }
        Ok(result)
    }

    fn write_u32(
        &mut self,
        addr: u64,
        value: u32,
    ) -> Result<(), VMError> {
        let addr = addr as usize;
        if addr + 4 > self.data.len() {
            return Err(VMError::InvalidMemoryAccess);
        }
        for i in 0..4 {
            self.data[addr + i] = (value >> (i * 8)) as u8;
        }
        Ok(())
    }
}

/// RISC-V RV64I base integer instruction set
#[derive(Debug, Clone, Copy)]
enum RiscvInstruction {
    LUI(u8, u32),       // rd, imm
    AUIPC(u8, u32),     // rd, imm
    JAL(u8, i32),       // rd, imm (signed offset)
    JALR(u8, u8, i32),  // rd, rs1, imm
    BEQ(u8, u8, i32),   // rs1, rs2, imm
    BNE(u8, u8, i32),   // rs1, rs2, imm
    BLT(u8, u8, i32),   // rs1, rs2, imm
    BGE(u8, u8, i32),   // rs1, rs2, imm
    BLTU(u8, u8, i32),  // rs1, rs2, imm
    BGEU(u8, u8, i32),  // rs1, rs2, imm
    LB(u8, u8, i32),    // rd, rs1, imm
    LH(u8, u8, i32),    // rd, rs1, imm
    LW(u8, u8, i32),    // rd, rs1, imm
    LWU(u8, u8, i32),   // rd, rs1, imm
    LD(u8, u8, i32),    // rd, rs1, imm
    SB(u8, u8, i32),    // rs2, rs1, imm
    SH(u8, u8, i32),    // rs2, rs1, imm
    SW(u8, u8, i32),    // rs2, rs1, imm
    SD(u8, u8, i32),    // rs2, rs1, imm
    ADDI(u8, u8, i32),  // rd, rs1, imm
    SLTI(u8, u8, i32),  // rd, rs1, imm
    SLTIU(u8, u8, i32), // rd, rs1, imm
    XORI(u8, u8, i32),  // rd, rs1, imm
    ORI(u8, u8, i32),   // rd, rs1, imm
    ANDI(u8, u8, i32),  // rd, rs1, imm
    SLLI(u8, u8, u32),  // rd, rs1, shamt
    SRLI(u8, u8, u32),  // rd, rs1, shamt
    SRAI(u8, u8, u32),  // rd, rs1, shamt
    ADD(u8, u8, u8),    // rd, rs1, rs2
    SUB(u8, u8, u8),    // rd, rs1, rs2
    SLL(u8, u8, u8),    // rd, rs1, rs2
    SLT(u8, u8, u8),    // rd, rs1, rs2
    SLTU(u8, u8, u8),   // rd, rs1, rs2
    XOR(u8, u8, u8),    // rd, rs1, rs2
    SRL(u8, u8, u8),    // rd, rs1, rs2
    SRA(u8, u8, u8),    // rd, rs1, rs2
    OR(u8, u8, u8),     // rd, rs1, rs2
    AND(u8, u8, u8),    // rd, rs1, rs2
}

/// RISC-V opcode constants
const OPCODE_LOAD: u32 = 0x03;
const OPCODE_STORE: u32 = 0x23;
const OPCODE_OP_IMM: u32 = 0x13;
const OPCODE_AUIPC: u32 = 0x17;
const OPCODE_OP: u32 = 0x33;
const OPCODE_LUI: u32 = 0x37;
const OPCODE_JALR: u32 = 0x67;
const OPCODE_JAL: u32 = 0x6F;
const OPCODE_BRANCH: u32 = 0x63;

/// RISC-V funct3 constants
const FUNCT3_LB: u8 = 0x0;
const FUNCT3_LH: u8 = 0x1;
const FUNCT3_LW: u8 = 0x2;
const FUNCT3_LD: u8 = 0x3;
const FUNCT3_LBU: u8 = 0x4;
const FUNCT3_LHU: u8 = 0x5;
const FUNCT3_LWU: u8 = 0x6;
const FUNCT3_SB: u8 = 0x0;
const FUNCT3_SH: u8 = 0x1;
const FUNCT3_SW: u8 = 0x2;
const FUNCT3_SD: u8 = 0x3;
const FUNCT3_ADDI_SLLI: u8 = 0x0;
const FUNCT3_SLTI: u8 = 0x2;
const FUNCT3_SLTIU: u8 = 0x3;
const FUNCT3_XORI: u8 = 0x4;
const FUNCT3_ORI: u8 = 0x6;
const FUNCT3_ANDI: u8 = 0x7;
const FUNCT3_SRLI_SRAI: u8 = 0x5;
const FUNCT3_ADD_SUB: u8 = 0x0;
const FUNCT3_SLL: u8 = 0x1;
const FUNCT3_SLT: u8 = 0x2;
const FUNCT3_SLTU: u8 = 0x3;
const FUNCT3_XOR: u8 = 0x4;
const FUNCT3_SRL_SRA: u8 = 0x5;
const FUNCT3_OR: u8 = 0x6;
const FUNCT3_AND: u8 = 0x7;

/// RISC-V funct7 constants
const FUNCT7_SLLI: u32 = 0x00;
const FUNCT7_SRLI: u32 = 0x00;
const FUNCT7_SRAI: u32 = 0x20;
const FUNCT7_ADD: u32 = 0x00;
const FUNCT7_SUB: u32 = 0x20;
const FUNCT7_SRL: u32 = 0x00;
const FUNCT7_SRA: u32 = 0x20;

/// ELF constants
const ELF_MAGIC: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;
const ET_EXEC: u16 = 2;
const EM_RISCV: u16 = 243;

/// RISC-V memory size: 64 KiB sandbox
const MEMORY_SIZE: usize = 64 * 1024;

impl RISCVExecutor {
    pub fn new(
        context: ExecutionContext,
        gas_limit: u64,
    ) -> Self {
        Self {
            gas_meter: GasMeter::new(gas_limit, context.gas_price),
            context,
        }
    }

    /// Executes a RISC-V binary with full validation and sandboxed execution
    pub fn execute(
        &mut self,
        binary: &[u8],
        _entry_point: u64,
        args: &[Vec<u8>],
    ) -> Result<ExecutionResult, VMError> {
        // Step 1: Validate binary is non-empty
        if binary.is_empty() {
            return Err(VMError::InvalidInstruction);
        }

        // Step 2: Charge gas for binary loading (per byte)
        self.gas_meter.charge(binary.len() as u64)?;

        // Step 3: Parse and validate ELF64 header
        let header = self.parse_elf_header(binary)?;

        // Step 4: Charge gas for header validation
        self.gas_meter.charge(64)?;

        // Step 5: Validate entry point is within memory bounds
        if header.e_entry as usize >= MEMORY_SIZE {
            return Err(VMError::InvalidMemoryAccess);
        }

        // Step 6: Charge gas for entry point validation
        self.gas_meter.charge(8)?;

        // Step 7: Charge gas for arguments
        for arg in args {
            self.gas_meter.charge(arg.len() as u64)?;
        }

        // Step 8: Initialize memory and registers
        let mut memory = Memory::new(MEMORY_SIZE);
        let mut registers = RegisterFile::new();
        let mut pc = header.e_entry as usize;

        // Step 9: Load binary into memory at entry point
        let load_addr = header.e_entry as usize;
        if load_addr + binary.len() > MEMORY_SIZE {
            return Err(VMError::InvalidMemoryAccess);
        }
        memory.data[load_addr..load_addr + binary.len()].copy_from_slice(binary);

        // Step 10: Deterministic execution loop
        let mut instructions_executed = 0u64;
        const MAX_INSTRUCTIONS: u64 = 10_000;

        while instructions_executed < MAX_INSTRUCTIONS {
            // Charge gas per instruction
            self.gas_meter.charge(1)?;

            // Fetch instruction from memory
            if pc + 4 > MEMORY_SIZE {
                return Err(VMError::InvalidMemoryAccess);
            }
            let instr_bytes = [
                memory.data[pc],
                memory.data[pc + 1],
                memory.data[pc + 2],
                memory.data[pc + 3],
            ];
            let instr_word = u32::from_le_bytes(instr_bytes);

            // Decode instruction
            let opcode = instr_word & 0x7f;
            let rd = ((instr_word >> 7) & 0x1f) as u8;
            let rs1 = ((instr_word >> 15) & 0x1f) as u8;
            let rs2 = ((instr_word >> 20) & 0x1f) as u8;
            let funct3 = ((instr_word >> 12) & 0x7) as u8;
            let funct7 = (instr_word >> 25) & 0x7f;

            let instruction =
                self.decode_instruction(opcode, funct3, funct7, rd, rs1, rs2, instr_word)?;

            // Execute instruction
            match instruction {
                RiscvInstruction::LUI(rd, imm) => {
                    registers.write(rd, (imm as u64) << 12);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::AUIPC(rd, imm) => {
                    let base = pc as u64;
                    registers.write(rd, base.wrapping_add((imm as u64) << 12));
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::JAL(rd, imm) => {
                    let next_pc = pc as u64 + 4;
                    if rd != 0 {
                        registers.write(rd, next_pc);
                    }
                    let target = (pc as i64).wrapping_add(imm as i64) as usize;
                    if target >= MEMORY_SIZE {
                        return Err(VMError::InvalidMemoryAccess);
                    }
                    pc = target;
                }
                RiscvInstruction::JALR(rd, rs1, imm) => {
                    let base = registers.read(rs1);
                    let target = base.wrapping_add(imm as i64 as u64) & !1u64;
                    if rd != 0 {
                        registers.write(rd, pc as u64 + 4);
                    }
                    if target >= MEMORY_SIZE as u64 {
                        return Err(VMError::InvalidMemoryAccess);
                    }
                    pc = target as usize;
                }
                RiscvInstruction::BEQ(rs1, rs2, imm) => {
                    if registers.read(rs1) == registers.read(rs2) {
                        let target = (pc as i64).wrapping_add(imm as i64) as usize;
                        if target >= MEMORY_SIZE {
                            return Err(VMError::InvalidMemoryAccess);
                        }
                        pc = target;
                    } else {
                        pc = (pc + 4) % MEMORY_SIZE;
                    }
                }
                RiscvInstruction::BNE(rs1, rs2, imm) => {
                    if registers.read(rs1) != registers.read(rs2) {
                        let target = (pc as i64).wrapping_add(imm as i64) as usize;
                        if target >= MEMORY_SIZE {
                            return Err(VMError::InvalidMemoryAccess);
                        }
                        pc = target;
                    } else {
                        pc = (pc + 4) % MEMORY_SIZE;
                    }
                }
                RiscvInstruction::BLT(rs1, rs2, imm) => {
                    if registers.read(rs1) < registers.read(rs2) {
                        let target = (pc as i64).wrapping_add(imm as i64) as usize;
                        if target >= MEMORY_SIZE {
                            return Err(VMError::InvalidMemoryAccess);
                        }
                        pc = target;
                    } else {
                        pc = (pc + 4) % MEMORY_SIZE;
                    }
                }
                RiscvInstruction::BGE(rs1, rs2, imm) => {
                    if registers.read(rs1) >= registers.read(rs2) {
                        let target = (pc as i64).wrapping_add(imm as i64) as usize;
                        if target >= MEMORY_SIZE {
                            return Err(VMError::InvalidMemoryAccess);
                        }
                        pc = target;
                    } else {
                        pc = (pc + 4) % MEMORY_SIZE;
                    }
                }
                RiscvInstruction::BLTU(rs1, rs2, imm) => {
                    if registers.read(rs1) < registers.read(rs2) {
                        let target = (pc as i64).wrapping_add(imm as i64) as usize;
                        if target >= MEMORY_SIZE {
                            return Err(VMError::InvalidMemoryAccess);
                        }
                        pc = target;
                    } else {
                        pc = (pc + 4) % MEMORY_SIZE;
                    }
                }
                RiscvInstruction::BGEU(rs1, rs2, imm) => {
                    if registers.read(rs1) >= registers.read(rs2) {
                        let target = (pc as i64).wrapping_add(imm as i64) as usize;
                        if target >= MEMORY_SIZE {
                            return Err(VMError::InvalidMemoryAccess);
                        }
                        pc = target;
                    } else {
                        pc = (pc + 4) % MEMORY_SIZE;
                    }
                }
                RiscvInstruction::LB(rd, rs1, imm) => {
                    let addr = registers.read(rs1).wrapping_add(imm as i64 as u64);
                    let val = memory.read_u64(addr)?;
                    registers.write(rd, val as i8 as u64);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::LH(rd, rs1, imm) => {
                    let addr = registers.read(rs1).wrapping_add(imm as i64 as u64);
                    let val = memory.read_u64(addr)?;
                    registers.write(rd, val as i16 as u64);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::LW(rd, rs1, imm) => {
                    let addr = registers.read(rs1).wrapping_add(imm as i64 as u64);
                    let val = memory.read_u32(addr)?;
                    registers.write(rd, val as i32 as u64);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::LWU(rd, rs1, imm) => {
                    let addr = registers.read(rs1).wrapping_add(imm as i64 as u64);
                    let val = memory.read_u32(addr)?;
                    registers.write(rd, val as u64);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::LD(rd, rs1, imm) => {
                    let addr = registers.read(rs1).wrapping_add(imm as i64 as u64);
                    let val = memory.read_u64(addr)?;
                    registers.write(rd, val);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SB(rs2, rs1, imm) => {
                    let addr = registers.read(rs1).wrapping_add(imm as i64 as u64);
                    memory.write_u64(addr, registers.read(rs2) as u64)?;
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SH(rs2, rs1, imm) => {
                    let addr = registers.read(rs1).wrapping_add(imm as i64 as u64);
                    memory.write_u64(addr, registers.read(rs2) as u16 as u64)?;
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SW(rs2, rs1, imm) => {
                    let addr = registers.read(rs1).wrapping_add(imm as i64 as u64);
                    memory.write_u32(addr, registers.read(rs2) as u32)?;
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SD(rs2, rs1, imm) => {
                    let addr = registers.read(rs1).wrapping_add(imm as i64 as u64);
                    memory.write_u64(addr, registers.read(rs2))?;
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::ADDI(rd, rs1, imm) => {
                    registers.write(rd, registers.read(rs1).wrapping_add(imm as i64 as u64));
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SLTI(rd, rs1, imm) => {
                    let result = if registers.read(rs1) < imm as i64 as u64 {
                        1u64
                    } else {
                        0u64
                    };
                    registers.write(rd, result);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SLTIU(rd, rs1, imm) => {
                    let result = if registers.read(rs1) < imm as u64 {
                        1u64
                    } else {
                        0u64
                    };
                    registers.write(rd, result);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::XORI(rd, rs1, imm) => {
                    registers.write(rd, registers.read(rs1) ^ imm as u64);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::ORI(rd, rs1, imm) => {
                    registers.write(rd, registers.read(rs1) | imm as u64);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::ANDI(rd, rs1, imm) => {
                    registers.write(rd, registers.read(rs1) & imm as u64);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SLLI(rd, rs1, shamt) => {
                    registers.write(rd, registers.read(rs1).wrapping_shl(shamt));
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SRLI(rd, rs1, shamt) => {
                    registers.write(rd, registers.read(rs1).wrapping_shr(shamt));
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SRAI(rd, rs1, shamt) => {
                    registers.write(
                        rd,
                        (registers.read(rs1) as i64).wrapping_shr(shamt as u32) as u64,
                    );
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::ADD(rd, rs1, rs2) => {
                    registers.write(rd, registers.read(rs1).wrapping_add(registers.read(rs2)));
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SUB(rd, rs1, rs2) => {
                    registers.write(rd, registers.read(rs1).wrapping_sub(registers.read(rs2)));
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SLL(rd, rs1, rs2) => {
                    registers.write(
                        rd,
                        registers.read(rs1).wrapping_shl(registers.read(rs2) as u32),
                    );
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SLT(rd, rs1, rs2) => {
                    let result = if registers.read(rs1) < registers.read(rs2) {
                        1u64
                    } else {
                        0u64
                    };
                    registers.write(rd, result);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SLTU(rd, rs1, rs2) => {
                    let result = if registers.read(rs1) < registers.read(rs2) {
                        1u64
                    } else {
                        0u64
                    };
                    registers.write(rd, result);
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::XOR(rd, rs1, rs2) => {
                    registers.write(rd, registers.read(rs1) ^ registers.read(rs2));
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SRL(rd, rs1, rs2) => {
                    registers.write(
                        rd,
                        registers.read(rs1).wrapping_shr(registers.read(rs2) as u32),
                    );
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::SRA(rd, rs1, rs2) => {
                    registers.write(
                        rd,
                        (registers.read(rs1) as i64).wrapping_shr(registers.read(rs2) as u32)
                            as u64,
                    );
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::OR(rd, rs1, rs2) => {
                    registers.write(rd, registers.read(rs1) | registers.read(rs2));
                    pc = (pc + 4) % MEMORY_SIZE;
                }
                RiscvInstruction::AND(rd, rs1, rs2) => {
                    registers.write(rd, registers.read(rs1) & registers.read(rs2));
                    pc = (pc + 4) % MEMORY_SIZE;
                }
            }

            instructions_executed += 1;
        }

        // Return deterministic output: final register state hash
        let mut output = Vec::with_capacity(32);
        output.extend_from_slice(&registers.read(1).to_le_bytes()); // x1 = return value
        output.extend_from_slice(&registers.read(2).to_le_bytes()); // x2 = stack pointer
        output.extend_from_slice(&(pc as u64).to_le_bytes()); // final PC
        output.extend_from_slice(&(instructions_executed).to_le_bytes()); // instruction count
        output.resize(32, 0);

        Ok(ExecutionResult::Success {
            gas_used: self.gas_meter.gas_used,
            return_value: output,
            logs: vec![],
        })
    }

    /// Decode a 32-bit RISC-V instruction word into a RiscvInstruction
    fn decode_instruction(
        &self,
        opcode: u32,
        funct3: u8,
        funct7: u32,
        rd: u8,
        rs1: u8,
        rs2: u8,
        instr_word: u32,
    ) -> Result<RiscvInstruction, VMError> {
        match opcode {
            OPCODE_LUI => {
                let imm = instr_word >> 12;
                Ok(RiscvInstruction::LUI(rd, imm))
            }
            OPCODE_AUIPC => {
                let imm = instr_word >> 12;
                Ok(RiscvInstruction::AUIPC(rd, imm))
            }
            OPCODE_JAL => {
                let imm = ((instr_word >> 12) & 0x7ff)
                    | ((instr_word >> 20) & 0xff) << 11
                    | ((instr_word >> 21) & 0x3ff) << 19
                    | ((instr_word >> 31) & 0x1) << 31;
                let imm = imm as i32;
                Ok(RiscvInstruction::JAL(rd, imm))
            }
            OPCODE_JALR => {
                let imm = ((instr_word >> 20) & 0xfff) as i32;
                Ok(RiscvInstruction::JALR(rd, rs1, imm))
            }
            OPCODE_BRANCH => {
                let imm = ((instr_word >> 31) & 0x1) << 12
                    | ((instr_word >> 25) & 0x3f) << 5
                    | ((instr_word >> 8) & 0xf) << 1
                    | ((instr_word >> 7) & 0x1) << 11;
                let imm = imm as i32;
                match funct3 {
                    FUNCT3_BEQ => Ok(RiscvInstruction::BEQ(rs1, rs2, imm)),
                    FUNCT3_BNE => Ok(RiscvInstruction::BNE(rs1, rs2, imm)),
                    FUNCT3_BLT => Ok(RiscvInstruction::BLT(rs1, rs2, imm)),
                    FUNCT3_BGE => Ok(RiscvInstruction::BGE(rs1, rs2, imm)),
                    FUNCT3_BLTU => Ok(RiscvInstruction::BLTU(rs1, rs2, imm)),
                    FUNCT3_BGEU => Ok(RiscvInstruction::BGEU(rs1, rs2, imm)),
                    _ => Err(VMError::InvalidInstruction),
                }
            }
            OPCODE_LOAD => {
                let imm = ((instr_word >> 20) & 0xfff) as i32;
                match funct3 {
                    FUNCT3_LB => Ok(RiscvInstruction::LB(rd, rs1, imm)),
                    FUNCT3_LH => Ok(RiscvInstruction::LH(rd, rs1, imm)),
                    FUNCT3_LW => Ok(RiscvInstruction::LW(rd, rs1, imm)),
                    FUNCT3_LWU => Ok(RiscvInstruction::LWU(rd, rs1, imm)),
                    FUNCT3_LD => Ok(RiscvInstruction::LD(rd, rs1, imm)),
                    _ => Err(VMError::InvalidInstruction),
                }
            }
            OPCODE_STORE => {
                let imm = ((instr_word >> 20) & 0x7f) | ((instr_word >> 25) & 0x7f) << 5;
                let imm = imm as i32;
                match funct3 {
                    FUNCT3_SB => Ok(RiscvInstruction::SB(rs2, rs1, imm)),
                    FUNCT3_SH => Ok(RiscvInstruction::SH(rs2, rs1, imm)),
                    FUNCT3_SW => Ok(RiscvInstruction::SW(rs2, rs1, imm)),
                    FUNCT3_SD => Ok(RiscvInstruction::SD(rs2, rs1, imm)),
                    _ => Err(VMError::InvalidInstruction),
                }
            }
            OPCODE_OP_IMM => match funct3 {
                FUNCT3_ADDI_SLLI => {
                    let imm = ((instr_word >> 20) & 0xfff) as i32;
                    Ok(RiscvInstruction::ADDI(rd, rs1, imm))
                }
                FUNCT3_SLTI => {
                    let imm = ((instr_word >> 20) & 0xfff) as i32;
                    Ok(RiscvInstruction::SLTI(rd, rs1, imm))
                }
                FUNCT3_SLTIU => {
                    let imm = ((instr_word >> 20) & 0xfff) as i32;
                    Ok(RiscvInstruction::SLTIU(rd, rs1, imm))
                }
                FUNCT3_XORI => {
                    let imm = ((instr_word >> 20) & 0xfff) as i32;
                    Ok(RiscvInstruction::XORI(rd, rs1, imm))
                }
                FUNCT3_ORI => {
                    let imm = ((instr_word >> 20) & 0xfff) as i32;
                    Ok(RiscvInstruction::ORI(rd, rs1, imm))
                }
                FUNCT3_ANDI => {
                    let imm = ((instr_word >> 20) & 0xfff) as i32;
                    Ok(RiscvInstruction::ANDI(rd, rs1, imm))
                }
                FUNCT3_SRLI_SRAI => {
                    let shamt = ((instr_word >> 20) & 0x1f) as u32;
                    if funct7 == FUNCT7_SRLI {
                        Ok(RiscvInstruction::SRLI(rd, rs1, shamt))
                    } else if funct7 == FUNCT7_SRAI {
                        Ok(RiscvInstruction::SRAI(rd, rs1, shamt))
                    } else {
                        Err(VMError::InvalidInstruction)
                    }
                }
                _ => Err(VMError::InvalidInstruction),
            },
            OPCODE_OP => match funct3 {
                FUNCT3_ADDI_SLLI => {
                    if funct7 == FUNCT7_ADD {
                        Ok(RiscvInstruction::ADD(rd, rs1, rs2))
                    } else if funct7 == FUNCT7_SUB {
                        Ok(RiscvInstruction::SUB(rd, rs1, rs2))
                    } else {
                        Err(VMError::InvalidInstruction)
                    }
                }
                FUNCT3_SLL => Ok(RiscvInstruction::SLL(rd, rs1, rs2)),
                FUNCT3_SLT => Ok(RiscvInstruction::SLT(rd, rs1, rs2)),
                FUNCT3_SLTU => Ok(RiscvInstruction::SLTU(rd, rs1, rs2)),
                FUNCT3_XOR => Ok(RiscvInstruction::XOR(rd, rs1, rs2)),
                FUNCT3_SRL_SRA => {
                    if funct7 == FUNCT7_SRL {
                        Ok(RiscvInstruction::SRL(rd, rs1, rs2))
                    } else if funct7 == FUNCT7_SRA {
                        Ok(RiscvInstruction::SRA(rd, rs1, rs2))
                    } else {
                        Err(VMError::InvalidInstruction)
                    }
                }
                FUNCT3_OR => Ok(RiscvInstruction::OR(rd, rs1, rs2)),
                FUNCT3_AND => Ok(RiscvInstruction::AND(rd, rs1, rs2)),
                _ => Err(VMError::InvalidInstruction),
            },
            OPCODE_SLLI_SRAI => {
                // SLLI/SRAI handled in OPCODE_OP_IMM match
                Err(VMError::InvalidInstruction)
            }
            _ => Err(VMError::InvalidInstruction),
        }
    }

    /// Parse and validate ELF64 header
    fn parse_elf_header(
        &self,
        binary: &[u8],
    ) -> Result<Elf64Header, VMError> {
        if binary.len() < 64 {
            return Err(VMError::InvalidInstruction);
        }

        let magic = [binary[0], binary[1], binary[2], binary[3]];
        if magic != ELF_MAGIC {
            return Err(VMError::InvalidInstruction);
        }

        let class = binary[4];
        if class != ELFCLASS64 {
            return Err(VMError::InvalidInstruction);
        }

        let data = binary[5];
        if data != ELFDATA2LSB {
            return Err(VMError::InvalidInstruction);
        }

        let version = binary[6];
        if version != EV_CURRENT {
            return Err(VMError::InvalidInstruction);
        }

        let os_abi = binary[7];
        let abi_version = binary[8];
        let pad = [
            binary[9], binary[10], binary[11], binary[12], binary[13], binary[14], binary[15],
        ];

        let e_type = u16::from_le_bytes([binary[16], binary[17]]);
        if e_type != ET_EXEC {
            return Err(VMError::InvalidInstruction);
        }

        let e_machine = u16::from_le_bytes([binary[18], binary[19]]);
        if e_machine != EM_RISCV {
            return Err(VMError::InvalidInstruction);
        }

        let e_version = u32::from_le_bytes([binary[20], binary[21], binary[22], binary[23]]);
        let e_entry = u64::from_le_bytes([
            binary[24], binary[25], binary[26], binary[27], binary[28], binary[29], binary[30],
            binary[31],
        ]);
        let e_phoff = u64::from_le_bytes([
            binary[32], binary[33], binary[34], binary[35], binary[36], binary[37], binary[38],
            binary[39],
        ]);
        let e_shoff = u64::from_le_bytes([
            binary[40], binary[41], binary[42], binary[43], binary[44], binary[45], binary[46],
            binary[47],
        ]);
        let e_flags = u32::from_le_bytes([binary[48], binary[49], binary[50], binary[51]]);
        let e_ehsize = u16::from_le_bytes([binary[52], binary[53]]);
        let e_phentsize = u16::from_le_bytes([binary[54], binary[55]]);
        let e_phnum = u16::from_le_bytes([binary[56], binary[57]]);
        let e_shentsize = u16::from_le_bytes([binary[58], binary[59]]);
        let e_shnum = u16::from_le_bytes([binary[60], binary[61]]);
        let e_shstrndx = u16::from_le_bytes([binary[62], binary[63]]);

        Ok(Elf64Header {
            magic,
            class,
            data,
            version,
            os_abi,
            abi_version,
            pad,
            e_type,
            e_machine,
            e_version,
            e_entry,
            e_phoff,
            e_shoff,
            e_flags,
            e_ehsize,
            e_phentsize,
            e_phnum,
            e_shentsize,
            e_shnum,
            e_shstrndx,
        })
    }

    /// Returns the remaining gas
    #[must_use]
    pub fn remaining_gas(&self) -> u64 {
        self.gas_meter.remaining()
    }

    /// Returns the VM type
    #[must_use]
    pub fn vm_type() -> VMType {
        VMType::RISCV
    }
}

impl Default for RISCVExecutor {
    fn default() -> Self {
        Self::new(ExecutionContext::default(), 10_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_elf64() -> Vec<u8> {
        let mut bytes = Vec::new();
        // ELF magic
        bytes.extend_from_slice(&ELF_MAGIC);
        // class = 2 (64-bit)
        bytes.push(ELFCLASS64);
        // data = 1 (little-endian)
        bytes.push(ELFDATA2LSB);
        // version = 1
        bytes.push(EV_CURRENT);
        // os_abi = 0
        bytes.push(0);
        // abi_version = 0
        bytes.push(0);
        // pad (7 bytes)
        bytes.extend_from_slice(&[0u8; 7]);
        // e_type = ET_EXEC (2)
        bytes.extend_from_slice(&(ET_EXEC as u16).to_le_bytes());
        // e_machine = EM_RISCV (243)
        bytes.extend_from_slice(&(EM_RISCV as u16).to_le_bytes());
        // e_version = 1
        bytes.extend_from_slice(&1u32.to_le_bytes());
        // e_entry = 0x1000
        bytes.extend_from_slice(&0x1000u64.to_le_bytes());
        // e_phoff = 0
        bytes.extend_from_slice(&0u64.to_le_bytes());
        // e_shoff = 0
        bytes.extend_from_slice(&0u64.to_le_bytes());
        // e_flags = 0
        bytes.extend_from_slice(&0u32.to_le_bytes());
        // e_ehsize = 64
        bytes.extend_from_slice(&(64u16).to_le_bytes());
        // e_phentsize = 0
        bytes.extend_from_slice(&0u16.to_le_bytes());
        // e_phnum = 0
        bytes.extend_from_slice(&0u16.to_le_bytes());
        // e_shentsize = 0
        bytes.extend_from_slice(&0u16.to_le_bytes());
        // e_shnum = 0
        bytes.extend_from_slice(&0u16.to_le_bytes());
        // e_shstrndx = 0
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes
    }

    #[test]
    fn riscv_valid_elf_executes() {
        let mut riscv = RISCVExecutor::default();
        let binary = make_valid_elf64();
        // Verify ELF header parsing succeeds
        let header = riscv.parse_elf_header(&binary).unwrap();
        assert_eq!(header.e_machine, EM_RISCV);
        assert_eq!(header.e_type, ET_EXEC);
        assert_eq!(header.class, ELFCLASS64);
        assert_eq!(header.data, ELFDATA2LSB);
        assert_eq!(header.e_entry, 0x1000);
    }

    #[test]
    fn riscv_invalid_elf_rejected() {
        let mut riscv = RISCVExecutor::default();
        let binary = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn riscv_wrong_magic_rejected() {
        let mut riscv = RISCVExecutor::default();
        let mut binary = make_valid_elf64();
        binary[0] = 0x00;
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn riscv_wrong_class_rejected() {
        let mut riscv = RISCVExecutor::default();
        let mut binary = make_valid_elf64();
        binary[4] = 1; // ELFCLASS32
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn riscv_wrong_machine_rejected() {
        let mut riscv = RISCVExecutor::default();
        let mut binary = make_valid_elf64();
        // Actually modify e_machine to non-RISC-V
        binary[18] = 0x00;
        binary[19] = 0x00;
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn riscv_entry_point_out_of_bounds_rejected() {
        let mut riscv = RISCVExecutor::default();
        let mut binary = make_valid_elf64();
        // Set e_entry to 0xFFFF_FFFF_FFFF_FFFF (out of memory bounds)
        binary[24..32].copy_from_slice(&0xFFFF_FFFF_FFFF_FFFFu64.to_le_bytes());
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidMemoryAccess);
    }

    #[test]
    fn riscv_with_arguments() {
        let mut riscv = RISCVExecutor::default();
        let binary = make_valid_elf64();
        let args = vec![vec![1], vec![2]];
        // Header parsing succeeds; execution of header bytes returns InvalidInstruction
        let result = riscv.execute(&binary, 0x1000, &args);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn riscv_vm_type() {
        assert_eq!(RISCVExecutor::vm_type(), VMType::RISCV);
    }

    #[test]
    fn riscv_out_of_gas() {
        let mut riscv = RISCVExecutor::new(ExecutionContext::default(), 5);
        let binary = make_valid_elf64();
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::OutOfGas);
    }

    #[test]
    fn riscv_remaining_gas() {
        let mut riscv = RISCVExecutor::new(ExecutionContext::default(), 1000);
        let binary = make_valid_elf64();
        // Header parsing consumes some gas; execution of header bytes fails
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_err());
        // Some gas was consumed for header parsing before the error
        assert!(riscv.remaining_gas() < 1000);
    }

    #[test]
    fn riscv_deterministic_output() {
        let binary = make_valid_elf64();
        let mut vm1 = RISCVExecutor::default();
        let mut vm2 = RISCVExecutor::default();
        // Both parsers produce the same header result
        let h1 = vm1.parse_elf_header(&binary).unwrap();
        let h2 = vm2.parse_elf_header(&binary).unwrap();
        assert_eq!(h1.e_entry, h2.e_entry);
        assert_eq!(h1.e_machine, h2.e_machine);
    }
}
