use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::utils;

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    None,
    Prompt(String),
    Search(String),
}

impl State {
    // Input is either in prompting or searching state
    pub fn is_active(&self) -> bool {
        match self {
            Self::Search(_) => true,
            Self::Prompt(_) => true,
            _ => false,
        }
    }

    pub fn is_prompting(&self) -> bool {
        match self {
            Self::Search(_) => false,
            Self::Prompt(_) => true,
            _ => false,
        }
    }

    pub fn prompt_input(&self) -> Option<String> {
        match self {
            Self::Prompt(input) => Some(input.clone()),
            _ => None,
        }
    }

    pub fn prompt_begin(&mut self) {
        if let Self::None = self {
            *self = Self::Prompt(String::new());
        }
    }

    pub fn prompt_add(&mut self, char: char) {
        if let Self::Prompt(input) = self {
            input.push(char);
        }
    }

    pub fn prompt_remove(&mut self) {
        if let Self::Prompt(input) = self {
            input.pop();
        }
    }

    pub fn search(&mut self) {
        if let Self::Prompt(input) = self {
            *self = Self::Search(input.clone());
        }
    }

    pub fn cancel(&mut self) {
        match self {
            Self::Prompt(_) => *self = Self::None,
            Self::Search(_) => *self = Self::None,
            _ => {}
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::None
    }
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
            State::None => Paragraph::new(""),
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
