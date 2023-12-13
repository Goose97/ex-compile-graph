use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget};
use std::sync::mpsc;

use crate::adapter::ServerAdapter;
use crate::app_event::AppEvent;
use crate::components::loading_icon::LoadingIcon;
use crate::utils;
use crate::{FileEntry, HandleEvent, ProduceEvent};

#[derive(Clone)]
pub struct FilePanel {}

impl FilePanel {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct State {
    pub files: Option<Vec<FileEntry>>,
    pub selected_file_index: usize,
}

impl State {
    pub fn new() -> Self {
        Self {
            files: None,
            selected_file_index: 0,
        }
    }
}

impl HandleEvent for State {
    type Widget = FilePanel;

    fn handle_event(
        &mut self,
        event: &AppEvent,
        _widget: &Self::Widget,
        _adapter: &mut impl ServerAdapter,
        _dispatcher: mpsc::Sender<AppEvent>,
    ) {
        if let Some(ref files) = self.files {
            if files.is_empty() {
                return;
            }

            match event {
                AppEvent::DownButtonPressed => {
                    if self.selected_file_index < files.len() - 1 {
                        self.selected_file_index += 1;
                    }
                }

                AppEvent::UpButtonPressed => {
                    if self.selected_file_index > 0 {
                        self.selected_file_index -= 1;
                    }
                }

                _ => (),
            }
        }
    }
}

impl ProduceEvent for State {
    type Widget = FilePanel;

    fn produce_event(
        &mut self,
        terminal_event: &crossterm::event::Event,
        _widget: &Self::Widget,
    ) -> Option<AppEvent> {
        if let crossterm::event::Event::Key(key) = terminal_event {
            if key.kind == crossterm::event::KeyEventKind::Press {
                return match key.code {
                    crossterm::event::KeyCode::Enter => {
                        if let Some(ref files) = self.files {
                            let index = self.selected_file_index;

                            Some(AppEvent::SelectFile(files[index].clone()))
                        } else {
                            None
                        }
                    }

                    _ => None,
                };
            }
        }

        None
    }
}

impl StatefulWidget for FilePanel {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut State) {
        let rect = utils::padding(&area, 1, 1);

        match state.files {
            Some(ref files) => render_files_list(files, state.selected_file_index, rect, buf),
            None => {
                let paragraph = Paragraph::new(Line::from(vec![
                    LoadingIcon::new().into(),
                    Span::from(" Collecting data"),
                ]))
                .style(Style::default().fg(Color::White))
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);

                let mut clone = rect.clone();
                clone.height = 1;
                utils::center_rect_in_container(&mut clone, &rect);
                paragraph.render(clone, buf);
            }
        }

        render_bounding_box(area, buf);
    }
}

fn render_files_list(
    files: &[FileEntry],
    selected_file_index: usize,
    area: Rect,
    buf: &mut Buffer,
) {
    let text: Vec<Line> = files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let max_width = area.width as usize - 5;
            let mut file_path = utils::compact_file_path(&file.path, max_width);
            file_path = format!("{:width$}", file_path, width = max_width);

            let dependents_count = format!("{: >3}", file.recompile_dependencies.len().to_string());

            let mut line = Line::from(vec![
                Span::from(" "),
                Span::from(file_path),
                Span::styled(dependents_count, Style::default().fg(Color::Yellow)),
                Span::from(" "),
            ]);

            if selected_file_index == index {
                line.patch_style(
                    Style::default()
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )
            }

            line
        })
        .collect();

    let paragraph = Paragraph::new(text).style(Style::default().fg(Color::White));
    paragraph.render(area, buf);
}

fn render_bounding_box(area: Rect, buf: &mut Buffer) {
    Block::default()
        .borders(Borders::ALL)
        .title("Files (with recompile dependencies count)")
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White))
        .render(area, buf);
}

#[cfg(test)]
mod handle_event_tests {
    use super::*;
    use crate::adapter::NoopAdapter;

    fn noop_adapter() -> NoopAdapter {
        NoopAdapter::new()
    }

    #[test]
    fn up_button() {
        let mut state = State::new();
        state.files = file_entries(&["one", "two", "three"]);
        state.selected_file_index = 1;

        let (tx, _) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::UpButtonPressed,
            &FilePanel::new(),
            &mut noop_adapter(),
            tx,
        );
        assert_eq!(state.selected_file_index, 0);
    }

    #[test]
    fn up_button_limit() {
        let mut state = State::new();
        state.files = file_entries(&["one", "two", "three"]);
        state.selected_file_index = 0;

        let (tx, _) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::UpButtonPressed,
            &FilePanel::new(),
            &mut noop_adapter(),
            tx,
        );
        assert_eq!(state.selected_file_index, 0);
    }

    #[test]
    fn down_button() {
        let mut state = State::new();
        state.files = file_entries(&["one", "two", "three"]);
        state.selected_file_index = 1;

        let (tx, _) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::DownButtonPressed,
            &FilePanel::new(),
            &mut noop_adapter(),
            tx,
        );
        assert_eq!(state.selected_file_index, 2);
    }

    #[test]
    fn down_button_limit() {
        let mut state = State::new();
        state.files = file_entries(&["one", "two", "three"]);
        state.selected_file_index = 2;

        let (tx, _) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::DownButtonPressed,
            &FilePanel::new(),
            &mut noop_adapter(),
            tx,
        );
        assert_eq!(state.selected_file_index, 2);
    }

    fn file_entries(files: &[&str]) -> Vec<FileEntry> {
        files
            .into_iter()
            .map(|f| FileEntry {
                path: f.to_string(),
                recompile_dependencies: vec![],
            })
            .collect()
    }
}
