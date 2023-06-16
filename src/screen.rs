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
            // TODO: check for erasure
            let index = (x + i) % Screen::WIDTH;
            let curr = self.display[y][index];
            let bit = if byte & 1 << ((7 - i) as u8) != 0 {
              1
            } else {
              0
            };
            let next = curr ^ bit;
            self.display[y][index] = next;
            col |= curr != next && next == 0;
          }
          y = (y + 1) % 64;
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
      let x = (i % 64) as usize;
      let y = (i / 64) as usize;

      let color = if self.display[y][x] != 0 {
        Screen::COLOR
      } else {
        [0x00, 0x00, 0x00, 0xFF]
      };

      pixel.copy_from_slice(&color);
    }
  }
}
