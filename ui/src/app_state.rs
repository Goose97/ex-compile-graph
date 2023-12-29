use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;
use std::sync::mpsc;

use crate::adapter::ServerAdapter;
use crate::app_event::AppEvent;
use crate::components::{dependency_cause_panel, file_dependent_panel, file_panel, search_input};
use crate::{FileEntry, HandleEvent, ProduceEvent};

#[derive(PartialEq, Debug)]
pub enum StateMachine {
    FilePanelView,
    FileDependentsView,
}

pub struct GlobalState {
    pub state_machine: StateMachine,
    pub selected_dependency_source: Option<FileEntry>,
    pub file_panel_search: search_input::State,
    pub file_dependent_panel_search: search_input::State,
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

                file_panel_search: search_input::State::default(),
                file_dependent_panel_search: search_input::State::default(),

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

            AppEvent::EnterSearch => match self.global.state_machine {
                StateMachine::FilePanelView => {
                    self.global.file_panel_search.prompt_begin();
                }

                StateMachine::FileDependentsView => {
                    self.global.file_dependent_panel_search.prompt_begin();
                }
            },

            AppEvent::SearchInput(char) => match self.global.state_machine {
                StateMachine::FilePanelView => {
                    self.global.file_panel_search.prompt_add(*char);
                }

                StateMachine::FileDependentsView => {
                    self.global.file_dependent_panel_search.prompt_add(*char);
                }
            },

            AppEvent::SearchInputDelete => match self.global.state_machine {
                StateMachine::FilePanelView => {
                    self.global.file_panel_search.prompt_remove();
                }

                StateMachine::FileDependentsView => {
                    self.global.file_dependent_panel_search.prompt_remove();
                }
            },

            AppEvent::SubmitSearch => match self.global.state_machine {
                StateMachine::FilePanelView => self.global.file_panel_search.search(),
                StateMachine::FileDependentsView => {
                    self.global.file_dependent_panel_search.search()
                }
            },

            AppEvent::Cancel if self.global.state_machine == StateMachine::FilePanelView => {
                if self.global.file_panel_search.is_active() {
                    self.global.file_panel_search.cancel();
                }
            }

            AppEvent::Cancel if self.global.state_machine == StateMachine::FileDependentsView => {
                if self.global.file_dependent_panel_search.is_active() {
                    self.global.file_dependent_panel_search.cancel();
                } else {
                    self.global.state_machine = StateMachine::FilePanelView;
                    self.global.selected_dependency_source = None;
                }
            }

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
                    crossterm::event::KeyCode::Char(char)
                        if self.file_panel_search.is_prompting()
                            || self.file_dependent_panel_search.is_prompting() =>
                    {
                        Some(AppEvent::SearchInput(char))
                    }

                    crossterm::event::KeyCode::Backspace
                        if self.file_panel_search.is_prompting()
                            || self.file_dependent_panel_search.is_prompting() =>
                    {
                        Some(AppEvent::SearchInputDelete)
                    }

                    crossterm::event::KeyCode::Enter
                        if self.file_panel_search.is_prompting()
                            || self.file_dependent_panel_search.is_prompting() =>
                    {
                        Some(AppEvent::SubmitSearch)
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

    fn dispatch_events(state: &mut AppState, events: &[AppEvent], tx: mpsc::Sender<AppEvent>) {
        for event in events {
            state.handle_event(&event, &NoopWidget {}, &mut NoopAdapter {}, tx.clone());
        }
    }

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
        assert!(state.global.selected_dependency_source.is_none());
    }

    #[test]
    fn enter_search() {
        let mut state = AppState::new();

        let event = AppEvent::EnterSearch;
        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&event, &NoopWidget {}, &mut NoopAdapter {}, tx);

