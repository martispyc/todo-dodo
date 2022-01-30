//importing n execute! macro
#[macro_use]
extern crate crossterm;
#[allow(unused_imports)]
use crossterm::{
    event::{read, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{Print, Color, Attribute, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
    cursor,
};
use std::{
    io::{self, stdout, BufRead, Stdout, Write, ErrorKind},
    time::Duration,
    fs::File,
    process,
    thread,
    env,
};


const NO_KEY_MODIFIERS: KeyModifiers = KeyModifiers::empty();

type Id = usize;

#[derive(PartialEq, Debug, Eq, Clone)]
enum Status {
    Todo,
    Done,
}

#[derive(Default)]
struct Ui {
    list_curr: Option<Id>,
    row: usize,
    col: usize,
    stdout: Option<Stdout>,
}

impl Status {
    fn change(&self) -> Self {
        match self {
            Status::Todo => Status::Done,
            Status::Done => Status::Todo,
        }
    }
}

impl Ui {   
    fn begin(&mut self, col: usize, row: usize) {
        self.row = row;
        self.col = col;
        self.stdout = Some(stdout());
    }

    fn begin_list(&mut self, id: Id) {
        assert!(self.list_curr.is_none(), "Nested lists are not allowed!");
        self.list_curr = Some(id);
    }
    
    fn list_element(&mut self, label: &str, id: Id) {
        let id_curr: Id = self.list_curr.expect("Not allowed to create list elements outside of lists");
        self.label(
            label,
            if id_curr == id {Some(Color::Black)} else {None},
            if id_curr == id {Some(Color::White)} else {None}
        );
        }

    fn end_list(&mut self) {
        self.list_curr = None;
    }

    fn label(&mut self, text: &str, foreground_color: Option<Color>, background_color: Option<Color>) {
       execute!(
            self.stdout.as_ref().unwrap(),
            cursor::MoveTo(self.col as u16, self.row as u16),
            SetForegroundColor(foreground_color.unwrap_or(Color::Reset)),
            SetBackgroundColor(background_color.unwrap_or(Color::Reset)),
            Print(text),
            ResetColor,
        ).unwrap(); 
        self.row += 1; 
    } 
    fn end(&mut self) {} 
}

fn type_on_line(starting_x: usize, y: usize) -> crossterm::Result<String> {
    let mut stdout: Stdout = stdout();
    let mut input: String = String::from("");
    let mut x: usize = input.len();
    // TODO: Not perfect so could be upgraded;
    // Key(KeyEvent { code: Char('d'), modifiers: NONE })
    execute!(
        stdout, cursor::Show, cursor::MoveTo(starting_x as u16 , y as u16), Clear(ClearType::CurrentLine), Print(&input) 
    ).unwrap();
    execute!(stdout, cursor::MoveTo(x as u16, y as u16)).unwrap();
    while let Event::Key(KeyEvent { code, .. }) = read()? {
        match code {
            KeyCode::Enter => break,

            KeyCode::Left => {
                if x > 0 {
                    x -= 1;
                }
                execute!(stdout, cursor::MoveTo(x as u16, y as u16)).unwrap();
            }

            KeyCode::Right => {
                if x < input.len() {
                    x += 1;
                }
                execute!(stdout, cursor::MoveTo(x as u16, y as u16)).unwrap();
            }

            KeyCode::Backspace => {
                if x > 0 {
                    input.remove(x-1);
                    x -= 1;
                }
            }
            KeyCode::Delete => {
                if x < input.len() {
                    input.remove(x);
                }
            }

            KeyCode::Char(c) => {
                input.insert(x, c);
                x+=1;
            }            
            _ => {}
        }

        execute!(
            stdout, Clear(ClearType::CurrentLine), cursor::MoveTo(starting_x as u16, y as u16), Print(&input) 
        ).unwrap();
        execute!(stdout, cursor::MoveTo(x as u16, y as u16)).unwrap();
    }

    Ok(input)
}

struct List {
    // status: Status,
    list: Vec<String>,
    curr: Id,
}


impl List {
    fn list_up(&mut self) {
        if self.curr > 0 {
            self.curr -= 1;
        }
    }

    fn list_down(&mut self) {
        if self.curr+1 < self.len() {
            self.curr += 1;
        }
    }

    fn curr_delete(&mut self) {
        if self.len() > 0 {
            self.list.remove(self.curr);

            if self.curr == self.len() {
                self.list_up();
            }
        }
    }

    fn curr_delete_ret(&mut self) -> String {
        let mut deleted: String = String::new();
        if self.len() > 0 {
            deleted = self.list.remove(self.curr);
            
            if self.curr == self.len() {
                self.list_up();
            }
        } 
        deleted
    }

    fn transfer_curr_to(&mut self, target_list: &mut List) {
        if self.len() > 0 {
            target_list.push(self.curr_delete_ret());
        }
    }

    fn rename_curr(&mut self) {

    }

    fn push(&mut self, text: String) {
        self.list.push(text);
    }

    fn len(&self) -> usize {
        self.list.len()
    }
}

fn clear_terminal(stdout: &mut Stdout) {
    execute!(stdout, cursor::MoveTo(0,0), Clear(ClearType::FromCursorDown), cursor::Hide).unwrap();
}

fn sleep_ms(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

fn parse_item(line: &str) -> Option<(Status, &str)> {
    let todo_item = line
        .strip_prefix("TODO: ")
        .map(|title| (Status::Todo, title));
    let done_item = line
        .strip_prefix("DONE: ")
        .map(|title| (Status::Done, title));
    done_item.or(todo_item)
}

fn load_state(todo_list: &mut List, done_list: &mut List, file_path: &str) -> io::Result<()> {
    let file = File::open(file_path)?;
    for (index, line) in io::BufReader::new(file).lines().enumerate() {
        if index == 0 {
            todo_list.curr = line?.strip_prefix("TODO_CURR: ").unwrap().parse::<usize>().unwrap();
            continue
        }
        if index == 1 {
            done_list.curr = line?.strip_prefix("DONE_CURR: ").unwrap().parse::<usize>().unwrap();
            continue
        }
        match parse_item(&line?) {
            Some((Status::Todo, title)) => todo_list.push(title.to_string()),
            Some((Status::Done, title)) => done_list.push(title.to_string()),
            None => {
                clear_terminal(&mut stdout());
                eprintln!("{}:{}: ERROR: ill-formed item line", file_path, index + 1);
                process::exit(1);
            }
        } 
    }

    Ok(())
}

fn save_state(todo_list: &List, done_list: &List, file_path: &str) {
    let mut file = File::create(file_path).unwrap();
    writeln!(file, "TODO_CURR: {}", todo_list.curr).unwrap();
    writeln!(file, "DONE_CURR: {}", done_list.curr).unwrap();
    for todo in todo_list.list.iter() {
        writeln!(file, "TODO: {}", todo).unwrap();
    }
    for done in done_list.list.iter() {
        writeln!(file, "DONE: {}", done).unwrap();
    }
}


// TODO: Save system
// TODO: ADD notification system
// TODO: Rename items


fn main() -> crossterm::Result<()> {
    sleep_ms(0); // TODO: remove this when the func that this points to is used in this lifetime
    
    let mut stdout = stdout();
    
    let mut args = env::args();
    args.next().unwrap();

    let mut todo_list = List {
        // status: Status::Todo,
        list: Vec::<String>::new(),
        curr: 0,
    };

    let mut done_list = List {
        // status: Status::Done,
        list: Vec::<String>::new(),
        curr: 0,
    };

    
    let mut ui = Ui::default();
    let mut status: Status = Status::Todo;


    let file_path = match args.next() {
        Some(file_path) => file_path,
        None => {
            clear_terminal(&mut stdout);
            execute!(stdout, Print("Usage: todo-rs <file-path>\n")).unwrap();
            execute!(stdout, Print("All info written in this file will not be saved!\n")).unwrap();
            execute!(stdout, Print("\nPress 'q' to quit or any key to continue")).unwrap();

            loop {
                match read()? {
                    Event::Key( KeyEvent { code: KeyCode::Char('q'), .. }) => {
                        clear_terminal(&mut stdout);
                        process::exit(0);
                    },
                    Event::Key( KeyEvent { .. }) => break,
                    _ => ()
                }
            }

            String::new()
        },
    };

    match load_state(&mut todo_list, &mut done_list, &file_path) {
        Ok(()) => (), // notification = format!("Loaded file {}", file_path), TODO: <-
        Err(error) => {
            if error.kind() == ErrorKind::NotFound {
                // notification = format!("New file {}", file_path)
            } else {
                panic!(
                    "Could not load state from file `{}`: {:?}",
                    file_path, error
                );
            }
        }
    }

    let mut current_list;
    let mut past_list;

    terminal::enable_raw_mode().unwrap();
    execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0), Print("")).unwrap();    

    let mut quit: bool = false;    
    while !quit {
        clear_terminal(&mut stdout);

        // execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0), Print("")).unwrap();
        ui.begin(0, 0);
        ui.label("TODO: ", if status == Status::Todo {Some(Color::Green)} else {None}, None);
        ui.begin_list(if status == Status::Todo {todo_list.curr} else {todo_list.len()});
        for (id, todo) in todo_list.list.iter().enumerate() {
            ui.list_element(&format!("- [ ] {}", todo), id);
        }
        ui.end_list();
        ui.end();

        ui.label("\n-----------", None, None);

        ui.begin(0, todo_list.len()+3);
        ui.label("DONE: ", if status == Status::Done {Some(Color::Green)} else {None}, None);
        ui.begin_list(if status == Status::Done {done_list.curr} else {done_list.len()});
        for (id, done) in done_list.list.iter().enumerate() {
            ui.list_element(&format!("- [x] {}", done), id);
        }
        ui.end_list();
        ui.end();


        match status {
            Status::Todo => {
                current_list = &mut todo_list;
                past_list = &mut done_list;
            },
            Status::Done => {
                current_list = &mut done_list;
                past_list = &mut todo_list;
            }
            // _ => {assert!(false, "Status must be Todo OR DOne, but was {:?}", status)} 
        }
 
        // execute!(stdout, Print(format!(" \n\n\n\n{} -- {}", todo_vec.len(), current_id)))
        match read()? {
            // Event::Resize(_x, _y) => list_todos(&todo_vec, current_id),
            // / for the keyboard events using key-strokes
            Event::Key( KeyEvent { code, modifiers }) => match (code, modifiers) {     
                (KeyCode::Char('w'), NO_KEY_MODIFIERS) => current_list.list_up(),
                (KeyCode::Char('s'), NO_KEY_MODIFIERS) => current_list.list_down(),
                (KeyCode::Char('x'), NO_KEY_MODIFIERS) => current_list.curr_delete(),
                (KeyCode::Char('r'), NO_KEY_MODIFIERS) => current_list.rename_curr(),

                (KeyCode::Char('n'), NO_KEY_MODIFIERS) => {
                    // list_todos(&todo_vec, 0);
                    let input: String = type_on_line(0, todo_list.len()+1).unwrap();
                    if input.trim() != "" {
                        todo_list.list.push(input);
                    }
                },

                (KeyCode::Enter, NO_KEY_MODIFIERS) => current_list.transfer_curr_to(&mut past_list),


                (KeyCode::Tab, NO_KEY_MODIFIERS) => status = status.change(),
                (KeyCode::Char('q'), NO_KEY_MODIFIERS) => quit = true,
                _ => ()                  
            }
            
            _ => ()
        }
    }

    clear_terminal(&mut stdout);
    if file_path != "" {
        save_state(&todo_list, &done_list, &file_path);
        execute!(stdout, Print(&format!("Saved state to {}", file_path))).unwrap();
    }
    terminal::disable_raw_mode().unwrap();
    Ok(())
}
