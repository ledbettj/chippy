use std::fmt::Display;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use fastrand;

use crate::instruction::{Instruction, ParseError};
use crate::screen::ScreenUpdate;

pub struct Machine {
  memory: [u8; Machine::MEMORY_SIZE],
  registers: [u8; Machine::REGISTER_COUNT],
  reg_i: u16,
  timers: [u8; Machine::TIMER_COUNT],
  ip: u16,
  stack: Vec<u16>,
  display: mpsc::Sender<ScreenUpdate>,
  collision: mpsc::Receiver<bool>,
  keypad: mpsc::Receiver<u16>,
  keys: u16,
}

impl Machine {
  const MEMORY_SIZE: usize = 0x1000;
  const REGISTER_COUNT: usize = 16;
  const TIMER_COUNT: usize = 2;
  const LOAD_ADDR: usize = 0x200;

  pub fn load(
    input: &[u8],
    display: mpsc::Sender<ScreenUpdate>,
    collision: mpsc::Receiver<bool>,
    keypad: mpsc::Receiver<u16>,
  ) -> Result<Self, ()> {
    if input.len() > 0x1000 - Machine::LOAD_ADDR {
      Err(())
    } else {
      let mut m = Machine::new(display, collision, keypad);
      m.memory[Machine::LOAD_ADDR..][..input.len()].copy_from_slice(&input);
      Ok(m)
    }
  }

  pub fn new(
    display: mpsc::Sender<ScreenUpdate>,
    collision: mpsc::Receiver<bool>,
    keypad: mpsc::Receiver<u16>,
  ) -> Self {
    Machine {
      memory: [0; Machine::MEMORY_SIZE],
      registers: [0; 16],
      reg_i: 0,
      timers: [0; 2],
      ip: Machine::LOAD_ADDR as u16,
      stack: vec![],
      keys: 0,
      display,
      collision,
      keypad,
    }
  }

  pub fn run(&mut self, hz: u32) -> Result<(), ParseError> {
    let interval_ms = Duration::from_secs_f64(1.0 / (hz as f64));
    let mut last = Instant::now();

    loop {
      self.step()?;
      // TODO: improve timing logic
      let mut remaining = interval_ms.saturating_sub(Instant::now() - last);
      while !remaining.is_zero() {
        std::thread::sleep(Duration::from_micros(10));
        remaining = interval_ms.saturating_sub(Instant::now() - last);
      }
      last = Instant::now();
    }
  }

  fn reg(&self, r: u8) -> u8 {
    self.registers[r as usize]
  }

  fn reg_set(&mut self, r: u8, v: u8) {
    self.registers[r as usize] = v;
  }

  fn mem(&self, a: u16) -> u8 {
    self.memory[a as usize]
  }

  fn mem_set(&mut self, a: u16, v: u8) {
    self.memory[a as usize] = v;
  }

