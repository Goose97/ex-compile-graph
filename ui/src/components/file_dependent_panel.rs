use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget};
use std::sync::mpsc;

use crate::adapter::ServerAdapter;
use crate::utils;
use crate::{
    AppEvent, DependencyLink, DependencyType, FilePath, HandleEvent, ProduceEvent,
    RecomplileDependency,
};

#[derive(Clone)]
pub struct FileDependentPanel {
    dependency_source: FilePath,
    files: Vec<RecomplileDependency>,
}

impl FileDependentPanel {
    pub fn new(dependency_source: FilePath, files: Vec<RecomplileDependency>) -> Self {
        Self {
            dependency_source,
            files,
        }
    }
}

pub struct State {
    // (Index for the outer list, Index for the expanded inner list)
    selected_file_index: (usize, Option<usize>),
    expanded_file: Option<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            selected_file_index: (0, None),
            expanded_file: None,
        }
    }
}

impl HandleEvent for State {
    type Widget = FileDependentPanel;

    fn handle_event(
        &mut self,
        event: &AppEvent,
        widget: &Self::Widget,
        _adapter: &mut impl ServerAdapter,
        mut dispatcher: mpsc::Sender<AppEvent>,
    ) {
        match event {
            AppEvent::DownButtonPressed => {
                handle_down_button_pressed(self, widget, &mut dispatcher)
            }
            AppEvent::UpButtonPressed => handle_up_button_pressed(self, widget, &mut dispatcher),

            AppEvent::SelectDependentFile(file) => match self.expanded_file {
                Some(ref expanded) if expanded == &file.id => self.expanded_file = None,
                _ => self.expanded_file = Some(file.id.clone()),
            },

            AppEvent::Cancel => {
                *self = Self::new();
            }
            _ => (),
        }
    }
}

fn handle_down_button_pressed(
    state: &mut State,
    widget: &FileDependentPanel,
    dispatcher: &mut mpsc::Sender<AppEvent>,
) {
    enum Action {
        NextOuterList,
        NextExpandedList,
        ExitExpandedList,
    }

    let outer_list_len = widget.files.len();
    let outer_list_index = state.selected_file_index.0;

    let actions: &[Action] = match state.expanded_file {
        Some(ref expanded) => {
            let at_expanded_file = widget.files[state.selected_file_index.0].id == *expanded;
            let expanded_list_len = widget.files[state.selected_file_index.0]
                .dependency_chain
                .len();

            if at_expanded_file {
                match state.selected_file_index.1 {
                    Some(expanded_index) if expanded_index == expanded_list_len - 1 => {
                        // We reach the end of the list
                        if outer_list_index == outer_list_len - 1 {
                            &[]
                        } else {
                            &[Action::ExitExpandedList, Action::NextOuterList]
                        }
                    }

                    _ => &[Action::NextExpandedList],
                }
            } else {
                &[Action::NextOuterList]
            }
        }

        None => &[Action::NextOuterList],
    };

    for action in actions {
        match action {
            Action::NextOuterList => {
                if widget.files.len() != 0 && state.selected_file_index.0 < widget.files.len() - 1 {
                    state.selected_file_index.0 += 1;
                }
            }

            Action::NextExpandedList => match state.selected_file_index.1 {
                Some(index) => {
                    dispatcher
                        .send(stop_view_file_event(state, index, widget))
                        .unwrap();
                    dispatcher
                        .send(view_file_event(state, index + 1, widget))
                        .unwrap();
                    state.selected_file_index.1 = Some(index + 1);
                }

                None => {
                    dispatcher.send(view_file_event(state, 0, widget)).unwrap();
                    state.selected_file_index.1 = Some(0);
                }
            },

            Action::ExitExpandedList => {
                dispatcher
                    .send(stop_view_file_event(
                        state,
                        state.selected_file_index.1.unwrap(),
                        widget,
                    ))
                    .unwrap();
                state.selected_file_index.1 = None;
            }
        }
    }
}

