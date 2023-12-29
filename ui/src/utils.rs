use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::cmp;
use std::cmp::Reverse;

use crate::components::search_input;
use crate::FilePath;

#[allow(dead_code)]
pub fn max_height(rect: &Rect, max: u16) -> Rect {
    let after = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Max(max), Constraint::Min(0)])
        .split(*rect)[0];

    after
}

pub fn max_width(rect: &Rect, max: u16) -> Rect {
    let after = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Max(max), Constraint::Min(0)])
        .split(*rect)[0];

    after
}

// Returns the rect after apply transform
pub fn transform(rect: &Rect, x: i16, y: i16) -> Rect {
    let after_x: i16 = (rect.x as i16) + x;
    let after_y: i16 = (rect.y as i16) + y;

    Rect {
        x: cmp::max(after_x, 0) as u16,
        y: cmp::max(after_y, 0) as u16,
        width: rect.width,
        height: rect.height,
    }
}

// Returns the inner rect after apply padding
pub fn padding(rect: &Rect, padding_x: i16, padding_y: i16) -> Rect {
    if padding_x as u16 * 2 > rect.width {
        panic!(
            "Padding x is too big, padding = {}, width = {}",
            padding_x, rect.width
        )
    }

    if padding_y as u16 * 2 > rect.height {
        panic!(
            "Padding y is too big, padding = {}, width = {}",
            padding_y, rect.height
        )
    }

    Rect {
        x: rect.x + padding_x as u16,
        y: rect.y + padding_y as u16,
        width: rect.width - padding_x as u16 * 2,
        height: rect.height - padding_y as u16 * 2,
    }
}

/// Center a child rect inside a container rect
/// The child MUST completely fits within the container
pub fn center_rect_in_container(child: &mut Rect, container: &Rect) {
    if child.width > container.width || child.height > container.height {
        panic!("A child Rect must fit within the container Rect")
    }

    let center_x = container.x + container.width / 2;
    let center_y = container.y + container.height / 2;

    child.x = center_x - child.width / 2;
    child.y = center_y - child.height / 2;
}

/// Compact a file path to fit a maximum width. If the file path is longer than the maximum
/// width, it will get truncated and have the leading ...
///
/// # Examples
///
/// The file path doesn't get truncated
///
/// ```
/// use crate::utils;
/// let result = utils::compact_file_path("a/b/c/d", 10);
/// assert_eq!(result, "a/b/c/d");
/// ```
///
/// The file path is truncated
///
/// ```
/// use crate::utils;
/// let result = utils::compact_file_path("a/b/c/d", 5);
/// assert_eq!(result, ".../d");
/// ```
pub fn compact_file_path(file_path: &str, maximum: usize) -> String {
    if file_path.len() <= maximum {
        return file_path.to_string();
    }

    let substrings: Vec<&str> = file_path.split('/').collect();
    let mut result: Vec<&str> = vec!["..."];
    let mut truncated = false;

    for substring in substrings.into_iter().rev() {
        let total_length = result.len() - 1 + result.iter().map(|f| f.len()).sum::<usize>();
        if total_length + substring.len() + 1 > maximum {
            truncated = true;
            break;
        }

        result.insert(1, substring);
    }

    if !truncated {
        result.pop();
    }

    result.join("/")
}

pub fn filter_files_list<'a, 'b, T: Into<FilePath> + Clone>(
    files: &'a [T],
    search_term: &search_input::State,
) -> Vec<T> {
    match search_term {
        search_input::State::Search(term) => {
            let matcher = SkimMatcherV2::default();

            let mut filtered = files
                .iter()
                .filter_map(|file| {
                    let file_path: FilePath = file.clone().into();
                    let score = matcher.fuzzy_match(&file_path, term);

                    match score {
                        Some(score) if score > 0 => Some((file, score)),
                        _ => None,
                    }
                })
                .collect::<Vec<(&T, i64)>>();

            filtered.sort_by_key(|item| Reverse(item.1));
            filtered.into_iter().map(|(file, _)| file.clone()).collect()
        }

        _ => files.to_vec(),
    }
}