  fn eval_next(&mut self) -> Result<(), ParseError> {
    let hi = self.memory[self.ip as usize];
    let lo = self.memory[(self.ip + 1) as usize];
    let instr: Instruction = ((hi as u16) << 8 | (lo as u16)).try_into()?;
    //println!("{:?}: {:#02x}", instr, self.keys);
    let next_instr = self.ip + 2;

    self.ip = match instr {
      Instruction::CLS => {
        self
          .display
          .send(ScreenUpdate::Clear)
          .expect("Display Disconnected!");
        next_instr
      }
      Instruction::RET => self.stack.pop().expect("RET without CALL"),
      Instruction::JP { a } => a,
      Instruction::CALL { a } => {
        self.stack.push(self.ip);
        a
      }
      Instruction::SEi { r, v } => {
        if self.reg(r) == v {
          self.ip + 4
        } else {
          next_instr
        }
      }
      Instruction::SNEi { r, v } => {
        if self.reg(r) != v {
          self.ip + 4
        } else {
          next_instr
        }
      }
      Instruction::SEr { r1, r2 } => {
        if self.reg(r1) == self.reg(r2) {
          self.ip + 4
        } else {
          next_instr
        }
      }
      Instruction::SETi { r, v } => {
        self.reg_set(r, v);
        next_instr
      }
      Instruction::ADDi { r, v } => {
        let (result, _) = self.reg(r).overflowing_add(v);
        self.reg_set(r, result);
        next_instr
      }
      Instruction::SETr { r1, r2 } => {
        self.reg_set(r1, self.reg(r2));
        next_instr
      }
      Instruction::OR { r1, r2 } => {
        self.registers[r1 as usize] |= self.reg(r2);
        next_instr
      }
      Instruction::AND { r1, r2 } => {
        self.registers[r1 as usize] &= self.reg(r2);
        next_instr
      }
      Instruction::XOR { r1, r2 } => {
        self.registers[r1 as usize] ^= self.reg(r2);
        next_instr
      }
      Instruction::ADD { r1, r2 } => {
        let (v, carry) = self.reg(r1).overflowing_add(self.reg(r2));
        self.reg_set(r1, v);
        self.reg_set(0xf, if carry { 1 } else { 0 });
        next_instr
      }
      Instruction::SUB { r1, r2 } => {
        let (v, carry) = self.reg(r1).overflowing_sub(self.reg(r2));
        self.reg_set(r1, v);
        self.reg_set(0xf, if carry { 1 } else { 0 });
        next_instr
      }
      Instruction::SHR { r1, .. } => {
        let (v, carry) = self.reg(r1).overflowing_shr(1);
        self.reg_set(r1, v);
        self.reg_set(0xf, if carry { 1 } else { 0 });
        next_instr
      }
      Instruction::SUBN { r1, r2 } => {
        let (v, carry) = self.reg(r2).overflowing_sub(self.reg(r1));
        self.reg_set(r1, v);
        self.reg_set(0xf, if carry { 1 } else { 0 });
        next_instr
      }
      Instruction::SHL { r1, .. } => {
        let (v, carry) = self.reg(r1).overflowing_shl(1);
        self.reg_set(r1, v);
        self.reg_set(0xf, if carry { 1 } else { 0 });
        next_instr
      }
      Instruction::SNEr { r1, r2 } => {
        if self.reg(r1) != self.reg(r2) {
          self.ip + 4
        } else {
          next_instr
        }
      }
      Instruction::LDI { a } => {
        self.reg_i = a;
        next_instr
      }
      Instruction::JPR { a } => a + self.reg(0) as u16,
      Instruction::RND { r, v } => {
        self.reg_set(r, fastrand::u8(..) & v);
        next_instr
      }
      Instruction::DRW { r1, r2, v } => {
        let bytes = &self.memory[(self.reg_i as usize)..][..(v as usize)];
        let coords = (self.reg(r1) as usize, self.reg(r2) as usize);
        self
          .display
          .send(ScreenUpdate::Draw {
            bytes: bytes.to_vec(),
            coords,
          })
          .expect("Display disconnected");
        self.reg_set(
          0xf,
          if self.collision.recv().expect("Display disconnected") {
            1
          } else {
            0
          },
        );
        next_instr
      }
      Instruction::SKP { v } => {
        println!("{:?}", instr);
        println!("{:?}", self.keys);
        if self.keys == v as u16 {
          next_instr + 2
        } else {
          next_instr
        }
      }
      Instruction::SKNP { v } => {
        println!("{:?}", instr);
        println!("{:?}", self.keys);
        if self.keys != v as u16 {
          next_instr + 2
        } else {
          next_instr
        }
      }
      Instruction::LDT { r } => {
        self.reg_set(r, self.timers[0]);
        next_instr
      }
      Instruction::INP { r } => {
        println!("unhandled instruction {:?}", instr);
        // TODO: block for input
        next_instr
      }
      Instruction::SDTr { r } => {
        self.timers[0] = self.reg(r);
        next_instr
      }
      Instruction::SSTr { r } => {
        self.timers[1] = self.reg(r);
        next_instr
      }
      Instruction::ADDI { r } => {
        self.reg_i += self.reg(r) as u16;
        next_instr
      }
      Instruction::STOR { r } => {
        for i in 0..(r + 1) {
          self.mem_set(self.reg_i + i as u16, self.reg(i));
        }
        next_instr
      }
      Instruction::LOAD { r } => {
        for i in 0..(r + 1) {
          self.reg_set(i, self.mem(self.reg_i + i as u16));
        }
        next_instr
      }
    };

    Ok(())
  }

  fn step(&mut self) -> Result<(), ParseError> {
    ////self.decrement_timers();
    while let Ok(v) = self.keypad.try_recv() {
      self.keys = v;
    }
    self.eval_next()
  }

  fn decrement_timers(&mut self) {
    self.timers[0] = self.timers[0].saturating_sub(1);
    self.timers[1] = self.timers[1].saturating_sub(1);
  }
}

impl Display for Machine {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "ip = {:#02x}", self.ip)?;
    writeln!(f, "i = {:#02x}", self.reg_i)?;

    for (index, value) in self.registers.iter().enumerate() {
      writeln!(f, "r{} = {}", index, value)?;
    }

    for i in (0..Machine::MEMORY_SIZE).step_by(16) {
      write!(f, "{:#02x}\t", i)?;
      self.memory[i..(i + 16)].iter().for_each(|value| {
        write!(f, "{:02x} ", value).unwrap();
      });
      write!(f, "\t")?;
      self.memory[i..(i + 16)].iter().for_each(|&value| {
        let ch = if value < 0x20 || value > 0x7e {
          '.'
        } else {
          value as char
        };
        write!(f, "{}", ch).unwrap();
      });
      writeln!(f)?;
    }

    Ok(())
  }
}
