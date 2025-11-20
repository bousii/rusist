use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::process::ExitCode;
use color_eyre::{ Result, eyre::eyre };
use crossterm::event::{ self, Event, KeyCode };
use ratatui::{
    DefaultTerminal,
    Frame,
    layout::{ Constraint, Layout, Rect, Alignment },
    widgets::{ Block, Borders, Table, Row, Paragraph },
    style::{ Style, Color, Modifier },
    text::Line,
};

const ADD_ENTRY: usize = 0;
const REMOVE_ENTRY: usize = 1;
const VIEW_ORDER: usize = 2;

enum InputField {
    Name,
    Initiative,
}

struct Args {
    filename: Option<String>,
}

struct Combatant {
    name: String,
    initiative: i32,
}

struct Combat {
    combatants: Vec<Combatant>,
    current_turn: usize,
    round: i8,
}

enum SetupMenuState {
    PopulateEntries,
    AddEntry,
    RemoveEntry,
    ViewOrder,
}

struct SetupState {
    combatants: Vec<Combatant>,
    menu: SetupMenuState,
    selected: usize,
    max_size: usize,
    name_input: String,
    initiative_input: String,
    active_field: InputField,
}

fn parse_args() -> Result<Args> {
    let mut args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        return Err(eyre!("Usage: program <filename>"));
    }
    let filename = if args.len() >= 2 {
        Some(args.remove(1))
    } else {
        None
    };
    Ok(Args { filename })
}

fn read_lines<P>(file_path: P) -> io::Result<io::Lines<io::BufReader<File>>> 
where P: AsRef<Path>, {
    let file = File::open(file_path)?;
    Ok(io::BufReader::new(file).lines())
}

