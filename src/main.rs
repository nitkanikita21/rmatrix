use crate::crossterm_ext::ColorExt;
use crossterm::cursor::{MoveDown, MoveLeft, MoveTo, SetCursorStyle};
use crossterm::style::{Color, Print, SetForegroundColor};
use crossterm::terminal::{size, Clear, ClearType};
use crossterm::{cursor, execute, queue, ExecutableCommand, QueueableCommand};
use rand::prelude::SliceRandom;
use rand::Rng;
use smart_default::SmartDefault;
use std::cell::OnceCell;
use std::io::{stdout, Write};
use std::ops::Range;
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::Duration;

mod crossterm_ext {
  use crossterm::style::Color;

  pub trait ColorExt {
    fn rgb(r: u8, g: u8, b: u8) -> Color;
  }

  impl ColorExt for Color {
    fn rgb(r: u8, g: u8, b: u8) -> Color {
      Color::Rgb { r, g, b }
    }
  }
}

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

fn get_all_unicode_chars() -> &'static [char] {
  static ALL_UNICODE_SYMBOLS: OnceLock<Vec<char>> = OnceLock::new();

  ALL_UNICODE_SYMBOLS.get_or_init(|| {
    (33..=0x7F_u32)
      .filter_map(|i| std::char::from_u32(i))
      .filter(|c| !c.is_whitespace())
      .collect()
  })
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
  speed: u8,
  y: u16,
  x: u16,
}

impl RainDrop {
  fn get_parts(&self) -> Box<[RainDropPart]> {
    let mut res: Vec<RainDropPart> = Vec::with_capacity(self.length as usize);

    match self.color {
      Color::Reset => {}
      Color::Rgb { r, g, b } => {
        let mut new_r = 0;
        let mut new_g = 0;
        let mut new_b = 0;

        let decrement_step_r = r / self.length;
        let decrement_step_g = g / self.length;
        let decrement_step_b = b / self.length;

        for i in 0..self.length {
          res.push(RainDropPart(
            self.get_char_for_part(i as usize),
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
      _ => {
        for i in 0..self.length {
          res.push(RainDropPart(self.get_char_for_part(i as usize), self.color));
        }
      }
    }

    res.push(RainDropPart(
      self.get_char_for_part(res.len()),
      Color::White,
    ));

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

  fn clear_tail(&self) -> anyhow::Result<()> {
    let mut stdout = stdout();
    for i in 0..self.speed {
      queue!(
        stdout,
        MoveTo(self.x, self.y.saturating_sub(self.length as u16 + i as u16)),
        Print(" ")
      )?;
    }
    Ok(())
  }

  fn is_end(&self) -> anyhow::Result<bool> {
    let (_, buffer_h) = size()?;
    Ok((self.y.saturating_sub(self.length as u16)) > buffer_h)
  }

  fn fall(&mut self) {
    self.y = self.y + self.speed as u16
  }

  fn get_char_for_part(&self, i: usize) -> char {
    let hash = self as *const Self as usize * 31 + (self.y as usize + i) * 31;

    let all = get_all_unicode_chars();
    all[hash % all.len()]
  }

  fn new(length: u8, color: Color, x: u16) -> Self {
    let mut rng = rand::thread_rng();
    Self {
      length,
      color,
      x,
      y: rng.gen_range(1..8),
      speed: rng.gen_range(1..3),
    }
  }
}

enum RainStyle {
  Solid(Color),
  Rainbow,
}

struct Rain {
  drops_count: usize,
  drop_length_range: Range<u8>,
  frame_delay: Duration,
  style: RainStyle,

  drops: Vec<RainDrop>,
}

impl Rain {
  fn new(
    drops_count: usize,
    drop_length: Range<u8>,
    style: RainStyle,
    frame_delay: Option<Duration>,
  ) -> anyhow::Result<Self> {
    let mut s = Self {
      drops_count,
      drop_length_range: drop_length,
      style,
      frame_delay: frame_delay.unwrap_or(Duration::from_millis(150)),
      drops: Vec::with_capacity(drops_count),
    };

    for _ in 0..drops_count {
      s.add_new_drop()?;
    }

    Ok(s)
  }

  fn add_new_drop(&mut self) -> anyhow::Result<()> {
    let (buffer_w, _) = size()?;
    let mut rng = rand::thread_rng();

    let len = rng.gen_range(self.drop_length_range.clone());
    let x = rng.gen_range(0..buffer_w);

    self.drops.push(match self.style {
      RainStyle::Solid(color) => RainDrop::new(len, color, x),
      RainStyle::Rainbow => RainDrop::new(
        len,
        Color::rgb(
          rng.gen_range(0..255),
          rng.gen_range(0..255),
          rng.gen_range(0..255),
        ),
        x,
      ),
    });

    Ok(())
  }

  fn draw(&mut self) -> anyhow::Result<()> {
    let mut stdout = stdout();
    execute!(stdout, Clear(ClearType::All), cursor::Hide, MoveTo(0, 0))?;

    loop {
      for i in 0..self.drops.len() {
        self.drops[i].draw()?;
        self.drops[i].fall();
        self.drops[i].clear_tail();

        if self.drops[i].is_end()? {
          self.drops.swap_remove(i);
          self.add_new_drop()?;
        }
      }
      stdout.flush()?;

      sleep(self.frame_delay)
    }
  }
}

fn main() -> anyhow::Result<()> {
  let mut rain = Rain::new(
    80,
    6..20,
    RainStyle::Rainbow,
    Some(Duration::from_millis(100)),
  )?;

  rain.draw()
}
