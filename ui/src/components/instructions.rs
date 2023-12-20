use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::utils;

#[derive(Clone)]
pub struct Instructions {}

impl Instructions {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct State {}

impl Widget for Instructions {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rect = utils::padding(&area, 1, 0);
        let paragraph = Paragraph::new(Line::from(vec![
            Span::from("j/k: Move; "),
            Span::from("<enter>: Select"),
        ]))
        .style(Style::default().fg(Color::Yellow));

        paragraph.render(rect, buf);
    }
}