        assert_eq!(collect_events(rx).len(), 0);
        assert!(state.global.file_panel_search.is_prompting());
    }

    #[test]
    fn search_input() {
        let mut state = AppState::new();
        state.global.file_panel_search = search_input::State::Prompt(String::new());

        let event_a = AppEvent::SearchInput('f');
        let event_b = AppEvent::SearchInput('o');
        let event_c = AppEvent::SearchInput('o');

        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&event_a, &NoopWidget {}, &mut NoopAdapter {}, tx.clone());
        state.handle_event(&event_b, &NoopWidget {}, &mut NoopAdapter {}, tx.clone());
        state.handle_event(&event_c, &NoopWidget {}, &mut NoopAdapter {}, tx.clone());

        assert_eq!(collect_events(rx).len(), 0);
        assert_eq!(
            state.global.file_panel_search.prompt_input().unwrap(),
            String::from("foo")
        );
    }

    #[test]
    fn search_input_delete() {
        let mut state = AppState::new();
        state.global.file_panel_search = search_input::State::Prompt(String::from("foo"));

        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::SearchInputDelete,
            &NoopWidget {},
            &mut NoopAdapter {},
            tx.clone(),
        );
        assert_eq!(
            state.global.file_panel_search.prompt_input().unwrap(),
            String::from("fo")
        );

        state.handle_event(
            &AppEvent::SearchInputDelete,
            &NoopWidget {},
            &mut NoopAdapter {},
            tx.clone(),
        );
        assert_eq!(
            state.global.file_panel_search.prompt_input().unwrap(),
            String::from("f")
        );

        state.handle_event(
            &AppEvent::SearchInputDelete,
            &NoopWidget {},
            &mut NoopAdapter {},
            tx.clone(),
        );
        assert_eq!(
            state.global.file_panel_search.prompt_input().unwrap(),
            String::from("")
        );

        state.handle_event(
            &AppEvent::SearchInputDelete,
            &NoopWidget {},
            &mut NoopAdapter {},
            tx.clone(),
        );
        assert_eq!(
            state.global.file_panel_search.prompt_input().unwrap(),
            String::from("")
        );

        assert_eq!(collect_events(rx).len(), 0);
    }

    #[test]
    fn search_submit() {
        let mut state = AppState::new();
        state.global.file_panel_search = search_input::State::Prompt(String::from("foo"));

        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::SubmitSearch,
            &NoopWidget {},
            &mut NoopAdapter {},
            tx.clone(),
        );
        assert_eq!(
            state.global.file_panel_search,
            search_input::State::Search(String::from("foo"))
        );
        assert_eq!(collect_events(rx).len(), 0);
    }

    #[test]
    fn cancel_search() {
        let mut state = AppState::new();
        state.global.file_panel_search = search_input::State::Prompt(String::from("foo"));

        let event = AppEvent::Cancel;
        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&event, &NoopWidget {}, &mut NoopAdapter {}, tx);

        assert_eq!(collect_events(rx).len(), 0);
        assert!(!state.global.file_panel_search.is_active());
    }

    #[test]
    fn submit_search_select_file_then_search_again() {
        let mut state = AppState::new();
        state.global.file_panel_search = search_input::State::Prompt(String::from("foo"));

        let (tx, rx) = mpsc::channel::<AppEvent>();

        dispatch_events(
            &mut state,
            &[
                AppEvent::SubmitSearch,
                // Select file and move to the file dependents panel
                AppEvent::SelectFile(FileEntry {
                    path: String::from("bar"),
                    recompile_dependencies: vec![],
                }),
            ],
            tx.clone(),
        );

        assert_eq!(state.global.state_machine, StateMachine::FileDependentsView);
        assert_eq!(
            state.global.file_panel_search,
            search_input::State::Search(String::from("foo"))
        );

        dispatch_events(
            &mut state,
            &[
                AppEvent::EnterSearch,
                AppEvent::SearchInput('b'),
                AppEvent::SearchInput('a'),
                AppEvent::SearchInput('z'),
                AppEvent::SubmitSearch,
            ],
            tx.clone(),
        );

        assert!(state.global.file_dependent_panel_search.is_active());
        assert_eq!(collect_events(rx).len(), 0);
    }

    #[test]
    fn submit_search_select_file_then_cancel_search() {
        let mut state = AppState::new();
        state.global.file_panel_search = search_input::State::Prompt(String::from("foo"));

        let (tx, rx) = mpsc::channel::<AppEvent>();

        dispatch_events(
            &mut state,
            &[
                AppEvent::SubmitSearch,
                // Select file and move to the file dependents panel
                AppEvent::SelectFile(FileEntry {
                    path: String::from("bar"),
                    recompile_dependencies: vec![],
                }),
            ],
            tx.clone(),
        );

        assert_eq!(state.global.state_machine, StateMachine::FileDependentsView);
        assert_eq!(
            state.global.file_panel_search,
            search_input::State::Search(String::from("foo"))
        );

        dispatch_events(&mut state, &[AppEvent::Cancel], tx.clone());

        assert!(state.global.file_panel_search.is_active());
        assert_eq!(state.global.state_machine, StateMachine::FilePanelView);
        assert_eq!(collect_events(rx).len(), 0);
    }
}