fn handle_up_button_pressed(
    state: &mut State,
    widget: &FileDependentPanel,
    dispatcher: &mut mpsc::Sender<AppEvent>,
) {
    enum Action {
        PrevOuterList,
        PrevExpandedList,
        ExitExpandedList,
    }

    let action: Action = match state.expanded_file {
        Some(ref expanded) => {
            let at_expanded_file = widget.files[state.selected_file_index.0].id == *expanded;

            if at_expanded_file {
                match state.selected_file_index.1 {
                    Some(expanded_index) if expanded_index == 0 => Action::ExitExpandedList,
                    None => Action::PrevOuterList,
                    _ => Action::PrevExpandedList,
                }
            } else {
                Action::PrevOuterList
            }
        }

        None => Action::PrevOuterList,
    };

    match action {
        Action::PrevOuterList => {
            if state.selected_file_index.0 > 0 {
                state.selected_file_index.0 -= 1;

                // If the previous item is expanded, move to the last item in the expanded list
                if let Some(expanded) = &state.expanded_file {
                    let selected_file = &widget.files[state.selected_file_index.0];

                    if selected_file.id == *expanded {
                        state.selected_file_index.1 =
                            Some(selected_file.dependency_chain.len() - 1);

                        dispatcher
                            .send(view_file_event(
                                state,
                                selected_file.dependency_chain.len() - 1,
                                widget,
                            ))
                            .unwrap();
                    }
                }
            }
        }

        Action::PrevExpandedList => match state.selected_file_index.1 {
            Some(index) if index > 0 => {
                state.selected_file_index.1 = Some(index - 1);
                dispatcher
                    .send(stop_view_file_event(state, index, widget))
                    .unwrap();
                dispatcher
                    .send(view_file_event(state, index - 1, widget))
                    .unwrap();
            }
            _ => (),
        },

        Action::ExitExpandedList => {
            dispatcher
                .send(stop_view_file_event(
                    state,
                    state.selected_file_index.1.unwrap(),
                    widget,
                ))
                .unwrap();
            state.selected_file_index.1 = None;
        }
    }
}

fn view_file_event(
    state: &State,
    dependency_node_index: usize,
    widget: &FileDependentPanel,
) -> AppEvent {
    let selected_file = &widget.files[state.selected_file_index.0];
    AppEvent::ViewDependentFile(selected_file.dependency_chain[dependency_node_index].clone())
}

fn stop_view_file_event(
    state: &State,
    dependency_node_index: usize,
    widget: &FileDependentPanel,
) -> AppEvent {
    let selected_file = &widget.files[state.selected_file_index.0];
    AppEvent::StopViewDependentFile(selected_file.dependency_chain[dependency_node_index].clone())
}

impl ProduceEvent for State {
    type Widget = FileDependentPanel;

    fn produce_event(
        &mut self,
        terminal_event: &crossterm::event::Event,
        widget: &Self::Widget,
    ) -> Option<AppEvent> {
        if let crossterm::event::Event::Key(key) = terminal_event {
            if key.kind == crossterm::event::KeyEventKind::Press {
                return match key.code {
                    crossterm::event::KeyCode::Enter => {
                        let index = self.selected_file_index.0;

                        Some(AppEvent::SelectDependentFile(widget.files[index].clone()))
                    }

                    _ => None,
                };
            }
        }

        None
    }
}

impl StatefulWidget for FileDependentPanel {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut State) {
        let rect = utils::padding(&area, 1, 1);

        let text: Vec<Line> = self
            .files
            .iter()
            .enumerate()
            .flat_map(|(index, file)| {
                let max_width = rect.width as usize - 2;
                let prefix = match state.expanded_file {
                    Some(ref expanded) if expanded == &file.id => "▼",
                    _ => "▶",
                };

                let mut content = utils::compact_file_path(&file.path, max_width - 2);
                content = format!("{} {:width$}", prefix, content, width = max_width);

                let mut lines = vec![];
                lines.push(Line::from(format!(" {} ", content)));

                match state.expanded_file {
                    Some(ref expanded) if expanded == &file.id => {
                        let mut dependencies_chain =
                            dependency_chain_text(&file.dependency_chain, area);
                        lines.append(&mut dependencies_chain);
                    }
                    _ => (),
                }

                if state.selected_file_index.0 == index {
                    let to_be_patched: Vec<&mut Line> = match state.selected_file_index.1 {
                        Some(expanded_index) => {
                            // Each expanded item spans 4 lines
                            lines
                                .iter_mut()
                                .skip(1 + expanded_index * 4)
                                .take(4)
                                .collect()
                        }

                        None => lines.iter_mut().take(1).collect(),
                    };

                    for line in to_be_patched {
                        line.patch_style(
                            Style::default()
                                .bg(Color::Blue)
                                .add_modifier(Modifier::BOLD),
                        )
                    }
                }

                lines
            })
            .collect();

