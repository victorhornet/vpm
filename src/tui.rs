use std::{
    collections::BTreeMap,
    error::Error,
    io::{self, Stdout},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::Project;

pub fn start(projects: BTreeMap<usize, Project>) -> Result<(), Box<dyn Error>> {
    let mut terminal = setup_terminal()?;
    run(&mut terminal, projects)?;
    restore_terminal(&mut terminal)?;
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn Error>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    projects: BTreeMap<usize, Project>,
) -> Result<(), Box<dyn Error>> {
    let mut selected_project = 0usize;
    loop {
        terminal.draw(|frame| {
            // let greeting = Paragraph::new("Hello World!");
            let items = projects
                .values()
                .map(|p| {
                    ListItem::new(format!(
                        "{:02} | {} | {}",
                        p.id,
                        p.date,
                        p.name.split('-').collect::<Vec<_>>().join(" ")
                    ))
                })
                .collect::<Vec<_>>();
            let list = List::new(items)
                .block(Block::default().title("Projects").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                .highlight_symbol(">>");
            let mut list_state = ListState::default();
            list_state.select(Some(selected_project));
            frame.render_stateful_widget(list, frame.size(), &mut list_state);
        })?;
        if event::poll(Duration::from_millis(1000))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Up | KeyCode::Char('k') => {
                            if selected_project == 0 {
                                selected_project = projects.len();
                            }
                            selected_project -= 1;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            selected_project = (selected_project + 1) % projects.len();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;
    Ok(())
}
