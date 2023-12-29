use ratatui::widgets::StatefulWidget;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::mpsc;

use crate::app_event::AppEvent;
use adapter::ServerAdapter;

pub mod adapter;
pub mod app_event;
pub mod app_state;
pub mod components;
pub mod utils;

pub static mut FRAME_COUNT: usize = 0;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RecomplileDependencyReason {
    #[serde(rename = "compile")]
    Compile,
    #[serde(rename = "exports_then_compile")]
    ExportsThenCompile,
    #[serde(rename = "exports")]
    Exports,
    #[serde(rename = "compile_then_runtime")]
    CompileThenRuntime,
}

#[derive(Deserialize, Debug, Clone)]
pub enum DependencyType {
    #[serde(rename = "compile")]
    Compile,
    #[serde(rename = "exports")]
    Exports,
    #[serde(rename = "runtime")]
    Runtime,
}

impl Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            DependencyType::Compile => "compile",
            DependencyType::Exports => "exports",
            DependencyType::Runtime => "runtime",
        };

        write!(f, "{}", text)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct DependencyLink {
    dependency_type: DependencyType,
    source: FilePath,
    sink: FilePath,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RecomplileDependency {
    id: String,
    path: FilePath,
    reason: RecomplileDependencyReason,
    dependency_chain: Vec<DependencyLink>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub path: FilePath,
    pub recompile_dependencies: Vec<RecomplileDependency>,
}

pub type FilePath = String;

#[derive(Deserialize, Debug, Clone)]
pub struct DependencyCause {
    pub source: FilePath,
    pub sink: FilePath,
    pub snippets: Vec<CodeSnippet>,
    #[serde(rename = "type")]
    pub dependency_type: DependencyType,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct CodeSnippet {
    content: String,
    highlight: (usize, usize),
    lines_span: (usize, usize),
}

pub trait HandleEvent {
    type Widget: StatefulWidget;

    // Handle incoming events, return a list of newly created events
    fn handle_event(
        &mut self,
        _event: &AppEvent,
        _widget: &Self::Widget,
        _adapter: &mut impl ServerAdapter,
        _dispatcher: mpsc::Sender<AppEvent>,
    );
}

pub trait ProduceEvent {
    type Widget: StatefulWidget;

    fn produce_event(
        &mut self,
        _terminal_event: &crossterm::event::Event,
        _widget: &Self::Widget,
    ) -> Option<AppEvent> {
        None
    }
}

impl Into<FilePath> for FileEntry {
    fn into(self) -> FilePath {
        self.path
    }
}

impl Into<FilePath> for RecomplileDependency {
    fn into(self) -> FilePath {
        self.path
    }
}