        let paragraph = Paragraph::new(text).style(Style::default().fg(Color::White));

        render_bounding_box(&self.dependency_source, area, buf);
        paragraph.render(rect, buf);
    }
}

fn render_bounding_box(source_file: &FilePath, area: Rect, buf: &mut Buffer) {
    let filename = source_file.split("/").last().or(Some("...")).unwrap();

    Block::default()
        .borders(Borders::ALL)
        .title(format!("Recompile files ({})", filename))
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White))
        .render(area, buf);
}

fn dependency_chain_text(chain: &[DependencyLink], area: Rect) -> Vec<Line> {
    chain
        .iter()
        .enumerate()
        .flat_map(|(index, link)| {
            // Each level will cascade further to the right
            let padding = index * 4 + 3;
            let padded_file_path = pad_left(&format!("└─➤ {}", link.sink), padding);

            let dependency_type = &link.dependency_type;
            let dependency_type_color = match dependency_type {
                DependencyType::Compile => Color::Red,
                DependencyType::Exports => Color::White,
                DependencyType::Runtime => Color::White,
            };

            [
                Line::from(pad_left("│", padding)),
                Line::from(vec![
                    Span::from(pad_left("│", padding)),
                    Span::from(format!(" ({})", dependency_type)).fg(dependency_type_color),
                ]),
                Line::from(pad_left("│", padding)),
                Line::from(padded_file_path),
            ]
        })
        .map(|mut l| {
            fill_line_width(&mut l, area.width);
            l
        })
        .collect()
}

fn pad_left(string: &str, amount: usize) -> String {
    format!("{}{}", " ".repeat(amount), string)
}

fn fill_line_width(line: &mut Line, width: u16) {
    let line_width = line.width();

    if line_width < width as usize {
        line.spans
            .push(Span::from(" ".repeat(width as usize - line_width)))
    }
}

#[cfg(test)]
mod handle_event_tests {
    use crate::adapter::NoopAdapter;
    use crate::RecomplileDependencyReason;
    use mpsc::Receiver;

    use super::*;

    fn noop_adapter() -> NoopAdapter {
        NoopAdapter::new()
    }

    fn widget() -> FileDependentPanel {
        FileDependentPanel::new(
            String::from("source"),
            recompile_dependencies(&["one", "two", "three"]),
        )
    }

    fn recompile_dependencies(files: &[&str]) -> Vec<RecomplileDependency> {
        files
            .into_iter()
            .map(|f| RecomplileDependency {
                id: f.to_string(),
                path: f.to_string(),
                reason: RecomplileDependencyReason::Compile,
                dependency_chain: vec![],
            })
            .collect()
    }

    fn dependency_chain() -> Vec<DependencyLink> {
        vec![
            DependencyLink {
                source: String::from("source"),
                sink: String::from("two.one"),
                dependency_type: DependencyType::Compile,
            },
            DependencyLink {
                source: String::from("two.one"),
                sink: String::from("two.two"),
                dependency_type: DependencyType::Compile,
            },
            DependencyLink {
                source: String::from("two.two"),
                sink: String::from("two.three"),
                dependency_type: DependencyType::Compile,
            },
        ]
    }

    fn collect_events(rx: Receiver<AppEvent>) -> Vec<AppEvent> {
        rx.try_iter().collect()
    }

    mod up_button {
        use super::*;

        #[test]
        fn up_button_no_expand() {
            let mut state = State::new();
            state.selected_file_index = (1, None);

            let (tx, rx) = mpsc::channel::<AppEvent>();

            state.handle_event(
                &AppEvent::UpButtonPressed,
                &widget(),
                &mut noop_adapter(),
                tx,
            );
            assert_eq!(state.selected_file_index, (0, None));
            assert_eq!(collect_events(rx).len(), 0);
        }

        #[test]
        fn up_button_with_expand() {
            let mut files = recompile_dependencies(&["one", "two", "three"]);
            files[1].dependency_chain = dependency_chain();
            let widget = FileDependentPanel::new(String::from("source"), files);

            let mut state = State::new();
            state.expanded_file = Some(String::from("two"));
            state.selected_file_index = (1, Some(1));

            let (tx, rx) = mpsc::channel::<AppEvent>();
            state.handle_event(&AppEvent::UpButtonPressed, &widget, &mut noop_adapter(), tx);
            assert_eq!(state.selected_file_index, (1, Some(0)));

            let events = collect_events(rx);
            assert_eq!(events.len(), 2);
            if let AppEvent::ViewDependentFile(ref dependency_link) = events[0] {
                assert_eq!(dependency_link.sink, "two.one");
            }

            if let AppEvent::StopViewDependentFile(ref dependency_link) = events[1] {
                assert_eq!(dependency_link.sink, "two.two");
            }
        }

