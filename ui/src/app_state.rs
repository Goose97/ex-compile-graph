use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;
use std::sync::mpsc;

use crate::adapter::ServerAdapter;
use crate::app_event::AppEvent;
use crate::components::{dependency_cause_panel, file_dependent_panel, file_panel};
use crate::{FileEntry, HandleEvent, ProduceEvent};

#[derive(PartialEq, Debug)]
pub enum StateMachine {
    FilePanelView,
    FileDependentsView,
}

pub struct GlobalState {
    pub state_machine: StateMachine,
    pub selected_dependency_source: Option<FileEntry>,
    pub searching: bool,
    pub search_input: String,
    pub search_term: Option<String>,
    pub files_list: Option<Vec<FileEntry>>,
}

pub struct AppState {
    pub file_panel: file_panel::State,
    pub file_dependent_panel: file_dependent_panel::State,
    pub dependency_cause_panel: dependency_cause_panel::State,
    pub global: GlobalState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            file_panel: file_panel::State::new(),
            file_dependent_panel: file_dependent_panel::State::new(),
            dependency_cause_panel: dependency_cause_panel::State::new(),
            global: GlobalState {
                state_machine: StateMachine::FilePanelView,
                selected_dependency_source: None,

                searching: false,
                search_input: String::new(),
                search_term: None,

                files_list: None,
            },
        }
    }
}

pub struct NoopWidget;

impl StatefulWidget for NoopWidget {
    type State = AppState;

    fn render(self, _area: Rect, _buf: &mut Buffer, _state: &mut Self::State) {}
}

impl HandleEvent for AppState {
    type Widget = NoopWidget;

    fn handle_event(
        &mut self,
        event: &AppEvent,
        _widget: &Self::Widget,
        _adapter: &mut impl ServerAdapter,
        _dispatcher: mpsc::Sender<AppEvent>,
    ) {
        match event {
            AppEvent::SelectFile(file_entry) => {
                self.global.state_machine = StateMachine::FileDependentsView;
                self.global.selected_dependency_source = Some(file_entry.clone());
            }

            AppEvent::GetFilesDone(files) => {
                self.global.files_list = Some(files.clone());
            }

            AppEvent::EnterSearch => self.global.searching = true,

            AppEvent::SearchInput(char) if self.global.searching => {
                self.global.search_input.push(*char);
            }

            AppEvent::SearchInputDelete if self.global.searching => {
                self.global.search_input.pop();
            }

            AppEvent::SubmitSearch(query) if self.global.searching => {
                self.global.searching = false;
                self.global.search_input = String::new();
                self.global.search_term = Some(query.clone());
            }

            AppEvent::Cancel if self.global.searching => {
                self.global.searching = false;
                self.global.search_input = String::new();
            }

            AppEvent::Cancel if self.global.search_term.is_some() => {
                self.global.search_term = None;
            }

            AppEvent::Cancel => match self.global.state_machine {
                StateMachine::FilePanelView => (),
                StateMachine::FileDependentsView => {
                    self.global.state_machine = StateMachine::FilePanelView;
                    self.global.selected_dependency_source = None;
                }
            },

            _ => (),
        }
    }
}

impl ProduceEvent for GlobalState {
    type Widget = NoopWidget;

    fn produce_event(
        &mut self,
        terminal_event: &crossterm::event::Event,
        _widget: &Self::Widget,
    ) -> Option<AppEvent> {
        if let crossterm::event::Event::Key(key) = terminal_event {
            if key.kind == crossterm::event::KeyEventKind::Press {
                return match key.code {
                    crossterm::event::KeyCode::Char(char) if self.searching => {
                        Some(AppEvent::SearchInput(char))
                    }

                    crossterm::event::KeyCode::Backspace if self.searching => {
                        Some(AppEvent::SearchInputDelete)
                    }

                    crossterm::event::KeyCode::Enter if self.searching => {
                        Some(AppEvent::SubmitSearch(self.search_input.clone()))
                    }

                    crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                        Some(AppEvent::DownButtonPressed)
                    }

                    crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                        Some(AppEvent::UpButtonPressed)
                    }

                    crossterm::event::KeyCode::Char('/') => Some(AppEvent::EnterSearch),
                    crossterm::event::KeyCode::Esc => Some(AppEvent::Cancel),

                    crossterm::event::KeyCode::Char('q') => Some(AppEvent::Quit),
                    _ => None,
                };
            }
        }

        None
    }
}

