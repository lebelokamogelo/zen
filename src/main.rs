use std::{
    error::Error,
    io::{stdout, Write},
    usize,
};

use crossterm::{
    cursor,
    event::{self, read},
    style::{self, Stylize},
    terminal, ExecutableCommand, QueueableCommand,
};

enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,

    Insert(char),
    InsertLineBelow,
    InsertLineAbove,

    DeleteLine,
    DeleteChar,

    InsertChar(u16, u16, char),
    InsertLine(u16, String),

    Undo,

    EnterMode(Mode),

    MoveEnd,
    MoveHome,

    PageDown,
    PageUp,

    Quit,
}

#[derive(Debug)]
enum Mode {
    Normal,
    Insert,
}

struct TextEditor {
    stdout: std::io::Stdout,
    buffer: Buffer,
    size: (u16, u16),
    cx: u16,
    cy: u16,
    mode: Mode,
    sv: usize,
    command_wait: Option<char>,
    undo: Vec<Action>,
}

impl Drop for TextEditor {
    fn drop(&mut self) {
        self.stdout.execute(terminal::LeaveAlternateScreen).unwrap();
        terminal::disable_raw_mode().unwrap();
    }
}

impl TextEditor {
    pub fn new(buffer: Buffer) -> Self {
        let mut stdout = stdout();

        terminal::enable_raw_mode().unwrap();
        stdout
            .execute(terminal::EnterAlternateScreen)
            .unwrap()
            .execute(terminal::Clear(terminal::ClearType::All))
            .unwrap();

        TextEditor {
            stdout,
            buffer,
            cx: 0,
            cy: 0,
            mode: Mode::Normal,
            size: terminal::size().unwrap(),
            sv: 0,
            command_wait: None,
            undo: vec![],
        }
    }

    pub fn draw(&mut self) -> Result<(), Box<dyn Error>> {
        _ = self.draw_buffer();
        _ = self.statusline()?;
        self.stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
        self.stdout.flush()?;

        Ok(())
    }

    fn draw_buffer(&mut self) {
        _ = stdout().execute(terminal::Clear(terminal::ClearType::All));

        for i in 0..self.buffer.lines.len() as u16 {
            self.stdout.queue(cursor::MoveTo(0, i)).unwrap();
            self.stdout
                .queue(style::Print(format!(
                    "{:<width$}",
                    self.buffer
                        .get(i as usize + self.sv)
                        .unwrap_or("".to_string()),
                    width = self.size.0 as usize
                )))
                .unwrap();
        }
    }

