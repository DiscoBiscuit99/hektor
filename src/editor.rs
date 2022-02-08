use std::{
    fs::{ self, File },
    io::prelude::*,
    io::{ stdout, Write },
    collections::HashMap,
};

use crossterm::{
    cursor::{ self, position },
    event::{ 
        self, 
        DisableMouseCapture, 
        EnableMouseCapture,
        Event,
        KeyEvent,
        KeyCode,
    },
    terminal::{ 
        self, 
        enable_raw_mode,
        disable_raw_mode, 
    },
    execute,
    queue,
    QueueableCommand,
};

use structopt::StructOpt;

use crate::buffer::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "Hektor", about = "A(nother) minimalistic text editor.")]
pub struct Options {
    /// The name of the file you want to edit. 
    #[structopt(name = "FILE")]
    file_name: Option<String>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum InputMode {
    Normal,
    Insert,
    Command,
}

impl From<InputMode> for String {
    fn from(mode: InputMode) -> Self {
        match mode {
            InputMode::Normal => "Normal".to_string(),
            InputMode::Insert => "Insert".to_string(),
            InputMode::Command => "Command".to_string(),
        }
    }
}

/// The struct keeping track of the app state.
pub struct Hektor {
    input_mode: InputMode,
    active_buffer_id: usize,
    buffers: Vec<Buffer>,
    command_buffer: Buffer,
    command_queue: Vec<String>,
    status: String,
    should_quit: bool,
}

impl Hektor {
    /// Creates and returns the framework of the editor.
    pub fn new(options: Options) -> Self {
        let buffers = match options.file_name {
            Some(ref name) => {
                // TODO: if the file does not exist, create it...

                let mut file = File::open(name)
                    .expect("failed to open file.");

                let mut contents = String::new();
                file.read_to_string(&mut contents).expect("failed to read file to string");

                let mut lines = vec![];
                for line in contents.lines() {
                    lines.push(String::from(line));
                }

                vec![Buffer { 
                    name: name.to_string(),
                    lines, 
                    cursor: Cursor::default(),
                }]
            },
            None => vec![Buffer {
                lines: vec![ String::new() ],
                ..Default::default()
            }],
        };

        Self {
            input_mode: InputMode::Normal,
            active_buffer_id: 0,
            buffers,
            command_buffer: Buffer {
                name: "_command_buffer_".to_string(),
                lines: vec![ String::new() ],
                cursor: Cursor { 
                    col: 2, 
                    row: terminal::size().expect("failed to grab size of terminal").1,
                    desired_col: 2,
                },
                ..Default::default()
            },
            command_queue: vec![],
            status: "Normal".to_string(),
            should_quit: false,
        }
    }

    /// Runs the editor.
    pub fn run(&mut self) {
        initialize();

        // The run-loop.
        while !self.should_quit {
            self.render();
            self.handle_events();
            self.handle_command_queue();
        }

        clean_up();
    }

    /// Renders the current state of the editor.
    fn render(&self) {
        if self.input_mode == InputMode::Insert {
            execute!(stdout(), cursor::SetCursorShape(cursor::CursorShape::Line))
                .expect("failed to clear the terminal");
        } else {
            execute!(stdout(), cursor::SetCursorShape(cursor::CursorShape::Block))
                .expect("failed to clear the terminal");
        }

        //execute!(stdout(), terminal::Clear(terminal::ClearType::All))
            //.expect("failed to clear the terminal");

        self.clear_in_buffer();


        execute!(stdout(), cursor::MoveTo(0, 0))
            .expect("failed to move cursor");

        for line in &self.buffers[self.active_buffer_id].lines {
            print!("{}\n\r", line);
            stdout().flush();
        }

        self.render_status();
        self.render_command_query();
    }

    fn clear_in_buffer(&self) {
        for i in 0..self.buffers[self.active_buffer_id].lines.len() {
            execute!(stdout(), cursor::MoveTo(0, i as u16))
                .expect("failed to move cursor");
            execute!(stdout(), terminal::Clear(terminal::ClearType::CurrentLine))
                .expect("failed to clear the terminal");
        }
    }

    /// Renders the status line.
    fn render_status(&self) {
        let (_, height) = terminal::size()
            .expect("failed to grab size of terminal");

        execute!(stdout(), cursor::MoveTo(1, height))
            .expect("failed to move cursor");
        execute!(stdout(), terminal::Clear(terminal::ClearType::CurrentLine))
            .expect("failed to clear current line");

        print!("{}", self.status);
        stdout().flush();

        execute!(stdout(), 
            cursor::MoveTo(
                self.buffers[self.active_buffer_id].cursor.col, 
                self.buffers[self.active_buffer_id].cursor.row))
            .expect("failed to move cursor");
    }

    /// Renders the command_query (if it's active).
    fn render_command_query(&self) {
        if self.input_mode == InputMode::Command {
            let (_, height) = terminal::size()
                .expect("failed to grab size of terminal");

            execute!(stdout(), cursor::MoveTo(1, height))
                .expect("failed to move cursor");
            execute!(stdout(), terminal::Clear(terminal::ClearType::CurrentLine))
                .expect("failed to clear current line");

            print!(":{}", self.command_buffer.lines[0].trim());
            stdout().flush();
        }
    }

