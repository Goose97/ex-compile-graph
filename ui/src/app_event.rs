use crate::{DependencyLink, FileEntry, RecomplileDependency};

#[derive(Debug)]
pub enum AppEvent {
    UpButtonPressed,
    DownButtonPressed,
    SelectFile(FileEntry),
    SelectDependentFile(RecomplileDependency),
    ViewDependentFile(DependencyLink),
    StopViewDependentFile(DependencyLink),
    Cancel,
    Quit,
}
