use std::process::Command;

pub struct Parser {
    verbose: bool,
}

#[derive(Debug, PartialEq)]
pub struct Section {
    pub file_name: String,
    pub line_start: i32,
    pub line_end: i32,
}

#[derive(Debug)]
pub struct SectionBuilder {
    file_name: Option<String>,
    line_start: Option<i32>,
    line_end: Option<i32>,
}

impl SectionBuilder {
    pub fn new() -> Self {
        Self {
            file_name: None,
            line_start: None,
            line_end: None,
        }
    }

    pub fn file_name(&mut self, file_name: String) {
        self.file_name = Some(file_name);
    }

    pub fn line_start(&mut self, line_start: i32) {
        self.line_start = Some(line_start);
    }

    pub fn line_end(&mut self, line_end: i32) {
        self.line_end = Some(line_end);
    }

    pub fn build(self) -> Option<Section> {
        match (self.file_name, self.line_start, self.line_end) {
            (Some(file_name), Some(line_start), Some(line_end)) => Some(Section {
                file_name,
                line_start,
                line_end,
            }),
            _ => None,
        }
    }
}

impl Parser {
    pub fn new() -> Self {
        Self { verbose: false }
    }

    pub fn set_verbose(&mut self, verbose: bool) -> &mut Self {
        self.verbose = verbose;
        self
    }

    pub fn get_sections(&self, target_branch: &str) -> Result<Vec<Section>, crate::error::Error> {
        self.diff(target_branch).map(|diff| self.sections(&diff))
    }

    fn diff(&self, target: &str) -> Result<String, crate::error::Error> {
        let cmd_output = Command::new("git")
            .args(&["diff", "-u", target])
            .output()
            .expect("Could not run git command.");
        if self.verbose {
            println!("{}", String::from_utf8(cmd_output.stdout.clone())?);
        }
        if cmd_output.status.success() {
            Ok(String::from_utf8(cmd_output.stdout)?)
        } else {
            Err(String::from_utf8(cmd_output.stderr)?.into())
        }
    }

    fn sections(&self, git_diff: &str) -> Vec<Section> {
        let mut sections = Vec::new();
        let mut file_name = "";
        for l in git_diff.lines() {
            // Add or edit a file
            // +++ b/Cargo.lock
            if l.starts_with("+++") {
                // TODO: do something less ugly with the bounds and indexing
                file_name = l[l.find('/').unwrap() + 1..].into();
            }
            // Actual diff lines
            // @@ -33,6 +33,9 @@ version = "0.1.0"
            if l.starts_with("@@") {
                // For now, we will focus on the added lines.
                // @@ and space
                let after_ats = &l[3..];
                // space and @@
                let before_second_ats_index = &after_ats.find("@@").unwrap() - 1;
                let diff_lines = &after_ats[..before_second_ats_index];
                // -33,6 +33,9
                let (_, a) = diff_lines.split_at(diff_lines.find(' ').unwrap());
                let added = a.trim();

                let (added_start, added_span) = if let Some(index) = added[1..].find(',') {
                    let (a, b) = added[1..].split_at(index);
                    (a, &b[1..])
                } else {
                    (added, "")
                };
                let min_line_start = added_start.parse::<i32>().unwrap();
                let mut current_section = SectionBuilder::new();
                current_section.file_name(file_name.to_string());
                current_section.line_start(min_line_start);
                current_section.line_end(min_line_start + added_span.parse::<i32>().unwrap_or(1));
                if let Some(s) = current_section.build() {
                    sections.push(s);
                }
            }
        }
        sections
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_set_verbose() {
        use crate::git::Parser;

        let mut parser = Parser::new();
        assert_eq!(false, parser.verbose);

        let p2 = parser.set_verbose(true);
        assert_eq!(true, p2.verbose);

        let p3 = p2.set_verbose(false);
        assert_eq!(false, p3.verbose);
    }
    #[test]
    fn test_empty_diff() {
        use crate::git::{Parser, Section};
        // Setup
        let diff = r#""#;
        let expected_sections: Vec<Section> = vec![];
        let parser = Parser::new();
        // Run
        let actual_sections = parser.sections(diff);
        // Assert
        assert_eq!(expected_sections, actual_sections);
    }
    #[test]
    fn test_simple_diff() {
        use crate::git::{Parser, Section};
        // Setup
        let diff = std::fs::read_to_string("test_files/git/one_diff.patch").unwrap();
        let expected_sections: Vec<Section> = vec![
            Section {
                file_name: "src/git.rs".to_string(),
                line_start: 4,
                line_end: 11,
            },
            Section {
                file_name: "src/git.rs".to_string(),
                line_start: 117,
                line_end: 147,
            },
        ];
        let parser = Parser::new();
        // Run
        let actual_sections = parser.sections(&diff);
        // Assert
        assert_eq!(expected_sections, actual_sections);
    }
    #[test]
    fn test_diff_several_files() {
        use crate::git::{Parser, Section};
        // Setup
        let diff = std::fs::read_to_string("test_files/git/diff_several_files.patch").unwrap();
        let expected_sections: Vec<Section> = vec![
            Section {
                file_name: "src/clippy.rs".to_string(),
                line_start: 124,
                line_end: 129,
            },
            Section {
                file_name: "src/git.rs".to_string(),
                line_start: 4,
                line_end: 11,
            },
            Section {
                file_name: "src/git.rs".to_string(),
                line_start: 117,
                line_end: 181,
            },
        ];
        let parser = Parser::new();
        // Run
        let actual_sections = parser.sections(&diff);
        // Assert
        assert_eq!(expected_sections, actual_sections);
    }
}
