use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub enum Instruction {
  CLS,
  RET,
  JP { a: u16 },
  CALL { a: u16 },
  SEi { r: u8, v: u8 },
  SNEi { r: u8, v: u8 },
  SEr { r1: u8, r2: u8 },
  SETi { r: u8, v: u8 },
  ADDi { r: u8, v: u8 },
  SETr { r1: u8, r2: u8 },
  OR { r1: u8, r2: u8 },
  AND { r1: u8, r2: u8 },
  XOR { r1: u8, r2: u8 },
  ADD { r1: u8, r2: u8 },
  SUB { r1: u8, r2: u8 },
  SHR { r1: u8, r2: u8 },
  SUBN { r1: u8, r2: u8 },
  SHL { r1: u8, r2: u8 },
  SNEr { r1: u8, r2: u8 },
  LDI { a: u16 },
  JPR { a: u16 },
  RND { r: u8, v: u8 },
  DRW { r1: u8, r2: u8, v: u8 },
  SKP { v: u8 },
  SKNP { v: u8 },
  LDT { r: u8 },
  INP { r: u8 },
  SDTr { r: u8 },
  SSTr { r: u8 },
  ADDI { r: u8 },
  STOR { r: u8 },
  LOAD { r: u8 },
}

#[derive(Debug)]
pub enum ParseError {
  InvalidInstruction(u16),
}

impl Error for ParseError {}

impl Display for ParseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl TryFrom<u16> for Instruction {
  type Error = ParseError;

  fn try_from(instr: u16) -> Result<Self, Self::Error> {
    let rest = instr & 0x0FFF;
    let lo = (rest & 0xFF) as u8;
    let nibbles = [
      ((0xF000 & instr) >> 12) as u8,
      ((0x0F00 & instr) >> 8) as u8,
      ((0x00F0 & instr) >> 4) as u8,
      ((0x000F & instr) >> 0) as u8,
    ];

    match nibbles {
      [0x0, 0x0, 0xE, 0x0] => Ok(Instruction::CLS),
      [0x0, 0x0, 0xE, 0xE] => Ok(Instruction::RET),
      [0x1, _, _, _] => Ok(Instruction::JP { a: rest }),
      [0x2, _, _, _] => Ok(Instruction::CALL { a: rest }),
      [0x3, r, _, _] => Ok(Instruction::SEi { r, v: lo }),
      [0x4, r, _, _] => Ok(Instruction::SNEi { r, v: lo }),
      [0x5, r1, r2, 0x0] => Ok(Instruction::SEr { r1, r2 }),
      [0x6, r, _, _] => Ok(Instruction::SETi { r, v: lo }),
      [0x7, r, _, _] => Ok(Instruction::ADDi { r, v: lo }),
      [0x8, r1, r2, 0x0] => Ok(Instruction::SETr { r1, r2 }),
      [0x8, r1, r2, 0x1] => Ok(Instruction::OR { r1, r2 }),
      [0x8, r1, r2, 0x2] => Ok(Instruction::AND { r1, r2 }),
      [0x8, r1, r2, 0x3] => Ok(Instruction::XOR { r1, r2 }),
      [0x8, r1, r2, 0x4] => Ok(Instruction::ADD { r1, r2 }),
      [0x8, r1, r2, 0x5] => Ok(Instruction::SUB { r1, r2 }),
      [0x8, r1, r2, 0x6] => Ok(Instruction::SHR { r1, r2 }),
      [0x8, r1, r2, 0x7] => Ok(Instruction::SUBN { r1, r2 }),
      [0x8, r1, r2, 0xE] => Ok(Instruction::SHL { r1, r2 }),
      [0x9, r1, r2, 0x0] => Ok(Instruction::SNEr { r1, r2 }),
      [0xA, _, _, _] => Ok(Instruction::LDI { a: rest }),
      [0xB, _, _, _] => Ok(Instruction::JPR { a: rest }),
      [0xC, r, _, _] => Ok(Instruction::RND { r, v: lo }),
      [0xD, r1, r2, v] => Ok(Instruction::DRW { r1, r2, v }),
      [0xE, v, 0x9, 0xE] => Ok(Instruction::SKP { v }),
      [0xE, v, 0xA, 0x1] => Ok(Instruction::SKNP { v }),
      [0xF, r, 0x0, 0x7] => Ok(Instruction::LDT { r }),
      [0xF, r, 0x0, 0xA] => Ok(Instruction::INP { r }),
      [0xF, r, 0x1, 0x5] => Ok(Instruction::SDTr { r }),
      [0xF, r, 0x1, 0x8] => Ok(Instruction::SSTr { r }),
      [0xF, r, 0x1, 0xE] => Ok(Instruction::ADDI { r }),
      // missing digits
      [0xF, r, 0x5, 0x5] => Ok(Instruction::STOR { r }),
      [0xF, r, 0x6, 0x5] => Ok(Instruction::LOAD { r }),
      [_, _, _, _] => Err(ParseError::InvalidInstruction(instr)),
    }
  }
}