#[cfg(test)]
mod max_height_tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn bigger_than_max_height() {
        let rect = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };

        let result = max_height(&rect, 5);

        assert_eq!(
            result,
            Rect {
                x: 0,
                y: 0,
                width: 10,
                height: 5
            }
        )
    }

    #[test]
    fn smaller_than_max_height() {
        let rect = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };

        let result = max_height(&rect, 15);

        assert_eq!(
            result,
            Rect {
                x: 0,
                y: 0,
                width: 10,
                height: 10
            }
        )
    }
}

#[cfg(test)]
mod max_width_tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn bigger_than_max_width() {
        let rect = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };

        let result = max_width(&rect, 5);

        assert_eq!(
            result,
            Rect {
                x: 0,
                y: 0,
                width: 5,
                height: 10
            }
        )
    }

    #[test]
    fn smaller_than_max_width() {
        let rect = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };

        let result = max_width(&rect, 15);

        assert_eq!(
            result,
            Rect {
                x: 0,
                y: 0,
                width: 10,
                height: 10
            }
        )
    }
}

#[cfg(test)]
mod transform_tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn normal() {
        let rect = Rect {
            x: 5,
            y: 5,
            width: 10,
            height: 10,
        };

        let result = transform(&rect, 2, -2);

        assert_eq!(
            result,
            Rect {
                x: 7,
                y: 3,
                width: 10,
                height: 10
            }
        )
    }

    #[test]
    fn exceed_limit() {
        let rect = Rect {
            x: 2,
            y: 4,
            width: 10,
            height: 10,
        };

        let result = transform(&rect, -5, -5);

        assert_eq!(
            result,
            Rect {
                x: 0,
                y: 0,
                width: 10,
                height: 10
            }
        )
    }
}

#[cfg(test)]
mod padding_tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn normal() {
        let rect = Rect {
            x: 2,
            y: 3,
            width: 10,
            height: 10,
        };

        let result = padding(&rect, 2, 1);

        assert_eq!(
            result,
            Rect {
                x: 4,
                y: 4,
                width: 6,
                height: 8
            }
        )
    }

    #[test]
    #[should_panic]
    fn exceed_limit() {
        let rect = Rect {
            x: 4,
            y: 4,
            width: 6,
            height: 10,
        };

        padding(&rect, 4, 4);
    }
}

#[cfg(test)]
mod center_rect_in_container_tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn child_fits_within_container() {
        let mut child = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };

        let container = Rect {
            x: 10,
            y: 10,
            width: 20,
            height: 20,
        };

        center_rect_in_container(&mut child, &container);

        assert_eq!(
            child,
            Rect {
                x: 15,
                y: 15,
                width: 10,
                height: 10
            }
        )
    }

    #[test]
    #[should_panic]
    fn child_does_not_fit_within_container() {
        let mut child = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };

        let container = Rect {
            x: 10,
            y: 10,
            width: 8,
            height: 20,
        };

        center_rect_in_container(&mut child, &container);
    }
}

#[cfg(test)]
mod compact_file_path_tests {
    use super::*;

    #[test]
    fn no_truncate() {
        let path = "a/b/c/d";
        let result = compact_file_path(path, 8);
        assert_eq!(result, "a/b/c/d");
    }

    #[test]
    fn truncate() {
        let path = "a/b/c/d";
        let result = compact_file_path(path, 5);
        assert_eq!(result, ".../d");
    }
}

#[cfg(test)]
mod filter_list_tests {
    use super::*;
    use crate::FileEntry;

    fn term(input: &str) -> search_input::State {
        search_input::State::Search(String::from(input))
    }

    #[test]
    fn found_one() {
        let files = file_entries(&["one", "two", "three"]);
        let filtered: Vec<String> = filter_files_list(&files, &term("one"))
            .into_iter()
            .map(|f| f.path)
            .collect();

        assert_eq!(filtered, vec!["one"]);
    }

    #[test]
    fn found_many_and_sort_score() {
        let files = file_entries(&["one", "two_one", "three_two"]);
        let filtered: Vec<String> = filter_files_list(&files, &term("one"))
            .into_iter()
            .map(|f| f.path)
            .collect();

        assert_eq!(filtered, vec!["one", "two_one"]);
    }

    #[test]
    fn found_none() {
        let files = file_entries(&["one", "two", "three"]);
        let filtered: Vec<String> = filter_files_list(&files, &term("four"))
            .into_iter()
            .map(|f| f.path)
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