#[cfg(test)]
mod handle_event_tests {
    use super::*;
    use crate::adapter::NoopAdapter;
    use mpsc::Receiver;

    fn collect_events(rx: Receiver<AppEvent>) -> Vec<AppEvent> {
        rx.try_iter().collect()
    }

    #[test]
    fn select_file() {
        let mut state = AppState::new();

        let event = AppEvent::SelectFile(FileEntry {
            path: String::from("foo"),
            recompile_dependencies: vec![],
        });

        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&event, &NoopWidget {}, &mut NoopAdapter {}, tx);
        assert_eq!(collect_events(rx).len(), 0);
        assert_eq!(state.global.state_machine, StateMachine::FileDependentsView);
        assert_eq!(
            state.global.selected_dependency_source.unwrap().path,
            String::from("foo")
        );
    }

    #[test]
    fn cancel() {
        let mut state = AppState::new();
        state.global.state_machine = StateMachine::FileDependentsView;
        state.global.selected_dependency_source = Some(FileEntry {
            path: String::from("foo"),
            recompile_dependencies: vec![],
        });

        let event = AppEvent::Cancel;

        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&event, &NoopWidget {}, &mut NoopAdapter {}, tx);
        assert_eq!(collect_events(rx).len(), 0);
        assert_eq!(state.global.state_machine, StateMachine::FilePanelView);
        assert!(state.global.selected_dependency_source.is_none());
    }

    #[test]
    fn enter_search() {
        let mut state = AppState::new();

        let event = AppEvent::EnterSearch;
        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&event, &NoopWidget {}, &mut NoopAdapter {}, tx);

        assert_eq!(collect_events(rx).len(), 0);
        assert!(state.global.searching);
    }

    #[test]
    fn search_input() {
        let mut state = AppState::new();
        state.global.searching = true;

        let event_a = AppEvent::SearchInput('f');
        let event_b = AppEvent::SearchInput('o');
        let event_c = AppEvent::SearchInput('o');

        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&event_a, &NoopWidget {}, &mut NoopAdapter {}, tx.clone());
        state.handle_event(&event_b, &NoopWidget {}, &mut NoopAdapter {}, tx.clone());
        state.handle_event(&event_c, &NoopWidget {}, &mut NoopAdapter {}, tx.clone());

        assert_eq!(collect_events(rx).len(), 0);
        assert_eq!(state.global.search_input, String::from("foo"));
    }

    #[test]
    fn search_input_delete() {
        let mut state = AppState::new();
        state.global.searching = true;
        state.global.search_input = String::from("foo");

        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::SearchInputDelete,
            &NoopWidget {},
            &mut NoopAdapter {},
            tx.clone(),
        );
        assert_eq!(state.global.search_input, String::from("fo"));

        state.handle_event(
            &AppEvent::SearchInputDelete,
            &NoopWidget {},
            &mut NoopAdapter {},
            tx.clone(),
        );
        assert_eq!(state.global.search_input, String::from("f"));

        state.handle_event(
            &AppEvent::SearchInputDelete,
            &NoopWidget {},
            &mut NoopAdapter {},
            tx.clone(),
        );
        assert_eq!(state.global.search_input, String::from(""));

        state.handle_event(
            &AppEvent::SearchInputDelete,
            &NoopWidget {},
            &mut NoopAdapter {},
            tx.clone(),
        );
        assert_eq!(state.global.search_input, String::from(""));

        assert_eq!(collect_events(rx).len(), 0);
    }

    #[test]
    fn search_submit() {
        let mut state = AppState::new();
        state.global.searching = true;
        state.global.search_input = String::from("foo");

        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::SubmitSearch(String::from("bar")),
            &NoopWidget {},
            &mut NoopAdapter {},
            tx.clone(),
        );
        assert!(state.global.search_input.is_empty());
        assert_eq!(state.global.searching, false);
        assert_eq!(state.global.search_term, Some(String::from("bar")));
        assert_eq!(collect_events(rx).len(), 0);
    }

    #[test]
    fn cancel_search() {
        let mut state = AppState::new();
        state.global.searching = true;
        state.global.search_input = String::from("foo");

        let event = AppEvent::Cancel;
        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&event, &NoopWidget {}, &mut NoopAdapter {}, tx);

        assert_eq!(collect_events(rx).len(), 0);
        assert_eq!(state.global.searching, false);
        assert!(state.global.search_input.is_empty());
    }
}
