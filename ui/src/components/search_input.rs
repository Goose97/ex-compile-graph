use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::utils;

#[derive(Clone)]
pub enum State {
    Prompt(String),
    Search(String),
}

#[derive(Clone)]
pub struct SearchInput {
    state: State,
}

impl SearchInput {
    pub fn new(state: State) -> Self {
        Self { state }
    }
}

impl Widget for SearchInput {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rect = utils::padding(&area, 1, 0);

        let paragraph = match self.state {
            State::Prompt(input) => {
                Paragraph::new(Line::from(vec![Span::from("Search: "), Span::from(input)]))
                    .style(Style::default().fg(Color::Cyan))
            }

            State::Search(query) => Paragraph::new(Line::from(vec![
                Span::from(format!("Search: {}", query)),
                Span::styled(
                    " | <esc> to exit search",
                    Style::default().fg(Color::Yellow),
                ),
            ]))
            .style(Style::default().fg(Color::Cyan)),
        };

        paragraph.render(rect, buf);
    }
}
