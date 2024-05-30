use crossterm::cursor::{MoveDown, MoveLeft, MoveTo, SetCursorStyle};
use crossterm::style::{Color, Print, SetForegroundColor};
use crossterm::terminal::{size, Clear, ClearType};
use crossterm::{cursor, execute, queue, ExecutableCommand, QueueableCommand};
use rand::prelude::SliceRandom;
use rand::Rng;
use std::io::{stdout, Write};
use std::thread::sleep;
use std::time::Duration;

fn get_random_char() -> char {
  let mut rng = rand::thread_rng();

  // Define the ASCII symbol ranges
  let ranges = [
    33..=47,   // ! " # $ % & ' ( ) * + , - . /
    58..=64,   // : ; < = > ? @
    65..=90,   // A-Z
    91..=96,   // [ \ ] ^ _ `
    97..=122,  // a-z
    123..=126, // { | } ~
  ];

  // Generate a random symbol within the selected ranges
  let random_symbol = ranges.choose(&mut rng).unwrap().clone();
  rng.gen_range(random_symbol) as u8 as char
}

struct RainDropPart(char, Color);

impl RainDropPart {
  fn draw(&self) -> anyhow::Result<()> {
    let mut stdout = stdout();

    queue!(stdout, SetForegroundColor(self.1), Print(self.0))?;

    Ok(())
  }
}

struct RainDrop {
  length: u8,
  color: Color,
  y: u16,
  x: u16,
}

impl RainDrop {
  fn get_parts(&self) -> Box<[RainDropPart]> {
    let mut res: Vec<RainDropPart> = Vec::with_capacity(self.length as usize);

    if let Color::Rgb { r, g, b } = self.color {
      let mut new_r = 0;
      let mut new_g = 0;
      let mut new_b = 0;

      let decrement_step_r = r / self.length;
      let decrement_step_g = g / self.length;
      let decrement_step_b = b / self.length;

      for _ in 0..self.length {
        res.push(RainDropPart(
          get_random_char(),
          Color::Rgb {
            r: new_r,
            g: new_g,
            b: new_b,
          },
        ));

        new_r = new_r.wrapping_add(decrement_step_r);
        new_g = new_g.wrapping_add(decrement_step_g);
        new_b = new_b.wrapping_add(decrement_step_b);
      }
    }
    res.push(RainDropPart(get_random_char(), Color::White));

    res.into_boxed_slice()
  }

  fn draw(&self) -> anyhow::Result<()> {
    let (_, buffer_h) = size()?;
    let mut stdout = stdout();

    for (i, part) in self.get_parts().into_iter().enumerate().filter(|(i, _)| {
      (0..buffer_h).contains(&(self.y + *i as u16).saturating_sub(self.length as u16))
    }) {
      queue!(
        stdout,
        MoveTo(
          self.x,
          (self.y + i as u16).saturating_sub(self.length as u16)
        )
      )?;

      part.draw()?
    }

    Ok(())
  }

  fn is_end(&self) -> anyhow::Result<bool> {
    let (_, buffer_h) = size()?;
    Ok((self.y.saturating_sub(self.length as u16)) > buffer_h)
  }

  fn fall(&mut self) {
    self.y = self.y + 1
  }

  fn new(length: u8, color: Color, x: u16) -> Self {
    Self {
      length,
      color,
      x,
      y: 0,
    }
  }
}

/*enum RainStyle {
  Monochrome(Color),
  Rgb(Color),


}

struct Rain {
  drops_count: usize,
  frame_delay: Duration
}
*/

fn main() -> anyhow::Result<()> {
  let mut stdout = stdout();
  execute!(stdout, Clear(ClearType::All), cursor::Hide, MoveTo(0, 0))?;

  let (buffer_w, buffer_h) = size()?;

  /*let mut rain_drop = RainDrop::new(12, Color::Rgb { r: 0, g: 255, b: 0 }, 0);

  loop {
    rain_drop.draw()?;
    rain_drop.fall();

    if rain_drop.is_end()? {

    }

    sleep(Duration::from_millis(150));
  }*/

  let mut rain_drops: Vec<RainDrop> = (0..buffer_w)
    .filter( | i | i % 2 == 0)
    .map(|x| RainDrop::new(12, Color::Rgb { r: 0, g: 255, b: 0 }, x))
    .collect();

  loop {
    for i in (0..rain_drops.len()) {
      rain_drops[i].draw()?;
      rain_drops[i].fall();

      if rain_drops[i].is_end()? { rain_drops.swap_remove(i); }

    }
    stdout.flush()?;
  }

  Ok(())
}