        #[test]
        fn up_button_out_of_expand_list() {
            let mut files = recompile_dependencies(&["one", "two", "three"]);
            files[1].dependency_chain = dependency_chain();
            let widget = FileDependentPanel::new(String::from("source"), files);

            let mut state = State::new();
            state.expanded_file = Some(String::from("two"));
            state.selected_file_index = (1, Some(0));

            let (tx, rx) = mpsc::channel::<AppEvent>();
            state.handle_event(&AppEvent::UpButtonPressed, &widget, &mut noop_adapter(), tx);
            assert_eq!(state.selected_file_index, (1, None));
            let events = collect_events(rx);
            assert_eq!(events.len(), 1);
            if let AppEvent::StopViewDependentFile(ref dependency_link) = events[0] {
                assert_eq!(dependency_link.sink, "two.one");
            }
        }

        #[test]
        fn up_button_into_expand_list() {
            let mut files = recompile_dependencies(&["one", "two", "three"]);
            files[1].dependency_chain = dependency_chain();
            let widget = FileDependentPanel::new(String::from("source"), files);

            let mut state = State::new();
            state.expanded_file = Some(String::from("two"));
            state.selected_file_index = (2, None);

            let (tx, rx) = mpsc::channel::<AppEvent>();
            state.handle_event(&AppEvent::UpButtonPressed, &widget, &mut noop_adapter(), tx);
            assert_eq!(state.selected_file_index, (1, Some(2)));
            let events = collect_events(rx);
            assert_eq!(events.len(), 1);
            if let AppEvent::ViewDependentFile(ref dependency_link) = events[0] {
                assert_eq!(dependency_link.sink, "two.three");
            }
        }

        #[test]
        fn up_button_limit_no_expand() {
            let mut state = State::new();
            state.selected_file_index = (0, None);

            let (tx, rx) = mpsc::channel::<AppEvent>();
            state.handle_event(
                &AppEvent::UpButtonPressed,
                &widget(),
                &mut noop_adapter(),
                tx,
            );
            assert_eq!(state.selected_file_index, (0, None));
            assert_eq!(collect_events(rx).len(), 0);
        }
    }

    mod down_button {
        use super::*;

        #[test]
        fn down_button_no_expand() {
            let mut state = State::new();
            state.selected_file_index = (1, None);

            let (tx, rx) = mpsc::channel::<AppEvent>();
            state.handle_event(
                &AppEvent::DownButtonPressed,
                &widget(),
                &mut noop_adapter(),
                tx,
            );
            assert_eq!(state.selected_file_index, (2, None));
            assert_eq!(collect_events(rx).len(), 0);
        }

        #[test]
        fn down_button_limit_no_expand() {
            let mut state = State::new();
            state.selected_file_index = (2, None);

            let (tx, rx) = mpsc::channel::<AppEvent>();
            state.handle_event(
                &AppEvent::DownButtonPressed,
                &widget(),
                &mut noop_adapter(),
                tx,
            );
            assert_eq!(state.selected_file_index, (2, None));
            assert_eq!(collect_events(rx).len(), 0);
        }

        #[test]
        fn down_button_with_expand_list() {
            let mut files = recompile_dependencies(&["one", "two", "three"]);
            files[1].dependency_chain = dependency_chain();
            let widget = FileDependentPanel::new(String::from("source"), files);

            let mut state = State::new();
            state.expanded_file = Some(String::from("two"));
            state.selected_file_index = (1, Some(1));

            let (tx, rx) = mpsc::channel::<AppEvent>();
            state.handle_event(
                &AppEvent::DownButtonPressed,
                &widget,
                &mut noop_adapter(),
                tx,
            );
            assert_eq!(state.selected_file_index, (1, Some(2)));

            let events = collect_events(rx);
            assert_eq!(events.len(), 2);

            if let AppEvent::ViewDependentFile(ref dependency_link) = events[0] {
                assert_eq!(dependency_link.sink, "two.three");
            }

            if let AppEvent::StopViewDependentFile(ref dependency_link) = events[1] {
                assert_eq!(dependency_link.sink, "two.two");
            }
        }

