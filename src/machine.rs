use std::fmt::Display;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use fastrand;

use crate::screen::ScreenUpdate;

pub struct Machine {
  memory: [u8; Machine::MEMORY_SIZE],
  registers: [u8; Machine::REGISTER_COUNT],
  reg_i: u16,
  timers: [u8; Machine::TIMER_COUNT],
  ip: u16,
  stack: Vec<u16>,
  display: mpsc::Sender<ScreenUpdate>,
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

impl Machine {
  const MEMORY_SIZE: usize = 0x1000;
  const REGISTER_COUNT: usize = 16;
  const TIMER_COUNT: usize = 2;
  const LOAD_ADDR: usize = 0x200;

  pub fn load(input: &[u8], display: mpsc::Sender<ScreenUpdate>) -> Result<Self, ()> {
    if input.len() > 0x1000 - Machine::LOAD_ADDR {
      Err(())
    } else {
      let mut m = Machine::new(display);
      m.memory[Machine::LOAD_ADDR..][..input.len()].copy_from_slice(&input);
      Ok(m)
    }
  }

  pub fn new(display: mpsc::Sender<ScreenUpdate>) -> Self {
    Machine {
      memory: [0; Machine::MEMORY_SIZE],
      registers: [0; 16],
      reg_i: 0,
      timers: [0; 2],
      ip: Machine::LOAD_ADDR as u16,
      stack: vec![],
      display,
    }
  }

  pub fn run(&mut self, hz: u32) {
    let interval_ms = Duration::from_secs_f64(1.0 / (hz as f64));
    let mut last = Instant::now();

    loop {
      self.step();
      let remaining = interval_ms.saturating_sub(Instant::now() - last);
      if remaining.is_zero() {
        println!("Warning: unable to maintain Machine hz");
      }
      std::thread::sleep(remaining);
      last = Instant::now();
    }
  }

  fn eval_next(&mut self) {
    let hi = self.memory[self.ip as usize];
    let lo = self.memory[(self.ip + 1) as usize];
    let instr = (hi as u16) << 8 | (lo as u16);
    let rest = instr & 0x0FFF;
    let nibbles = [
      (0xF000 & instr) >> 12,
      (0x0F00 & instr) >> 8,
      (0x00F0 & instr) >> 4,
      (0x000F & instr) >> 0,
    ];
    println!("{:#02x}", instr);
    match nibbles {
      [0x0, 0x0, 0xE, 0x0] => {
        // CLS
        let payload = ScreenUpdate::Clear;
        self.display.send(payload).expect("Display disconnected");
        self.ip += 2;
      }
      // [0x0, 0x0, 0xE, 0xE] => {
      //   // RET
      // },
      [0x1, _, _, _] => {
        // JP
        self.ip = rest;
      }
      [0x2, _, _, _] => {
        // CALL
        self.stack.push(self.ip as u16);
        self.ip = rest;
      }
      [0x3, x, _, _] => {
        // SE
        self.ip += if self.registers[x as usize] == lo {
          4
        } else {
          2
        };
      }
      [0x4, x, _, _] => {
        // SNE
        self.ip += if self.registers[x as usize] != lo {
          4
        } else {
          2
        };
      }
      [0x5, x, y, 0] => {
        // SE
        self.ip += if self.registers[x as usize] == self.registers[y as usize] {
          4
        } else {
          2
        };
      }
      [0x6, x, _, _] => {
        // LD
        self.registers[x as usize] = lo;
        self.ip += 2;
      }
      [0x7, x, _, _] => {
        self.registers[x as usize] += lo;
        self.ip += 2;
      }
      [0x8, x, y, 0] => {
        self.registers[x as usize] = self.registers[y as usize];
        self.ip += 2;
      }
      [0x8, x, y, 1] => {
        self.registers[x as usize] |= self.registers[y as usize];
        self.ip += 2;
      }
      [0x8, x, y, 2] => {
        self.registers[x as usize] &= self.registers[y as usize];
        self.ip += 2;
      }
      [0x8, x, y, 3] => {
        self.registers[x as usize] ^= self.registers[y as usize];
        self.ip += 2;
      }
      [0x8, x, y, 4] => {
        let (v, carry) = self.registers[x as usize].overflowing_add(self.registers[y as usize]);
        self.registers[x as usize] = v;
        self.registers[0xf] = if carry { 1 } else { 0 };
        self.ip += 2;
      }
      [0x8, x, y, 5] => {
        let (v, carry) = self.registers[x as usize].overflowing_sub(self.registers[y as usize]);
        self.registers[x as usize] = v;
        self.registers[0xf] = if carry { 1 } else { 0 };
        self.ip += 2;
      }
      [0x8, x, _, 6] => {
        let (v, carry) = self.registers[x as usize].overflowing_shr(1);
        self.registers[x as usize] = v;
        self.registers[0xf] = if carry { 1 } else { 0 };
        self.ip += 2;
      }
      [0x8, x, y, 7] => {
        let (v, carry) = self.registers[y as usize].overflowing_sub(self.registers[x as usize]);
        self.registers[x as usize] = v;
        self.registers[0xf] = if carry { 1 } else { 0 };
        self.ip += 2;
      }
      [0x8, x, _, 0xE] => {
        let (v, carry) = self.registers[x as usize].overflowing_shl(1);
        self.registers[x as usize] = v;
        self.registers[0xf] = if carry { 1 } else { 0 };
        self.ip += 2;
      }
      [0x9, x, y, 0x0] => {
        // SNE
        self.ip += if self.registers[x as usize] != self.registers[y as usize] {
          4
        } else {
          2
        };
      }
      [0xA, _, _, _] => {
        self.reg_i = rest;
        self.ip += 2;
      }
      [0xB, _, _, _] => {
        self.ip = rest + (self.registers[0] as u16);
      }
      [0xC, x, _, _] => {
        self.registers[x as usize] = fastrand::u8(..) & lo;
      }
      [0xD, x, y, n] => {
        println!("draw instr");
        let bytes = &self.memory[(self.reg_i as usize)..][..(n as usize)];
        let coords = (
          self.registers[x as usize] as usize,
          self.registers[y as usize] as usize,
        );
        let payload = ScreenUpdate::Draw {
          bytes: bytes.to_vec(),
          coords,
        };
        self.display.send(payload).expect("Display disconnected");
        self.ip += 2;
      }
      [_, _, _, _] => {
        panic!("unimplemented instruction: {:#02x}", instr);
      }
    };
  }

  fn step(&mut self) {
    ////self.decrement_timers();
    self.eval_next();
  }

  fn decrement_timers(&mut self) {
    self.timers[0] = self.timers[0].saturating_sub(1);
    self.timers[1] = self.timers[1].saturating_sub(1);
  }
}