    pub fn statusline(&mut self) -> Result<(), Box<dyn Error>> {
        let mode = format!(" {:?} ", self.mode);
        let cpos = format!(" {}:{}", self.cy + self.sv as u16, self.cx);

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
                "{:width$}",
                format!(" {}", self.buffer.file),
                width = (self.size.0 - cpos.len() as u16 - mode.len() as u16) as usize
            )
            .on(style::Color::Rgb {
                r: 37,
                g: 37,
                b: 37,
            }),
        ))?;

        self.stdout.queue(style::PrintStyledContent(
            cpos.bold()
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

    fn current_line_len(&self) -> u16 {
        self.buffer
            .get(self.cy as usize + self.sv)
            .map_or(0, |s| s.len() as u16)
    }

    fn bounds(&mut self) {
        self.cx = self.cx.min(self.current_line_len());

        if self.sv + self.cy as usize >= self.buffer.lines.len() {
            if self.buffer.lines.len() > 0 {
                self.cy = self.buffer.lines.len() as u16 - self.sv as u16 - 1;
            } else {
                self.cy = self.buffer.lines.len() as u16 - self.sv as u16;
            }
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            self.bounds();
            self.draw()?;
            if let Some(action) = self.handle_event(read()?)? {
                match action {
                    Action::Quit => break,
                    Action::MoveUp => {
                        if self.cy == 0 {
                            if self.sv > 0 {
                                self.sv -= 1;
                            }
                        } else {
                            self.cy = self.cy.saturating_sub(1);
                        }
                    }
                    Action::MoveDown => {
                        if self.buffer.lines.len() as u16 > self.cy + self.sv as u16 {
                            self.cy += 1;
                        }
                        if self.cy >= self.size.1 - 1 {
                            self.cy -= 1;
                            self.sv += 1;
                        }
                    }
                    Action::MoveLeft => {
                        self.cx = self.cx.saturating_sub(1);
                    }
                    Action::MoveRight => {
                        self.cx += 1;
                    }

                    Action::PageUp => self.cy = 0,
                    Action::PageDown => self.cy = self.size.1 - 2,
                    Action::EnterMode(mode) => {
                        self.mode = mode;
                    }
                    Action::MoveHome => self.cx = 0,
                    Action::MoveEnd => self.cx = self.current_line_len(),
                    Action::Insert(c) => {
                        self.stdout.queue(cursor::MoveTo(self.cx, self.cy))?;
                        self.buffer.insert(self.cx, self.cy + self.sv as u16, c);
                        self.cx += 1;
                    }

                    Action::InsertLineBelow => {
                        let line = self.cy as usize + self.sv + 1;
                        self.buffer.lines.insert(line, String::new());

                        self.cy = line as u16;
                    }

                    Action::InsertLineAbove => {
                        let line = self.cy as usize + self.sv;
                        self.buffer.lines.insert(line, String::new());
                    }

                    Action::DeleteChar => {
                        let line = self.cy + self.sv as u16;
                        let line_str = self.buffer.get(line as usize).unwrap();

                        if line_str.len() > 0 {
                            self.buffer.remove(self.cx, line);
                            self.undo.push(Action::InsertChar(
                                self.cx,
                                line,
                                line_str.chars().nth(self.cx as usize).unwrap(),
                            ))
                        }
                    }

                    Action::DeleteLine => {
                        let line = self.cy + self.sv as u16;
                        let line_str = self.buffer.get(line as usize).unwrap();
                        match self.command_wait {
                            Some(command) => match command {
                                'd' => {
                                    if self.buffer.lines.len() > 0 {
                                        self.buffer.lines.remove(line as usize);
                                        self.undo.push(Action::InsertLine(line, line_str));
                                        self.command_wait = None
                                    }
                                }
                                _ => {}
                            },
                            None => self.command_wait = Some('d'),
                        }
                    }

                    Action::Undo => match self.undo.pop() {
                        Some(Action::InsertLine(line, content)) => {
                            self.buffer.insert_line(line as usize, content)
                        }
                        Some(Action::InsertChar(x, y, c)) => self.buffer.insert(x, y, c),
                        _ => {}
                    },

                    _ => {}
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

    fn handle_normal_event(&mut self, e: event::Event) -> Result<Option<Action>, Box<dyn Error>> {
        let action = match e {
            event::Event::Key(event) => match event.code {
                event::KeyCode::Char('q') => Some(Action::Quit),
                event::KeyCode::Up => Some(Action::MoveUp),
                event::KeyCode::Down => Some(Action::MoveDown),
                event::KeyCode::Left => Some(Action::MoveLeft),
                event::KeyCode::Right => Some(Action::MoveRight),
                event::KeyCode::Char('i') => Some(Action::EnterMode(Mode::Insert)),
                event::KeyCode::Char('b') => Some(Action::PageUp),
                event::KeyCode::Char('f') => Some(Action::PageDown),
                event::KeyCode::Char('0') => Some(Action::MoveHome),
                event::KeyCode::Char('$') => Some(Action::MoveEnd),
                event::KeyCode::Char('d') => Some(Action::DeleteLine),
                event::KeyCode::Char('x') => Some(Action::DeleteChar),
                event::KeyCode::Char('u') => Some(Action::Undo),
                event::KeyCode::Char('o') => Some(Action::InsertLineBelow),
                event::KeyCode::Char('O') => Some(Action::InsertLineAbove),
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

                event::KeyCode::Char(c) => Some(Action::Insert(c)),
                _ => None,
            },
            _ => None,
        };

        Ok(action)
    }
}

struct Buffer {
    file: String,
    lines: Vec<String>,
}

impl Buffer {
    fn new(file: String) -> Self {
        let lines = std::fs::read_to_string(file.clone())
            .unwrap_or_default()
            .lines()
            .map(|line| line.to_string())
            .collect();

        Self { file, lines }
    }

    fn get(&self, line: usize) -> Option<String> {
        if self.lines.len() >= line + 1 {
            return Some(self.lines[line].clone());
        }
        None
    }

    fn insert(&mut self, x: u16, y: u16, c: char) {
        if self.lines.len() == y as usize {
            self.lines.resize(y as usize + 1, String::new());
        }

        if let Some(line) = self.lines.get_mut(y as usize) {
            line.insert(x as usize, c);
        }
    }

    fn insert_line(&mut self, index: usize, content: String) {
        self.lines.insert(index, content)
    }

    fn remove(&mut self, x: u16, y: u16) {
        if let Some(line) = self.lines.get_mut(y as usize) {
            line.remove(x as usize);
        }
    }

    fn remove_line(&mut self, y: u16) {
        self.lines.remove(y as usize);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let file = std::env::args().nth(1);

    let mut editor = TextEditor::new(Buffer::new(file.unwrap_or("Empty".to_string())));
    _ = editor.run();
    Ok(())
}