fn grab_initiative(filename: String) -> Result<Vec<Combatant>> {
    //println!("Grabbing initiative from file {}", filename);
    let mut file_combatants: Vec<Combatant> = Vec::new();
    if let Ok(lines) = read_lines(filename) {
        for line in lines.map_while(Result::ok) {
            let splices: Vec<&str> = line.split(", ").collect();
            if splices.len() != 2 {
                return Err(eyre!("Line format: <Name>, <Initiative Roll>"));
            }
            let fighter_name : String = splices[0].to_string();
            let initiative_str = splices[1].to_string();

            match initiative_str.parse::<i32>() {
                Ok(initiative_roll) => {
                    //println!("{} {}", fighter_name, initiative_roll);
                    let combatant: Combatant = Combatant { name: fighter_name, initiative: initiative_roll };
                    file_combatants.push(combatant);
                }
                Err(e) => {
                    return Err(eyre!("{}", e));
                }
            }
        }
    }
    Ok(file_combatants)
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

fn render_populate_entries(frame: &mut Frame, state: &SetupState) {
    let area = frame.area();
    let centered_area = centered_rect(60, 50, area);

    let rows = vec![
        Row::new(vec![Line::from("Create New Entry").alignment(Alignment::Center)]),
        Row::new(vec![Line::from("Remove Entry").alignment(Alignment::Center)]),
        Row::new(vec![Line::from("View Initiative Order").alignment(Alignment::Center)]),
        Row::new(vec![Line::from("Continue").alignment(Alignment::Center)]),
    ];

    let table = Table::new(
        rows,
        [Constraint::Percentage(100)],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Navigate with ↑/↓, Enter to select")
            .title_alignment(Alignment::Center)
    )
    .highlight_style(
        Style::default()
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    );

    frame.render_stateful_widget(table, centered_area, &mut ratatui::widgets::TableState::default().with_selected(Some(state.selected)));
}

fn render_add_entry(frame: &mut Frame, state: &SetupState) {
    let area = frame.area();
    let centered_area = centered_rect(70, 60, area);

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(3),
    ])
    .split(centered_area);

    let name_style = if matches!(state.active_field, InputField::Name) {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let name_block = Block::default()
        .borders(Borders::ALL)
        .title("Name")
        .style(name_style);
    let name_paragraph = Paragraph::new(state.name_input.as_str())
        .block(name_block);
    frame.render_widget(name_paragraph, chunks[0]);

    let initiative_style = if matches!(state.active_field, InputField::Initiative) {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    let initiative_block = Block::default()
        .borders(Borders::ALL)
        .title("Initiative")
        .style(initiative_style);
    let initiative_paragraph = Paragraph::new(state.initiative_input.as_str())
        .block(initiative_block);
    frame.render_widget(initiative_paragraph, chunks[1]);

}

fn render_remove_entry(frame: &mut Frame, state: &SetupState) {
    let area = frame.area();
    let centered_area = centered_rect(70, 60, area);

    let rows: Vec<Row> = state.combatants.iter().map(|c| {
        Row::new(vec![c.name.clone(), c.initiative.to_string()])
    }).collect();

    let table = Table::new(
        rows,
        [Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Select entry to remove (Enter to delete, Esc to cancel)")
            .title_alignment(Alignment::Center)
    )
    .header(
        Row::new(vec!["Name", "Initiative"])
            .style(Style::default().add_modifier(Modifier::BOLD))
    )
    .highlight_style(
        Style::default()
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    );

    frame.render_stateful_widget(
        table,
        centered_area,
        &mut ratatui::widgets::TableState::default().with_selected(Some(state.selected))
    );
}

fn render_view_initiative_order(frame: &mut Frame, state: &SetupState) {
    let area = frame.area();
    let centered_area = centered_rect(60, 50, area);


    let rows: Vec<Row> = state.combatants.iter().map(|c| {
        Row::new(vec![c.name.clone(), c.initiative.to_string()])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Initiative Order")
            .title_alignment(Alignment::Center)
    )
    .header(
        Row::new(vec!["Name", "Initiative"])
            .style(Style::default().add_modifier(Modifier::BOLD))
    );

    frame.render_widget(table, centered_area);
}

fn add_entry(state: &mut SetupState) -> Result<bool> {

    if state.name_input.is_empty() {
        return Ok(false)
    }

    if let Ok(initiative) = state.initiative_input.parse::<i32>() {
        let combatant: Combatant = Combatant { name: state.name_input.clone(), initiative: initiative };
        state.combatants.push(combatant);
    }

    state.name_input.clear();
    state.initiative_input.clear();
    state.active_field = InputField::Name;

    state.menu = SetupMenuState::PopulateEntries;
    state.selected = 0;
    Ok(false)
}

fn remove_entry(state: &mut SetupState) -> Result<bool> {
    if state.selected >= state.combatants.len() {
        return Ok(false)
    }

    state.combatants.remove(state.selected);
    state.menu = SetupMenuState::PopulateEntries;
    state.selected = 0;
    Ok(false)
}

fn view_initiative_order(state: &mut SetupState) -> Result<bool> {
    state.combatants.sort_by(|a, b| b.initiative.cmp(&a.initiative));
    state.menu = SetupMenuState::PopulateEntries;
    state.selected = 0;
    Ok(false)
}


fn populate_entries(terminal: &mut DefaultTerminal) -> Result<Vec<Combatant>> {
    let mut state = SetupState { menu: SetupMenuState::PopulateEntries, selected: 0, combatants: Vec::new(), max_size: 3,
                                 name_input: String::new(), initiative_input: String::new(), active_field: InputField::Name };

    loop {
        if !matches!(state.menu, SetupMenuState::PopulateEntries) {
            state.max_size = state.combatants.len();
        } else {
            state.max_size = 3;
        }

        terminal.draw(|frame| render(frame, &state))?;

        if let Event::Key(key) = event::read()? {
          let exit: bool = handle_input(&mut state, key.code)?;
          if exit {
              break Ok(state.combatants);
          }
        }
    }
}


fn render(frame: &mut Frame, state: &SetupState) {
    match state.menu {
        SetupMenuState::PopulateEntries => render_populate_entries(frame, state),
        SetupMenuState::AddEntry => render_add_entry(frame, state),
        SetupMenuState::RemoveEntry=> render_remove_entry(frame, state),
        SetupMenuState::ViewOrder=> render_view_initiative_order(frame, state),
    }
}

fn handle_enter(state: &mut SetupState) -> Result<bool> {
    match state.menu {
        SetupMenuState::PopulateEntries => {
            match state.selected {
                ADD_ENTRY => {
                    state.menu = SetupMenuState::AddEntry;
                    state.selected = 0;
                    Ok(false)
                },
                REMOVE_ENTRY => {
                    state.menu = SetupMenuState::RemoveEntry;
                    state.selected = 0;
                    Ok(false)
                },
                VIEW_ORDER => {
                    state.menu = SetupMenuState::ViewOrder;
                    state.selected = 0;
                    Ok(false)
                },
                _ => { Ok(true) },
            }
        },
        SetupMenuState::AddEntry => add_entry(state),
        SetupMenuState::RemoveEntry => remove_entry(state),
        SetupMenuState::ViewOrder => view_initiative_order(state),
    }

}


fn handle_input(state: &mut SetupState, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Up => {
            if state.selected > 0 {
                state.selected -= 1;
            }
            Ok(false)
        },
        KeyCode::Down => {
            if state.selected < state.max_size {
                state.selected += 1;
            }
            Ok(false)
        },
        KeyCode::Enter => handle_enter(state),
        KeyCode::Esc => {
            if matches!(state.menu, SetupMenuState::PopulateEntries) {
                return Ok(true);  // Exit app if on main
            }
            state.menu = SetupMenuState::PopulateEntries;
            state.selected = 0;
            Ok(false)
        },
        KeyCode::Tab => {
            if matches!(state.menu, SetupMenuState::AddEntry) {
                state.active_field = if matches!(state.active_field, InputField::Name) {
                    InputField::Initiative
                } else {
                    InputField::Name
                };
            }
            Ok(false)
        },
        KeyCode::Backspace => {
            if matches!(state.menu, SetupMenuState::AddEntry) {
                match state.active_field {
                    InputField::Initiative => { state.initiative_input.pop(); },
                    InputField::Name => { state.name_input.pop(); },
                }
            }
            Ok(false)
        },
        KeyCode::Char(c) => {
            if matches!(state.menu, SetupMenuState::AddEntry) {
                match state.active_field {
                    InputField::Name => state.name_input.push(c),
                    InputField::Initiative => {
                        // Only allow digits and minus sign for initiative
                        if c.is_ascii_digit() || (c == '-' && state.initiative_input.is_empty()) {
                            state.initiative_input.push(c);
                        }
                    },
                }
            }
            Ok(false)
        },
        _ => {Ok(false)},
    }
}

fn run(args: Args, mut terminal: DefaultTerminal) -> Result<()> {
    let mut combatants = match args.filename {
        Some(name) => match grab_initiative(name) {
            Ok(fighters) => fighters,
            Err(e) => {
                return Err(e)
            },
        },
        None => match populate_entries(&mut terminal) {
            Ok(fighters) => fighters,
            Err(e) => return Err(e),
        }
    };

    if combatants.is_empty() {
        return Err(eyre!("Initialization error: Unable to form a combatants list"));
    }

    combatants.sort_by(|a, b| b.initiative.cmp(&a.initiative));

    let mut combat: Combat = Combat { combatants: combatants, current_turn: 0, round: 0 };
    loop {
        terminal.draw(|frame| render_combat(frame, &combat))?;

        if let Event::Key(key) = event::read()? {
            let exit: bool = handle_combat_input(&mut combat, key.code)?;
            if exit {
                break;
            }
        }
    }

    Ok(())
}

fn handle_combat_input(state: &mut Combat, key: KeyCode) -> Result<bool> {
    match key {
        KeyCode::Enter => {
            state.current_turn = (state.current_turn + 1) % state.combatants.len();
            if state.current_turn == 0 {
                state.round += 1;
            }
            Ok(false)
        },
        KeyCode::Backspace => {
            if state.current_turn == 0 {
                state.round -= 1;
                state.current_turn = state.combatants.len() - 1;
            } else {
                state.current_turn = state.current_turn - 1;
            }
            Ok(false)
        },
        KeyCode::Esc => {
            return Ok(true);  // Exit app if on main
        },
        _ => {Ok(false)},
    }

}

fn render_combat(frame: &mut Frame, state: &Combat) {
    let area = frame.area();
    let centered_area = centered_rect(60, 50, area);

    let rows: Vec<Row> = state.combatants.iter().map(|c| {
        Row::new(vec![c.name.clone(), c.initiative.to_string()])
    }).collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(70),
            Constraint::Percentage(30),
        ],
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Initative! Round {} - Turn {}", state.round + 1, state.current_turn + 1))
            .title_alignment(Alignment::Center)
    )
    .highlight_style(
        Style::default()
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    );

    frame.render_stateful_widget(table, centered_area, &mut ratatui::widgets::TableState::default().with_selected(Some(state.current_turn)));

}

fn main() -> Result<ExitCode> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = match parse_args() {
        Ok(args) => match run(args, terminal) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Error: {}", e);
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("{}", e);
            ExitCode::FAILURE
        }
    };
    ratatui::restore();
    Ok(result)
}
