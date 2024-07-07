use std::{
    error::Error,
    io::{stdout, Write},
    u16,
};

use crossterm::{
    cursor,
    event::{self, read},
    style::{self, Stylize},
    terminal, ExecutableCommand, QueueableCommand,
};

enum Action {
    Quit,

    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,

    Char(char),

    EnterMode(Mode),
}

#[derive(Debug)]
enum Mode {
    Normal,
    Insert,
}

struct TextEditor {
    stdout: std::io::Stdout,
    size: (u16, u16),
    cx: u16,
    cy: u16,
    mode: Mode,
}

impl Drop for TextEditor {
    fn drop(&mut self) {
        self.stdout.execute(terminal::LeaveAlternateScreen).unwrap();
        terminal::disable_raw_mode().unwrap();
    }
}

impl TextEditor {
    pub fn new() -> Self {
        let mut stdout = stdout();

        terminal::enable_raw_mode().unwrap();
        stdout
            .execute(terminal::EnterAlternateScreen)
            .unwrap()
            .execute(terminal::Clear(terminal::ClearType::All))
            .unwrap();

        TextEditor {
            stdout,
            cx: 0,
            cy: 0,
            mode: Mode::Normal,
            size: terminal::size().unwrap(),
        }
    }

    pub fn draw(&mut self) -> Result<(), Box<dyn Error>> {
        _ = self.statusline()?;
        self.stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
        self.stdout.flush()?;

        Ok(())
    }

    pub fn statusline(&mut self) -> Result<(), Box<dyn Error>> {
        let mode = format!(" {:?} ", self.mode);

        let cposition = format!(" {}:{} ", self.cy, self.cx);

        self.stdout.queue(cursor::MoveTo(0, self.size.1 - 1))?;
        self.stdout.queue(style::PrintStyledContent(
            mode.to_uppercase()
                .bold()
                .with(style::Color::Rgb { r: 0, g: 0, b: 0 })
                .on(style::Color::Rgb {
                    r: 184,
                    g: 144,
                    b: 243,
                }),
        ))?;
        self.stdout.queue(style::PrintStyledContent(
            format!(
                "{:^width$}",
                "",
                width = (self.size.0 - cposition.len() as u16 - mode.len() as u16) as usize
            )
            .on(style::Color::Rgb {
                r: 37,
                g: 37,
                b: 37,
            }),
        ))?;

        self.stdout.queue(style::PrintStyledContent(
            cposition
                .bold()
                .with(style::Color::Rgb { r: 0, g: 0, b: 0 })
                .on(style::Color::Rgb {
                    r: 184,
                    g: 144,
                    b: 243,
                }),
        ))?;

        self.stdout.flush()?;

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            self.draw()?;
            if let Some(action) = self.handle_event(read()?)? {
                match action {
                    Action::Quit => break,
                    Action::MoveUp => {
                        self.cy = self.cy.saturating_sub(1);
                    }
                    Action::MoveDown => {
                        self.cy += 1u16;
                    }
                    Action::MoveLeft => {
                        self.cx = self.cx.saturating_sub(1);
                    }
                    Action::MoveRight => {
                        self.cx += 1u16;
                    }
                    Action::EnterMode(mode) => {
                        self.mode = mode;
                    }
                    Action::Char(c) => {
                        self.stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
                        self.stdout.queue(style::Print(c))?;
                        self.cx += 1;
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, e: event::Event) -> Result<Option<Action>, Box<dyn Error>> {
        if matches!(e, event::Event::Resize(_, _)) {
            self.size = terminal::size()?
        }

        match self.mode {
            Mode::Normal => self.handle_normal_event(e),
            Mode::Insert => self.handle_insert_event(e),
        }
    }

    fn handle_normal_event(&self, e: event::Event) -> Result<Option<Action>, Box<dyn Error>> {
        let action = match e {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Char('q') => Some(Action::Quit),
                event::KeyCode::Up => Some(Action::MoveUp),
                event::KeyCode::Down => Some(Action::MoveDown),
                event::KeyCode::Left => Some(Action::MoveLeft),
                event::KeyCode::Right => Some(Action::MoveRight),
                event::KeyCode::Char('i') => Some(Action::EnterMode(Mode::Insert)),
                _ => None,
            },
            _ => None,
        };

        Ok(action)
    }

    fn handle_insert_event(&self, e: event::Event) -> Result<Option<Action>, Box<dyn Error>> {
        let action = match e {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Esc => Some(Action::EnterMode(Mode::Normal)),
                event::KeyCode::Char(c) => Some(Action::Char(c)),
                _ => None,
            },
            _ => None,
        };

        Ok(action)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut editor = TextEditor::new();
    _ = editor.run();
    Ok(())
}
