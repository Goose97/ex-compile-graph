use crate::{DependencyCause, DependencyLink, FileEntry, RecomplileDependency};

#[derive(Debug)]
pub enum AppEvent {
    UpButtonPressed,
    DownButtonPressed,

    SelectFile(FileEntry),
    SelectDependentFile(RecomplileDependency),
    ViewDependentFile(DependencyLink),
    StopViewDependentFile(DependencyLink),

    GetFilesDone(Vec<FileEntry>),
    GetDependencyCausesDone(Vec<DependencyCause>),

    Cancel,
    Quit,
}
