use anyhow::Result;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::terminal::Terminal;
use ratatui::Frame;
use std::io::Stderr;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use ui::components::dependency_cause_panel::DependencyCausePanel;

use ui::adapter::{Adapter, ServerAdapter};
use ui::app_event::AppEvent;
use ui::app_state::StateMachine;
use ui::app_state::{AppState, NoopWidget};
use ui::components::file_dependent_panel::FileDependentPanel;
use ui::components::file_panel::FilePanel;
use ui::components::instructions::Instructions;
use ui::components::search_input::SearchInput;
use ui::utils::filter_files_list;
use ui::{FileEntry, RecomplileDependency, FRAME_COUNT};
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
        .stderr(Stdio::piped())
        .spawn()
        .expect("mix command failed to start");

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
    let mut exit_output = String::new();
    let (tx, rx) = std::sync::mpsc::channel::<AppEvent>();

    let tx_clone = tx.clone();
    adapter.get_files(Box::new(move |files| {
        tx_clone.send(AppEvent::GetFilesDone(files)).unwrap();
    }));

    // Main application loop
    'main_loop: loop {
        unsafe {
            FRAME_COUNT += 1;
        }

        let (files_list, file_panel_title) = get_files_list(&app_state);

        let widget_board = WidgetBoard {
            file_panel: FilePanel::new(files_list, file_panel_title),
            file_dependent_panel: app_state.global.selected_dependency_source.as_ref().map(
                |file| {
                    let (dependencies_list, file_dependent_panel_title) =
                        get_dependent_files_list(&app_state, file);

                    FileDependentPanel::new(
                        file.path.clone(),
                        dependencies_list,
                        file_dependent_panel_title,
                    )
                },
            ),
            dependency_cause_panel: DependencyCausePanel::new(
                app_state
                    .global
                    .selected_dependency_source
                    .as_ref()
                    .map(|f| f.path.clone()),
            ),
        };

        terminal.draw(|f| {
            let widget_board = widget_board.clone();
            let frame_rect = f.size();

            let [left_rect, right_rect, bottom_rect] = calculate_layout(frame_rect);

            render_left_panel(f, &widget_board, &mut app_state, left_rect);

            f.render_stateful_widget(
                widget_board.dependency_cause_panel,
                right_rect,
                &mut app_state.dependency_cause_panel,
            );

            render_footer(f, &mut app_state, bottom_rect);
        })?;

        adapter.poll_responses();

        let terminal_events = poll_terminal_event(&mut app_state, &widget_board)?;
        let dispatcher_events = rx.try_iter();

        for event in terminal_events.into_iter().chain(dispatcher_events) {
            match event {
                AppEvent::Quit => break 'main_loop,
                event => dispatch_event(
                    &mut app_state,
                    &event,
                    &widget_board,
                    &mut adapter,
                    tx.clone(),
                ),
            }
        }

        match adapter.check_server_status() {
            Some(output) => {
                exit_output = output;
                break 'main_loop;
            }

            None => (),
        }
    }

    // shutdown down: reset terminal back to original state
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    println!("{}", exit_output);

    Ok(())
}

fn get_files_list(app_state: &AppState) -> (Option<Vec<FileEntry>>, Option<String>) {
    let filtered_files_list = app_state
        .global
        .files_list
        .as_ref()
        .map(|files| filter_files_list(files, &app_state.global.file_panel_search));

    let total_files_count = app_state
        .global
        .files_list
        .as_ref()
        .map(|f| f.len())
        .unwrap_or(0);
    let total_filtered_files_count = filtered_files_list.as_ref().map(|f| f.len()).unwrap_or(0);

    let title = if app_state.global.file_panel_search.is_searching() {
        Some(format!(
            " ({} of {})",
            total_filtered_files_count, total_files_count
        ))
    } else {
        None
    };

    return (filtered_files_list, title);
}

fn get_dependent_files_list(
    app_state: &AppState,
    file_entry: &FileEntry,
) -> (Vec<RecomplileDependency>, Option<String>) {
    let filtered_dependencies_list = filter_files_list(
        &file_entry.recompile_dependencies,
        &app_state.global.file_dependent_panel_search,
    );

    let total_files_count = file_entry.recompile_dependencies.len();
    let total_filtered_files_count = filtered_dependencies_list.len();

    let panel_title = if app_state.global.file_dependent_panel_search.is_searching() {
        Some(format!(
            " ({} of {})",
            total_filtered_files_count, total_files_count
        ))
    } else {
        None
    };

    return (filtered_dependencies_list, panel_title);
}

fn calculate_layout(root_rect: Rect) -> [Rect; 3] {
    let layouts = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(100), Constraint::Min(1)])
        .split(root_rect);

    let main_rect = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(layouts[0]);

    return [main_rect[0], main_rect[1], layouts[1]];
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

fn render_footer(f: &mut Frame<CrosstermBackend<Stderr>>, app_state: &mut AppState, area: Rect) {
    match app_state.global.state_machine {
        StateMachine::FilePanelView => {
            if app_state.global.file_panel_search.is_active() {
                f.render_widget(
                    SearchInput::new(app_state.global.file_panel_search.clone()),
                    area,
                );
            } else {
                f.render_widget(Instructions::new(), area)
            }
        }

        StateMachine::FileDependentsView => {
            if app_state.global.file_dependent_panel_search.is_active() {
                f.render_widget(
                    SearchInput::new(app_state.global.file_dependent_panel_search.clone()),
                    area,
                );
            } else {
                f.render_widget(Instructions::new(), area)
            }
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
                if !app_state.global.file_panel_search.is_prompting() {
                    if let Some(event) = app_state
                        .file_panel
                        .produce_event(&terminal_event, &widget_board.file_panel)
                    {
                        app_events.push(event)
                    }
                }
            }

            StateMachine::FileDependentsView => {
                if !app_state.global.file_dependent_panel_search.is_prompting() {
                    if let Some(event) = app_state.file_dependent_panel.produce_event(
                        &terminal_event,
                        &widget_board.file_dependent_panel.clone().unwrap(),
                    ) {
                        app_events.push(event)
                    }
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
    dispatcher: mpsc::Sender<AppEvent>,
) {
    match app_state.global.state_machine {
        StateMachine::FilePanelView => app_state.file_panel.handle_event(
            event,
            &widget_board.file_panel,
            adapter,
            dispatcher.clone(),
        ),
        StateMachine::FileDependentsView => {
            app_state.file_dependent_panel.handle_event(
                event,
                // It is guarantee that the widget exists if the app is in this state
                &widget_board.file_dependent_panel.clone().unwrap(),
                adapter,
                dispatcher.clone(),
            )
        }
    };

    app_state.dependency_cause_panel.handle_event(
        &event,
        &widget_board.dependency_cause_panel,
        adapter,
        dispatcher.clone(),
    );

    // AppState is a special case since it doesn't have a concrete widget associated with it
    // We create a dummy widget to solve that
    app_state.handle_event(&event, &NoopWidget {}, adapter, dispatcher);
}