        #[test]
        fn down_button_out_expand_list() {
            let mut files = recompile_dependencies(&["one", "two", "three"]);
            files[1].dependency_chain = dependency_chain();
            let widget = FileDependentPanel::new(String::from("source"), files);

            let mut state = State::new();
            state.expanded_file = Some(String::from("two"));
            state.selected_file_index = (1, Some(2));

            let (tx, rx) = mpsc::channel::<AppEvent>();
            state.handle_event(
                &AppEvent::DownButtonPressed,
                &widget,
                &mut noop_adapter(),
                tx,
            );
            assert_eq!(state.selected_file_index, (2, None));

            let events = collect_events(rx);
            assert_eq!(events.len(), 1);

            if let AppEvent::StopViewDependentFile(ref dependency_link) = events[0] {
                assert_eq!(dependency_link.sink, "two.three");
            }
        }

        #[test]
        fn down_button_into_expand_list() {
            let mut files = recompile_dependencies(&["one", "two", "three"]);
            files[1].dependency_chain = dependency_chain();
            let widget = FileDependentPanel::new(String::from("source"), files);

            let mut state = State::new();
            state.expanded_file = Some(String::from("two"));
            state.selected_file_index = (1, None);

            let (tx, rx) = mpsc::channel::<AppEvent>();
            state.handle_event(
                &AppEvent::DownButtonPressed,
                &widget,
                &mut noop_adapter(),
                tx,
            );
            assert_eq!(state.selected_file_index, (1, Some(0)));

            let events = collect_events(rx);
            assert_eq!(events.len(), 1);

            if let AppEvent::ViewDependentFile(ref dependency_link) = events[0] {
                assert_eq!(dependency_link.sink, "two.one");
            }
        }
    }

    mod select_file {
        use super::*;

        #[test]
        fn expand_file_from_initial() {
            let recompile_dependencies = recompile_dependencies(&["one", "two", "three"]);
            let widget =
                FileDependentPanel::new(String::from("source"), recompile_dependencies.clone());

            let mut state = State::new();
            let event = AppEvent::SelectDependentFile(recompile_dependencies[0].clone());
            let (tx, _) = mpsc::channel::<AppEvent>();
            state.handle_event(&event, &widget, &mut noop_adapter(), tx);
            assert_eq!(state.expanded_file, Some(String::from("one")));
        }

        #[test]
        fn expand_file_when_already_expanded() {
            let recompile_dependencies = recompile_dependencies(&["one", "two", "three"]);
            let widget =
                FileDependentPanel::new(String::from("source"), recompile_dependencies.clone());

            let mut state = State::new();
            state.expanded_file = Some(String::from("two"));

            let (tx, _) = mpsc::channel::<AppEvent>();
            let event = AppEvent::SelectDependentFile(recompile_dependencies[0].clone());
            state.handle_event(&event, &widget, &mut noop_adapter(), tx);
            assert_eq!(state.expanded_file, Some(String::from("one")));
        }

        #[test]
        fn collapse_file() {
            let recompile_dependencies = recompile_dependencies(&["one", "two", "three"]);
            let widget =
                FileDependentPanel::new(String::from("source"), recompile_dependencies.clone());

            let mut state = State::new();
            state.expanded_file = Some(String::from("two"));

            let (tx, _) = mpsc::channel::<AppEvent>();
            let event = AppEvent::SelectDependentFile(recompile_dependencies[1].clone());
            state.handle_event(&event, &widget, &mut noop_adapter(), tx);
            assert_eq!(state.expanded_file, None);
        }

        #[test]
        fn cancel_reset_state() {
            let recompile_dependencies = recompile_dependencies(&["one", "two", "three"]);
            let widget = FileDependentPanel::new(String::from("source"), recompile_dependencies);

            let mut state = State::new();
            state.selected_file_index = (2, None);

            let (tx, _) = mpsc::channel::<AppEvent>();
            state.handle_event(&AppEvent::Cancel, &widget, &mut noop_adapter(), tx);
            assert_eq!(state.expanded_file, None);
            assert_eq!(state.selected_file_index, (0, None));
            assert_eq!(state.selected_file_index, (0, None));
            assert_eq!(state.expanded_file, None);
        }
    }
}
