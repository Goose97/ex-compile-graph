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
                self.file_panel.files = Some(files.clone());
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
                    crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                        Some(AppEvent::DownButtonPressed)
                    }

                    crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                        Some(AppEvent::UpButtonPressed)
                    }

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
        let mut count = 0;
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
        let events = state.handle_event(&event, &NoopWidget {}, &mut NoopAdapter {}, tx);
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
        let events = state.handle_event(&event, &NoopWidget {}, &mut NoopAdapter {}, tx);
        assert_eq!(collect_events(rx).len(), 0);
        assert_eq!(state.global.state_machine, StateMachine::FilePanelView);
        assert!(state.global.selected_dependency_source.is_none());
    }
}
