use crossterm::terminal;

#[derive(Default, Debug)]
pub struct Cursor {
    pub col: u16,
    pub row: u16,
    pub desired_col: u16,
}

#[derive(Default, Debug)]
pub struct Buffer {
    pub name: String,
    pub lines: Vec<String>,
    pub cursor: Cursor,
}

impl Buffer {
    pub fn cursor_up(&mut self) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.cursor.col = self.cursor.desired_col;
            self.clamp_cursor();
        }
    }

    pub fn cursor_down(&mut self) {
        self.cursor.row += 1;
        self.cursor.col = self.cursor.desired_col;
        self.clamp_cursor();
    }

    pub fn cursor_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
            self.cursor.desired_col = self.cursor.col;
            self.clamp_cursor();
        }
    }

    pub fn cursor_right(&mut self) {
        self.cursor.col += 1;
        self.cursor.desired_col = self.cursor.col;
        self.clamp_cursor();
    }

    pub fn cursor_to_start(&mut self) {
        self.cursor.col = 0;
    }

    pub fn cursor_to_end(&mut self) {
        self.cursor.col = self.lines[self.cursor.row as usize].len() as u16;
    }

    pub fn insert_char(&mut self, c: char) {
        self.lines[self.cursor.row as usize].insert(self.cursor.col as usize, c);
    }

    pub fn delete_char(&mut self) {
        if self.cursor.col > 0 && (self.cursor.col as usize) <= self.lines[self.cursor.row as usize].len()  {
            self.lines[self.cursor.row as usize].remove(self.cursor.col as usize - 1);
        }
    }

    pub fn insert_line(&mut self) {
        self.lines.insert(self.cursor.row as usize + 1, String::new());
    }

    fn clamp_cursor(&mut self) {
        let (width, height) = terminal::size()
            .expect("failed to grab terminal size.");

        if self.cursor.col > width {
            self.cursor.col = width;
        } else if self.cursor.row > height - 2 {
            self.cursor.row = height - 2;
        }

        self.cursor.row = self.cursor.row.clamp(0, self.lines.len() as u16 - 1);
        self.cursor.col = self.cursor.col.clamp(0, self.lines[self.cursor.row as usize].len() as u16);
    }
}

