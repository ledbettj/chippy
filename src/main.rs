use std::env;
use std::fs::File;
use std::io::Read;

use std::sync::mpsc;
use std::thread;

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

mod instruction;
mod machine;
mod screen;

use machine::Machine;
use screen::{Screen, ScreenUpdate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let input = env::args().skip(1).next();
  let mut buf = vec![];

  match input {
    None => {
      println!("Usage: ./chippy <file.ch8>");
      return Ok(());
    }
    Some(s) => {
      let mut f = File::open(&s)?;
      f.read_to_end(&mut buf)?;
    }
  }

  let (disp_tx, disp_rx) = mpsc::channel::<ScreenUpdate>();
  let (col_tx, col_rx) = mpsc::channel::<bool>();
  let (key_tx, key_rx) = mpsc::channel::<u16>();

  //let (input_tx, input_rx) = mpsc::channel::<ScreenUpdate>();

  thread::spawn(move || {
    let mut m = Machine::load(&buf, disp_tx, col_rx, key_rx).expect("Failed to load");
    println!("{}", m);
    m.run(500).expect("Panic");
  });

  let event_loop = EventLoop::new();
  let mut input = WinitInputHelper::new();
  let window = {
    let size = LogicalSize::new(Screen::WIDTH as f64 * 16.0, Screen::HEIGHT as f64 * 16.0);
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

  let mut screen = Screen::new();
  let mut last_mask: u16 = 0;

  event_loop.run(move |event, _, control_flow| {
    // Update internal state and request a redraw
    while let Ok(msg) = disp_rx.try_recv() {
      if let Some(col) = screen.update(&msg) {
        col_tx.send(col).expect("Machine disconnected");
      }
    }

    // Draw the current frame
    if let Event::RedrawRequested(_) = event {
      screen.draw(pixels.frame_mut());

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

      let keys = [
        VirtualKeyCode::Key1,
        VirtualKeyCode::Key2,
        VirtualKeyCode::Key3,
        VirtualKeyCode::Key4,
        VirtualKeyCode::Q,
        VirtualKeyCode::W,
        VirtualKeyCode::E,
        VirtualKeyCode::R,
        VirtualKeyCode::W,
        VirtualKeyCode::S,
        VirtualKeyCode::D,
        VirtualKeyCode::F,
        VirtualKeyCode::Z,
        VirtualKeyCode::X,
        VirtualKeyCode::C,
        VirtualKeyCode::V,
      ];
      let mut mask: u16 = 0;
      for (i, &k) in keys.iter().enumerate() {
        if input.key_held(k) {
          mask = (i + 1) as u16;
        }
      }
      if mask != last_mask {
        key_tx.send(mask).expect("Machine disconnected");
        last_mask = mask;
      }

      window.request_redraw();
    }
  });
}
