use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget};
use std::sync::mpsc;

use crate::adapter::ServerAdapter;
use crate::{utils, AppEvent, CodeSnippet, DependencyCause, FilePath, HandleEvent};

#[derive(Clone)]
pub struct DependencyCausePanel {
    source_file: Option<FilePath>,
}

impl DependencyCausePanel {
    pub fn new(source_file: Option<FilePath>) -> Self {
        Self { source_file }
    }
}

pub struct State {
    dependency_causes: Vec<DependencyCause>,
    viewing_recompile_dependency_file: Option<FilePath>,
}

impl State {
    pub fn new() -> Self {
        Self {
            dependency_causes: vec![],
            viewing_recompile_dependency_file: None,
        }
    }
}

impl HandleEvent for State {
    type Widget = DependencyCausePanel;

    fn handle_event(
        &mut self,
        event: &AppEvent,
        widget: &Self::Widget,
        adapter: &mut impl ServerAdapter,
        dispatcher: mpsc::Sender<AppEvent>,
    ) {
        match event {
            AppEvent::SelectDependentFile(recompile_dependency) => {
                match widget.source_file {
                    Some(ref source) => {
                        // The source and sink is reverse in this case
                        adapter.get_dependency_causes(
                            &recompile_dependency.path,
                            source,
                            &recompile_dependency.reason,
                            Box::new(move |causes| {
                                dispatcher
                                    .send(AppEvent::GetDependencyCausesDone(causes))
                                    .unwrap();
                            }),
                        );
                    }

                    None => unreachable!(),
                };
            }

            AppEvent::GetDependencyCausesDone(causes) => {
                self.dependency_causes = causes.clone();
            }

            AppEvent::ViewDependentFile(dependency_link) => {
                self.viewing_recompile_dependency_file = Some(dependency_link.sink.clone());
            }

            AppEvent::StopViewDependentFile(_) => {
                self.viewing_recompile_dependency_file = None;
            }

            AppEvent::Cancel => {
                *self = Self::new();
            }

            _ => (),
        }
    }
}

impl<'a> StatefulWidget for DependencyCausePanel {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_bounding_box(area, buf);
        render_cause_snippets(area, buf, state);
    }
}

fn render_bounding_box(area: Rect, buf: &mut Buffer) {
    Block::default()
        .borders(Borders::ALL)
        .title("Dependency causes")
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White))
        .render(area, buf);
}

fn render_cause_snippets(area: Rect, buf: &mut Buffer, state: &mut State) {
    if let Some(ref viewing_file) = state.viewing_recompile_dependency_file {
        let lines = match state
            .dependency_causes
            .iter()
            .find(|cause| cause.sink == *viewing_file)
        {
            Some(cause) if cause.snippets.len() == 0 => {
                vec![Line::styled(
                    "No snippets",
                    Style::default().add_modifier(Modifier::BOLD),
                )]
            }

            Some(cause) if cause.snippets.len() > 0 => cause
                .snippets
                .iter()
                .flat_map(|snippet| code_snippet_text(cause.source.clone(), snippet))
                .collect(),

            _ => vec![],
        };

        Paragraph::new(lines)
            .style(Style::default().fg(Color::White))
            .render(utils::padding(&area, 2, 2), buf);
    }
}

fn code_snippet_text(source_file: FilePath, snippet: &CodeSnippet) -> Vec<Line> {
    let header_line = Line::from(vec![
        Span::from("-- File: "),
        Span::from(source_file).add_modifier(Modifier::BOLD),
    ]);

    let max_line_number_len = snippet.lines_span.1.to_string().len();

    let content_lines = snippet
        .content
        .split("\n")
        .enumerate()
        .map(|(index, line)| {
            let line_number = index + snippet.lines_span.0;
            let is_highlight =
                line_number >= snippet.highlight.0 && line_number <= snippet.highlight.1;

            let line_number_span = if is_highlight {
                Span::from(format!(
                    "{: >width$} =>",
                    line_number,
                    width = max_line_number_len
                ))
            } else {
                Span::from(format!(
                    "{: >width$}   ",
                    line_number,
                    width = max_line_number_len
                ))
            };

            let mut line = Line::from(vec![line_number_span, Span::from(format!(" │ {}", line))]);
            if is_highlight {
                line.patch_style(Style::default().fg(Color::Green));
            }

            line
        });

    let mut result = vec![header_line, Line::from("")];
    result.extend(content_lines);
    // Snippets separator
    result.push(Line::from(""));

    result
}