    /// Handles the key events.
    fn handle_events(&mut self) {
        // Blocking read.
        let event = event::read()
            .expect("failed to read input");

        match self.input_mode {
            InputMode::Normal => {
                match event {
                    Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                        self.status = self.input_mode.into();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char(':'), .. }) => {
                        self.input_mode = InputMode::Command;
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char('i'), .. }) => {
                        self.input_mode = InputMode::Insert;
                        self.status = self.input_mode.into();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char('I'), .. }) => {
                        self.input_mode = InputMode::Insert;
                        self.status = self.input_mode.into();
                        self.buffers[self.active_buffer_id].cursor_to_start();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char('a'), .. }) => {
                        self.input_mode = InputMode::Insert;
                        self.status = self.input_mode.into();
                        self.buffers[self.active_buffer_id].cursor_right();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char('A'), .. }) => {
                        self.input_mode = InputMode::Insert;
                        self.status = self.input_mode.into();
                        self.buffers[self.active_buffer_id].cursor_to_end();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char('o'), .. }) => {
                        self.buffers[self.active_buffer_id].insert_line();
                        self.buffers[self.active_buffer_id].cursor_down();
                        self.buffers[self.active_buffer_id].cursor_to_start();

                        self.input_mode = InputMode::Insert;
                        self.status = self.input_mode.into();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char('k'), .. }) => {
                        self.buffers[self.active_buffer_id].cursor_up();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char('j'), .. }) => {
                        self.buffers[self.active_buffer_id].cursor_down();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char('h'), .. }) => {
                        self.buffers[self.active_buffer_id].cursor_left();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char('l'), .. }) => {
                        self.buffers[self.active_buffer_id].cursor_right();
                    },
                    _ => {},
                }
            },
            InputMode::Insert => {
                match event {
                    Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                        self.input_mode = InputMode::Normal;
                        self.status = self.input_mode.into();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                        self.buffers[self.active_buffer_id].insert_line();
                        self.buffers[self.active_buffer_id].cursor_down();
                        self.buffers[self.active_buffer_id].cursor_to_start();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) => {
                        // remove the character before the cursor...
                        self.buffers[self.active_buffer_id].delete_char();
                        self.buffers[self.active_buffer_id].cursor_left();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char(c), .. }) => {
                        // insert `c` into the currently active buffer...
                        self.buffers[self.active_buffer_id].insert_char(c);
                        self.buffers[self.active_buffer_id].cursor_right();
                    },
                    _ => {},
                }
            },
            InputMode::Command => {
                match event {
                    Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                        self.input_mode = InputMode::Normal;
                        self.status = self.input_mode.into();
                        self.command_buffer.lines[0].clear();
                    },
                    Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                        // push the command(s) to the command queue...
                        let mut commands = self.command_buffer.lines[0]
                            .split_ascii_whitespace();
                        while let Some(command) = commands.next() {
                            self.command_queue.push(command.to_string());
                        }
                        self.command_buffer.lines[0].clear();
                        self.input_mode = InputMode::Normal;
                    },
                    Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) => {
                        if self.command_buffer.lines[0].len() > 0 {
                            self.command_buffer.lines[0].pop();
                            self.command_buffer.cursor_left();
                        }
                    },
                    Event::Key(KeyEvent { code: KeyCode::Char(c), .. }) => {
                        self.command_buffer.lines[0].push(c);
                        self.command_buffer.cursor_right();
                    },
                    _ => {},
                }
            },
        }
    }

    /// Executes the commands in the command queue in order.
    fn handle_command_queue(&mut self) {
        while self.command_queue.len() > 0 {
            let command = self.command_queue.remove(0);

            match command.as_ref() {
                "q" => self.should_quit = true,
                "w" => self.write_current_buffer(),
                _ => self.print_err("Unrecognized command..."),
            }
        }
    }

    fn write_current_buffer(&mut self) {
        Self::write_buffer(&self.buffers[self.active_buffer_id]);
    }

    fn write_buffer(buffer: &Buffer) {
        for line in &buffer.lines {
            fs::write(&buffer.name, line)
                .expect("Unable to write file");
        }
    }

    fn print_err(&self, msg: &str) {
        let (_, height) = terminal::size()
            .expect("failed to grab size of terminal");

        execute!(stdout(), cursor::MoveTo(1, height))
            .expect("failed to move cursor");
        execute!(stdout(), terminal::Clear(terminal::ClearType::CurrentLine))
            .expect("failed to clear current line");

        print!("{}", msg);
        stdout().flush();

        execute!(stdout(), 
            cursor::MoveTo(
                self.buffers[self.active_buffer_id].cursor.col, 
                self.buffers[self.active_buffer_id].cursor.row))
            .expect("failed to move cursor");
    }
}

/// Initializes the editor.
fn initialize() {
    execute!(stdout(), terminal::Clear(terminal::ClearType::All))
        .expect("failed to clear terminal");
    execute!(stdout(), cursor::MoveTo(0, 0))
        .expect("failed to move cursor");
    execute!(stdout(), terminal::EnterAlternateScreen)
        .expect("failed to enter alternate screen");
    execute!(stdout(), terminal::DisableLineWrap)
        .expect("failed to disable line wrap");

    enable_raw_mode()
        .expect("failed to enable raw mode");
}

/// Resets the terminal to normal before closing.
fn clean_up() {
    execute!(stdout(), terminal::LeaveAlternateScreen)
        .expect("failed to leave alternate screen");
    execute!(stdout(), terminal::EnableLineWrap)
        .expect("failed to enable line wrap");

    disable_raw_mode()
        .expect("failed to disable raw mode");
}

