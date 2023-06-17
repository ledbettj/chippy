pub struct Screen {
  display: [[u8; Screen::WIDTH]; Screen::HEIGHT],
}

pub enum ScreenUpdate {
  Draw {
    bytes: Vec<u8>,
    coords: (usize, usize),
  },
  Clear,
}

impl Screen {
  pub const WIDTH: usize = 64;
  pub const HEIGHT: usize = 32;
  pub const COLOR: [u8; 4] = [0x00, 0xCC, 0x00, 0xFF];

  pub fn new() -> Self {
    Self {
      display: [[0; Screen::WIDTH]; Screen::HEIGHT],
    }
  }

  pub fn update(&mut self, msg: &ScreenUpdate) -> Option<bool> {
    match msg {
      ScreenUpdate::Draw { coords, bytes } => {
        let mut col = false;
        let (x, mut y) = coords;
        for byte in bytes {
          for i in 0..8 {
            let index = (x + i) % Screen::WIDTH;
            let curr = self.display[y][index];
            let bit = (byte >> (7 - i)) & 1;
            let next = curr ^ bit;
            self.display[y][index] = next;
            // if the pixel was on and is now off, signal a collision
            col |= curr != next && next == 0;
          }
          y = (y + 1) % Screen::WIDTH;
        }
        Some(col)
      }
      ScreenUpdate::Clear => {
        self.display = [[0; Screen::WIDTH]; Screen::HEIGHT];
        None
      }
    }
  }

  pub fn draw(&self, frame: &mut [u8]) {
    for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
      let x = (i % Screen::WIDTH) as usize;
      let y = (i / Screen::WIDTH) as usize;

      let color = if self.display[y][x] != 0 {
        Screen::COLOR
      } else {
        [0x00, 0x00, 0x00, 0xFF]
      };

      pixel.copy_from_slice(&color);
    }
  }
}
