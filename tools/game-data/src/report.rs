//! What the generator leaves out or finds odd, written next to the data it generates
//! (`report.md`): after a patch of the game, a diff of this file shows what appeared and what
//! went, instead of lines on the standard output that nobody keeps.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

/// The notes of a run, by section, each section sorted.
#[derive(Default)]
pub struct Report {
    sections: BTreeMap<&'static str, BTreeSet<String>>,
}

impl Report {
    /// Adds `note` to `section`, and prints it.
    pub fn note(&mut self, section: &'static str, note: impl Into<String>) {
        let note = note.into();
        println!("{section}: {note}");
        self.sections.entry(section).or_default().insert(note);
    }

    /// The file of the report, read from `sources` (the commits the data come from).
    pub fn to_markdown(&self, sources: &str) -> String {
        let mut text = format!(
            "# Game data report\n\nWritten by `tools/game-data` with the data, from {sources}. \
             Not read by the application: what the generator left out or found odd, to compare \
             with the report of the previous patch (`git diff`).\n"
        );
        for (section, notes) in &self.sections {
            // Writing to a `String` never fails.
            let _ = write!(text, "\n## {section}\n\n");
            for note in notes {
                let _ = writeln!(text, "- `{note}`");
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_notes_are_written_by_section_sorted_and_once() {
        let mut report = Report::default();
        report.note("Second", "b");
        report.note("First", "z");
        report.note("Second", "a");
        report.note("Second", "b");

        let text = report.to_markdown("a commit");

        assert!(text.starts_with("# Game data report\n\nWritten by"));
        assert!(text.contains("from a commit."));
        assert!(text.ends_with("\n## First\n\n- `z`\n\n## Second\n\n- `a`\n- `b`\n"));
    }
}
