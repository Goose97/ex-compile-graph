use anyhow::Result;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::terminal::Terminal;
use ratatui::Frame;
use std::io::Stderr;
use std::process::{Command, Stdio};
use ui::components::dependency_cause_panel::DependencyCausePanel;

use ui::adapter::{Adapter, ServerAdapter};
use ui::app_event::AppEvent;
use ui::app_state::StateMachine;
use ui::app_state::{AppState, NoopWidget};
use ui::components::file_dependent_panel::FileDependentPanel;
use ui::components::file_panel::FilePanel;
use ui::{HandleEvent, ProduceEvent};

#[derive(Clone)]
struct WidgetBoard {
    file_panel: FilePanel,
    file_dependent_panel: Option<FileDependentPanel>,
    dependency_cause_panel: DependencyCausePanel,
}

fn main() {
    let child_proc = Command::new("mix")
        .args(["run", "--no-halt"])
        .current_dir("..")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("mix command failed to start");

    // TODO: handle server crash
    let mut adapter = Adapter::new(child_proc);
    adapter.init_server();

    let _ = render(adapter);
}

fn render(mut adapter: Adapter) -> Result<()> {
    // startup: Enable raw mode for the terminal, giving us fine control over user input
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut app_state = AppState::new();

    let response = adapter.get_files();
    app_state.file_panel.files = response;

    // Main application loop
    'main_loop: loop {
        let widget_board: WidgetBoard = WidgetBoard {
            file_panel: FilePanel::new(),
            file_dependent_panel: app_state
                .global
                .selected_dependency_source
                .clone()
                .map(|file| FileDependentPanel::new(file.path, file.recompile_dependencies)),
            dependency_cause_panel: DependencyCausePanel::new(
                app_state
                    .global
                    .selected_dependency_source
                    .clone()
                    .map(|f| f.path),
            ),
        };

        terminal.draw(|f| {
            let widget_board = widget_board.clone();
            let frame_rect = f.size();

            let [left_rect, right_rect] = calculate_layout(frame_rect);

            render_left_panel(f, &widget_board, &mut app_state, left_rect);

            f.render_stateful_widget(
                widget_board.dependency_cause_panel,
                right_rect,
                &mut app_state.dependency_cause_panel,
            );
        })?;

        for event in poll_terminal_event(&mut app_state, &widget_board)? {
            match event {
                AppEvent::Quit => break 'main_loop,
                event => dispatch_event(&mut app_state, &event, &widget_board, &mut adapter),
            }
        }
    }

    // shutdown down: reset terminal back to original state
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    Ok(())
}

fn calculate_layout(root_rect: Rect) -> [Rect; 2] {
    let layouts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(root_rect);

    return [layouts[0], layouts[1]];
}

fn render_left_panel(
    f: &mut Frame<CrosstermBackend<Stderr>>,
    widget_board: &WidgetBoard,
    app_state: &mut AppState,
    area: Rect,
) {
    match &app_state.global.state_machine {
        StateMachine::FilePanelView => f.render_stateful_widget(
            widget_board.file_panel.clone(),
            area,
            &mut app_state.file_panel,
        ),

        StateMachine::FileDependentsView => {
            f.render_stateful_widget(
                // It is guarantee that the widget exists if the app is in this state
                widget_board.file_dependent_panel.clone().unwrap(),
                area,
                &mut app_state.file_dependent_panel,
            )
        }
    };
}

fn poll_terminal_event(
    app_state: &mut AppState,
    widget_board: &WidgetBoard,
) -> Result<Vec<AppEvent>> {
    if crossterm::event::poll(std::time::Duration::from_millis(25))? {
        let mut app_events = vec![];
        let terminal_event = crossterm::event::read()?;

        if let Some(event) = app_state
            .global
            .produce_event(&terminal_event, &NoopWidget {})
        {
            app_events.push(event)
        };

        match app_state.global.state_machine {
            StateMachine::FilePanelView => {
                if let Some(event) = app_state
                    .file_panel
                    .produce_event(&terminal_event, &widget_board.file_panel)
                {
                    app_events.push(event)
                }
            }

            StateMachine::FileDependentsView => {
                if let Some(event) = app_state.file_dependent_panel.produce_event(
                    &terminal_event,
                    &widget_board.file_dependent_panel.clone().unwrap(),
                ) {
                    app_events.push(event)
                }
            }
        }

        return Ok(app_events);
    }

    Ok(vec![])
}

fn dispatch_event(
    app_state: &mut AppState,
    event: &AppEvent,
    widget_board: &WidgetBoard,
    adapter: &mut Adapter,
) {
    let events_a = match app_state.global.state_machine {
        StateMachine::FilePanelView => {
            app_state
                .file_panel
                .handle_event(event, &widget_board.file_panel, adapter)
        }
        StateMachine::FileDependentsView => {
            app_state.file_dependent_panel.handle_event(
                event,
                // It is guarantee that the widget exists if the app is in this state
                &widget_board.file_dependent_panel.clone().unwrap(),
                adapter,
            )
        }
    };

    let events_b = app_state.dependency_cause_panel.handle_event(
        &event,
        &widget_board.dependency_cause_panel,
        adapter,
    );

    // AppState is a special case since it doesn't have a concrete widget associated with it
    // We create a dummy widget to solve that
    let events_c = app_state.handle_event(&event, &NoopWidget {}, adapter);

    // TODO: In theory, this could result in a infinite loop
    // We should have a safety net to avoid that
    for event in events_a.iter() {
        dispatch_event(app_state, event, widget_board, adapter);
    }

    for event in events_b.iter() {
        dispatch_event(app_state, event, widget_board, adapter);
    }

    for event in events_c.iter() {
        dispatch_event(app_state, event, widget_board, adapter);
    }
}

// use std::fs::OpenOptions;
// use std::io::Write;
// let mut file = OpenOptions::new()
//     .append(true)
//     .create(true)
//     .open("debug.log")
//     .unwrap();
// file.write(format!("Dispatching event {:?}\n", event).as_bytes())
//     .unwrap();
