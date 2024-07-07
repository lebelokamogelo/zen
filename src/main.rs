use std::{
    error::Error,
    io::{self, stdout, Write},
};

use crossterm::{
    cursor,
    event::{self, read},
    style, terminal, ExecutableCommand, QueueableCommand,
};

enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,

    Quit,

    EnterMode(Mode),
}

enum Mode {
    Normal,
    Insert,
}

fn handle_event(
    cx: &mut u16,
    stdout: &mut io::Stdout,
    mode: &Mode,
    e: event::Event,
) -> Result<Option<Action>, Box<dyn Error>> {
    match mode {
        Mode::Normal => handle_normal_mode(e),
        Mode::Insert => handle_insert_mode(cx, stdout, e),
    }
}

fn handle_normal_mode(e: event::Event) -> Result<Option<Action>, Box<dyn Error>> {
    match e {
        event::Event::Key(event) => match event.code {
            event::KeyCode::Char('q') => Ok(Some(Action::Quit)),
            event::KeyCode::Up => Ok(Some(Action::MoveUp)),
            event::KeyCode::Down => Ok(Some(Action::MoveDown)),
            event::KeyCode::Left => Ok(Some(Action::MoveLeft)),
            event::KeyCode::Right => Ok(Some(Action::MoveRight)),
            event::KeyCode::Char('i') => Ok(Some(Action::EnterMode(Mode::Insert))),

            _ => Ok(None),
        },
        _ => Ok(None),
    }
}

fn handle_insert_mode(
    cx: &mut u16,
    stdout: &mut io::Stdout,
    e: event::Event,
) -> Result<Option<Action>, Box<dyn Error>> {
    match e {
        event::Event::Key(event) => match event.code {
            event::KeyCode::Esc => Ok(Some(Action::EnterMode(Mode::Normal))),
            event::KeyCode::Char(c) => {
                stdout.queue(style::Print(c))?;
                *cx += 1u16;
                Ok(None)
            }
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut stdout = stdout();
    let mut mode = Mode::Normal;

    // cursor coordinates
    let mut cx = 0;
    let mut cy = 0;

    terminal::enable_raw_mode().unwrap();
    _ = stdout.execute(terminal::EnterAlternateScreen);

    _ = stdout.execute(terminal::Clear(terminal::ClearType::All));

    loop {
        _ = stdout.queue(cursor::MoveTo(cx, cy));
        _ = stdout.flush();

        if let Some(action) = handle_event(&mut cx, &mut stdout, &mode, read()?).unwrap() {
            match action {
                Action::Quit => break,
                Action::MoveUp => {
                    cy = cy.saturating_sub(1);
                }
                Action::MoveDown => {
                    cy += 1u16;
                }
                Action::MoveLeft => {
                    cx = cx.saturating_sub(1);
                }
                Action::MoveRight => {
                    cx += 1u16;
                }
                Action::EnterMode(nmode) => {
                    mode = nmode;
                }

                _ => {}
            }
        };
    }

    _ = stdout.execute(terminal::LeaveAlternateScreen);
    terminal::disable_raw_mode().unwrap();

    Ok(())
}
