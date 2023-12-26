use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    StatefulWidget, Widget,
};
use std::cmp::Reverse;
use std::sync::mpsc;

use crate::adapter::ServerAdapter;
use crate::app_event::AppEvent;
use crate::components::loading_icon::LoadingIcon;
use crate::utils;
use crate::{FileEntry, HandleEvent, ProduceEvent};

#[derive(Clone)]
pub struct FilePanel {
    files: Option<Vec<FileEntry>>,
}

impl FilePanel {
    pub fn new(files: Option<Vec<FileEntry>>) -> Self {
        Self { files }
    }
}

pub struct State {
    pub selected_file_index: usize,
}

impl State {
    pub fn new() -> Self {
        Self {
            selected_file_index: 0,
        }
    }
}

impl HandleEvent for State {
    type Widget = FilePanel;

    fn handle_event(
        &mut self,
        event: &AppEvent,
        widget: &Self::Widget,
        _adapter: &mut impl ServerAdapter,
        _dispatcher: mpsc::Sender<AppEvent>,
    ) {
        if let Some(ref files) = widget.files {
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

                AppEvent::SubmitSearch(_) => self.selected_file_index = 0,
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
        widget: &Self::Widget,
    ) -> Option<AppEvent> {
        if let crossterm::event::Event::Key(key) = terminal_event {
            if key.kind == crossterm::event::KeyEventKind::Press {
                return match key.code {
                    crossterm::event::KeyCode::Enter => {
                        if let Some(ref files) = widget.files {
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
        render_bounding_box(area, buf);

        match self.files {
            Some(ref files) => {
                let files_rect = utils::padding(&area, 1, 1);
                render_files_list(files, state, files_rect, buf);

                // We have padding y of 1, hence the -2
                let overflow = files.len() as u16 > (area.height - 2);
                if overflow {
                    render_scroll_bar(
                        files.len() as u16,
                        state.selected_file_index as u16,
                        area,
                        buf,
                    );
                }
            }

            None => {
                let paragraph = Paragraph::new(Line::from(vec![
                    LoadingIcon::new().into(),
                    Span::from(" Collecting data"),
                ]))
                .style(Style::default().fg(Color::White))
                .add_modifier(Modifier::BOLD)
                .alignment(Alignment::Center);

                let mut clone = area.clone();
                clone.height = 1;
                utils::center_rect_in_container(&mut clone, &area);
                paragraph.render(clone, buf);
            }
        }
    }
}

pub fn filter_files_list<'a, 'b>(
    files: &'a [FileEntry],
    search_term: Option<String>,
) -> Vec<&'a FileEntry> {
    match search_term {
        Some(term) => {
            let matcher = SkimMatcherV2::default();

            let mut filtered = files
                .iter()
                .filter_map(|file| {
                    let score = matcher.fuzzy_match(&file.path, &term);

                    match score {
                        Some(score) if score > 0 => Some((file, score)),
                        _ => None,
                    }
                })
                .collect::<Vec<(&FileEntry, i64)>>();

            filtered.sort_by_key(|item| Reverse(item.1));
            filtered.iter().map(|(file, _)| *file).collect()
        }

        None => files.iter().collect(),
    }
}

fn render_files_list(files: &[FileEntry], state: &State, area: Rect, buf: &mut Buffer) {
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

            if state.selected_file_index == index {
                line.patch_style(
                    Style::default()
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )
            }

            line
        })
        .collect();

    let scroll_offset = if (state.selected_file_index as u16) < area.height {
        0
    } else {
        state.selected_file_index as u16 - area.height + 1
    };

    let paragraph = Paragraph::new(text[scroll_offset as usize..].to_vec())
        .style(Style::default().fg(Color::White));

    paragraph.render(area, buf);
}

fn render_scroll_bar(content_length: u16, scroll_position: u16, area: Rect, buf: &mut Buffer) {
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .track_symbol(None)
        .track_style(Style::default().fg(Color::Gray))
        .thumb_style(Style::default().fg(Color::Gray));

    let mut scrollbar_state = ScrollbarState::default()
        .content_length(content_length)
        .position(scroll_position);

    scrollbar.render(area, buf, &mut scrollbar_state);
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
        state.selected_file_index = 1;

        let (tx, _) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::UpButtonPressed,
            &FilePanel::new(Some(file_entries(&["one", "two", "three"]))),
            &mut noop_adapter(),
            tx,
        );
        assert_eq!(state.selected_file_index, 0);
    }

    #[test]
    fn up_button_limit() {
        let mut state = State::new();
        state.selected_file_index = 0;

        let (tx, _) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::UpButtonPressed,
            &FilePanel::new(Some(file_entries(&["one", "two", "three"]))),
            &mut noop_adapter(),
            tx,
        );
        assert_eq!(state.selected_file_index, 0);
    }

    #[test]
    fn down_button() {
        let mut state = State::new();
        state.selected_file_index = 1;

        let (tx, _) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::DownButtonPressed,
            &FilePanel::new(Some(file_entries(&["one", "two", "three"]))),
            &mut noop_adapter(),
            tx,
        );
        assert_eq!(state.selected_file_index, 2);
    }

    #[test]
    fn down_button_limit() {
        let mut state = State::new();
        state.selected_file_index = 2;

        let (tx, _) = mpsc::channel::<AppEvent>();
        state.handle_event(
            &AppEvent::DownButtonPressed,
            &FilePanel::new(Some(file_entries(&["one", "two", "three"]))),
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

#[cfg(test)]
mod filter_list_tests {
    use super::*;

    #[test]
    fn found_one() {
        let files = file_entries(&["one", "two", "three"]);
        let filtered: Vec<&str> = filter_files_list(&files, Some(String::from("one")))
            .iter()
            .map(|f| f.path.as_str())
            .collect();

        assert_eq!(filtered, vec!["one"]);
    }

    #[test]
    fn found_many_and_sort_score() {
        let files = file_entries(&["one", "two_one", "three_two"]);
        let filtered: Vec<&str> = filter_files_list(&files, Some(String::from("one")))
            .iter()
            .map(|f| f.path.as_str())
            .collect();

        assert_eq!(filtered, vec!["one", "two_one"]);
    }

    #[test]
    fn found_none() {
        let files = file_entries(&["one", "two", "three"]);
        let filtered: Vec<&str> = filter_files_list(&files, Some(String::from("four")))
            .iter()
            .map(|f| f.path.as_str())
            .collect();

        assert!(filtered.is_empty());
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
