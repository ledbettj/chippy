use std::env;
use std::fs::File;
use std::fmt::Display;
use std::io::Read;
use std::time::{Instant, Duration};

use std::thread;
use std::sync::mpsc;

use fastrand;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

struct Machine {
  memory:    [u8; Machine::MEMORY_SIZE],
  registers: [u8; Machine::REGISTER_COUNT],
  reg_i:     u16,
  timers:    [u8; Machine::TIMER_COUNT],
  ip:        u16,
  stack:     Vec<u16>,
  display:   mpsc::Sender<DisplayUpdate>,
}

#[derive(Debug)]
struct DisplayUpdate {
  bytes: Vec<u8>,
  coords: (usize, usize)
}

type Screen = [[u8; 64]; 32];

impl Display for Machine {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "ip = {:#02x}", self.ip)?;
    for (index, value) in self.registers.iter().enumerate() {
      writeln!(f, "r{} = {}", index, value)?;
    };

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
  const MEMORY_SIZE : usize = 0x1000;
  const REGISTER_COUNT : usize = 16;
  const TIMER_COUNT : usize = 2;
  const LOAD_ADDR : usize = 0x200;

  fn load(input: &[u8], display: mpsc::Sender<DisplayUpdate>) -> Result<Self, ()> {
    if input.len() > 0x1000 - Machine::LOAD_ADDR {
      Err(())
    } else {
      let mut m = Machine::new(display);
      m.memory[Machine::LOAD_ADDR..][..input.len()].copy_from_slice(&input);
      Ok(m)
    }
  }

  fn new(display: mpsc::Sender<DisplayUpdate>) -> Self {
    Machine {
      memory:    [0; Machine::MEMORY_SIZE],
      registers: [0; 16],
      reg_i:     0,
      timers:    [0; 2],
      ip:        Machine::LOAD_ADDR as u16,
      stack:     vec![],
      display,
    }
  }

  fn run(&mut self, hz: u32) {
    let interval_ms = Duration::from_secs_f64(1.0 / (hz as f64));
    let mut last = Instant::now();

    loop {
      self.step();
      let remaining = interval_ms.saturating_sub(Instant::now() - last);
      if remaining.is_zero() {
        println!("Warning: unable to maintain CPU hz");
      }
      std::thread::sleep(remaining);
      last = Instant::now();
    };
  }

  fn eval_next(&mut self) {
    let hi    = self.memory[self.ip as usize];
    let lo    = self.memory[(self.ip + 1) as usize];
    let instr = (hi as u16) << 8 | (lo as u16);
    let rest  = instr & 0x0FFF;
    let nibbles = [
      (0xF000 & instr) >> 12,
      (0x0F00 & instr) >> 8,
      (0x00F0 & instr) >> 4,
      (0x000F & instr) >> 0
    ];
    println!("{:#02x}", instr);
    match nibbles {
      [0x0, 0x0, 0xE, 0x0] => {
        // CLS
        self.ip += 2;
      },
      // [0x0, 0x0, 0xE, 0xE] => {
      //   // RET
      // },
      [0x1, _, _, _] => {
        // JP
        self.ip = rest;
      },
      [0x2, _, _, _] => {
        // CALL
        self.stack.push(self.ip as u16);
        self.ip = rest;
      },
      [0x3, x, _, _] => {
        // SE
        self.ip += if self.registers[x as usize] == lo {
          4
        } else {
          2
        };
      },
      [0x4, x, _, _] => {
        // SNE
        self.ip += if self.registers[x as usize] != lo {
          4
        } else {
          2
        };
      },
      [0x5, x, y, 0] => {
        // SE
        self.ip += if self.registers[x as usize] == self.registers[y as usize] {
          4
        } else {
          2
        };
      },
      [0x6, x, _, _] => {
        // LD
        self.registers[x as usize] = lo;
        self.ip += 2;
      },
      [0x7, x, _, _] => {
        self.registers[x as usize] += lo;
        self.ip += 2;
      },
      [0x8, x, y, 0] => {
        self.registers[x as usize] = self.registers[y as usize];
        self.ip += 2;
      },
      [0x8, x, y, 1] => {
        self.registers[x as usize] |= self.registers[y as usize];
        self.ip += 2;
      },
      [0x8, x, y, 2] => {
        self.registers[x as usize] &= self.registers[y as usize];
        self.ip += 2;
      },
      [0x8, x, y, 3] => {
        self.registers[x as usize] ^= self.registers[y as usize];
        self.ip += 2;
      },
      [0x8, x, y, 4] => {
        let (v, carry) = self.registers[x as usize].overflowing_add(self.registers[y as usize]);
        self.registers[x as usize] = v;
        self.registers[0xf] = if carry { 1 } else {  0 };
        self.ip += 2;
      },
      [0x8, x, y, 5] => {
        let (v, carry) = self.registers[x as usize].overflowing_sub(self.registers[y as usize]);
        self.registers[x as usize] = v;
        self.registers[0xf] = if carry { 1 } else {  0 };
        self.ip += 2;
      },
      [0x8, x, _, 6] => {
        let (v, carry) = self.registers[x as usize].overflowing_shr(1);
        self.registers[x as usize] = v;
        self.registers[0xf] = if carry { 1 } else {  0 };
        self.ip += 2;
      },
      [0x8, x, y, 7] => {
        let (v, carry) = self.registers[y as usize].overflowing_sub(self.registers[x as usize]);
        self.registers[x as usize] = v;
        self.registers[0xf] = if carry { 1 } else {  0 };
        self.ip += 2;
      },
      [0x8, x, _, 0xE] => {
        let (v, carry) = self.registers[x as usize].overflowing_shl(1);
        self.registers[x as usize] = v;
        self.registers[0xf] = if carry { 1 } else {  0 };
        self.ip += 2;
      },
      [0x9, x, y, 0x0] => {
        // SNE
        self.ip += if self.registers[x as usize] != self.registers[y as usize] {
          4
        } else {
          2
        };
      },
      [0xA, _, _, _] => {
        self.reg_i = rest;
        self.ip += 2;
      },
      [0xB, _, _, _] => {
        self.ip = rest + (self.registers[0] as u16);
      },
      [0xC, x, _, _] => {
        self.registers[x as usize] = fastrand::u8(..) & lo;
      },
      [0xD, x, y, n] => {
        println!("draw instr");
        let bytes = &self.memory[(self.reg_i as usize)..][..(n as usize)];
        let coords = (self.registers[x as usize] as usize, self.registers[y as usize] as usize);
        let payload = DisplayUpdate {
          bytes: bytes.to_vec(),
          coords,
        };
        self.display.send(payload).expect("Disconnected from display!");
        self.ip += 2;
      },
      [_, _, _, _] => {
        panic!("unimplemented instruction: {:#02x}", instr);
      },
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let input = env::args().skip(1).next();
  let mut buf = vec![];

  match input {
    None => {
      println!("Usage: ./chippy <file.ch8>");
      return Ok(());
    },
    Some(s) => {
      let mut f = File::open(&s)?;
      f.read_to_end(&mut buf)?;
    }
  }

  let (disp_tx, disp_rx) = mpsc::channel::<DisplayUpdate>();
  let (input_tx, input_rx) = mpsc::channel::<DisplayUpdate>();

  thread::spawn(move || {
    let mut m = Machine::load(&buf, disp_tx).expect("Failed to load");
    println!("{}", m);
    m.run(500);
  });

  let event_loop = EventLoop::new();
  let mut input = WinitInputHelper::new();
  let window = {
    let size = LogicalSize::new(64.0 * 8.0, 32.0 * 8.0);
    WindowBuilder::new()
      .with_title("Hello Pixels")
      .with_inner_size(size)
      .with_min_inner_size(size)
      .build(&event_loop)
      .unwrap()
  };

  let mut pixels = {
    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    Pixels::new(64, 32, surface_texture)?
  };

  let mut screen = [[0; 64]; 32];

  event_loop.run(move |event, _, control_flow| {
    // Update internal state and request a redraw
    while let Ok(msg) = disp_rx.try_recv() {
      let (x, mut y) = msg.coords;
      for byte in msg.bytes {
        for i in 0..8 {
          screen[y][x + i] ^= if byte & 1 << ((7 - i) as u8) != 0 { 1 } else { 0 };
        }
        y = (y + 1) % 64;
      }
    };

    // Draw the current frame
    if let Event::RedrawRequested(_) = event {
      for (i, pixel) in pixels.frame_mut().chunks_exact_mut(4).enumerate() {
        let x = (i % 64) as usize;
        let y = (i / 64) as usize;

        let color = if screen[y][x] != 0 {
          [0xFF, 0xFF, 0xFF, 0xFF]
        } else {
          [0x00, 0x00, 0x00, 0xFF]
        };

        pixel.copy_from_slice(&color);
      }

      if pixels.render().is_err() {
        *control_flow = ControlFlow::Exit;
        return;
      }
    }

    // Handle input events
    if input.update(&event) {
      // Close events
      if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
        *control_flow = ControlFlow::Exit;
        return;
      }

      // Resize the window
      if let Some(size) = input.window_resized() {
        if pixels.resize_surface(size.width, size.height).is_err() {
          *control_flow = ControlFlow::Exit;
          return;
        }
      }

      window.request_redraw();
    }
  });
}
