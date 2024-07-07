use std::{
    error::Error,
    io::{stdout, Write},
};

use crossterm::{
    cursor,
    event::{self, read},
    style, terminal, ExecutableCommand, QueueableCommand,
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

enum Mode {
    Normal,
    Insert,
}

struct Editor {
    cx: u16,
    cy: u16,
    mode: Mode,
}

impl Editor {
    pub fn new() -> Self {
        Editor {
            cx: 0,
            cy: 0,
            mode: Mode::Normal,
        }
    }

    pub fn draw(&self, stdout: &mut std::io::Stdout) -> Result<(), Box<dyn Error>> {
        stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
        stdout.flush()?;

        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut stdout = stdout();

        terminal::enable_raw_mode()?;
        stdout
            .execute(terminal::EnterAlternateScreen)?
            .execute(terminal::Clear(terminal::ClearType::All))?;

        loop {
            self.draw(&mut stdout)?;
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
                    Action::EnterMode(nmode) => {
                        self.mode = nmode;
                    }
                    Action::Char(c) => {
                        stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
                        stdout.queue(style::Print(c))?;
                        self.cx += 1;
                    }
                }
            }
        }

        stdout.execute(terminal::LeaveAlternateScreen)?;
        terminal::disable_raw_mode()?;

        Ok(())
    }

    fn handle_event(&mut self, e: event::Event) -> Result<Option<Action>, Box<dyn Error>> {
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
    let mut editor = Editor::new();
    _ = editor.run();
    Ok(())
}