#[cfg(test)]
mod handle_event_tests {
    use super::*;
    use crate::{
        adapter::NoopAdapter, DependencyLink, DependencyType, FileEntry, RecomplileDependency,
        RecomplileDependencyReason,
    };
    use mpsc::Receiver;

    fn widget() -> DependencyCausePanel {
        DependencyCausePanel::new(Some(String::from("source")))
    }

    fn mock_adapter(snippets: Vec<CodeSnippet>) -> impl ServerAdapter {
        struct MockAdapter {
            snippets: Vec<CodeSnippet>,
        }

        impl ServerAdapter for MockAdapter {
            fn init_server(&mut self) {
                unreachable!()
            }

            fn get_files(&mut self, _callback: Box<dyn FnOnce(Vec<FileEntry>) -> ()>) {
                unreachable!()
            }

            fn get_dependency_causes(
                &mut self,
                _source: &FilePath,
                _sink: &FilePath,
                _reason: &crate::RecomplileDependencyReason,
                callback: Box<dyn FnOnce(Vec<DependencyCause>)>,
            ) {
                callback(vec![DependencyCause {
                    source: String::from("source"),
                    sink: String::from("sink"),
                    dependency_type: DependencyType::Compile,
                    snippets: self.snippets.clone(),
                }])
            }
        }

        MockAdapter { snippets }
    }

    fn collect_events(rx: Receiver<AppEvent>) -> Vec<AppEvent> {
        rx.try_iter().collect()
    }

    #[test]
    fn select_file() {
        let snippets = vec![CodeSnippet {
            content: String::from("content"),
            highlight: (2, 2),
            lines_span: (1, 3),
        }];
        let mut adapter = mock_adapter(snippets.clone());

        let mut state = State::new();

        let event = AppEvent::SelectDependentFile(RecomplileDependency {
            id: String::from("id"),
            path: String::from("recompile_dependency"),
            reason: RecomplileDependencyReason::Compile,
            dependency_chain: vec![],
        });
        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&event, &widget(), &mut adapter, tx.clone());

        let events = collect_events(rx);
        assert_eq!(events.len(), 1);

        if let AppEvent::GetDependencyCausesDone(causes) = &events[0] {
            assert_eq!(causes[0].snippets, snippets);
        } else {
            panic!("Expected GetDependencyCausesDone event");
        }

        state.handle_event(&events[0], &widget(), &mut adapter, tx);
        assert_eq!(state.dependency_causes[0].snippets, snippets);
    }

    #[test]
    fn view_dependent_file() {
        let mut state = State::new();

        let event = AppEvent::ViewDependentFile(DependencyLink {
            source: String::from("source"),
            sink: String::from("sink"),
            dependency_type: DependencyType::Compile,
        });
        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&event, &widget(), &mut NoopAdapter::new(), tx);

        assert_eq!(
            state.viewing_recompile_dependency_file,
            Some(String::from("sink"))
        );
        assert_eq!(collect_events(rx).len(), 0);
    }

    #[test]
    fn cancel() {
        let mut state = State::new();
        state.dependency_causes = vec![DependencyCause {
            source: String::from("source"),
            sink: String::from("sink"),
            dependency_type: DependencyType::Compile,
            snippets: vec![],
        }];

        let (tx, rx) = mpsc::channel::<AppEvent>();
        state.handle_event(&AppEvent::Cancel, &widget(), &mut NoopAdapter::new(), tx);
        assert_eq!(state.dependency_causes.len(), 0);
        assert_eq!(collect_events(rx).len(), 0);
    }
}
