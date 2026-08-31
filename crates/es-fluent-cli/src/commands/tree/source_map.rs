#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SourcePosition {
    pub(super) line: usize,
    pub(super) column: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct EntryLocation {
    pub(super) id_position: SourcePosition,
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) struct FtlSourceMap<'a> {
    pub(super) source: &'a str,
    pub(super) line_starts: Vec<usize>,
}

impl<'a> FtlSourceMap<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' && offset + 1 < source.len() {
                line_starts.push(offset + 1);
            }
        }

        Self {
            source,
            line_starts,
        }
    }

    pub(super) fn find_message(&self, id: &str) -> Option<EntryLocation> {
        self.find_entry(id, EntryKind::Message)
    }

    pub(super) fn find_term(&self, id: &str) -> Option<EntryLocation> {
        self.find_entry(id, EntryKind::Term)
    }

    pub(super) fn find_attribute(&self, entry: EntryLocation, id: &str) -> Option<SourcePosition> {
        let first_line = self.line_index(entry.start);
        let last_line = self.line_index(entry.end.saturating_sub(1));

        for line_index in first_line..=last_line {
            let line = self.line(line_index);
            let trimmed = line.trim_start();
            let leading = line.len() - trimmed.len();
            let Some(rest) = trimmed.strip_prefix('.') else {
                continue;
            };
            let Some(after_id) = rest.strip_prefix(id) else {
                continue;
            };
            if after_id.trim_start().starts_with('=') {
                return Some(self.position(self.line_starts[line_index] + leading));
            }
        }

        None
    }

    pub(super) fn find_variable(&self, entry: EntryLocation, name: &str) -> Option<SourcePosition> {
        let needle = format!("${name}");
        let mut offset = entry.start;

        while offset < entry.end {
            let relative = self.source[offset..entry.end].find(&needle)?;
            let candidate = offset + relative;
            let after = candidate + needle.len();
            if self.is_variable_boundary(after) {
                return Some(self.position(candidate));
            }
            offset = after;
        }

        None
    }

    pub(super) fn find_entry(&self, id: &str, kind: EntryKind) -> Option<EntryLocation> {
        for line_index in 0..self.line_starts.len() {
            let line = self.line(line_index);
            let trimmed = line.trim_start();
            let leading = line.len() - trimmed.len();

            let id_offset = match kind {
                EntryKind::Message => message_id_offset(trimmed, id),
                EntryKind::Term => term_id_offset(trimmed, id),
            };

            if let Some(id_offset) = id_offset {
                let start = self.line_starts[line_index] + leading;
                let id_start = start + id_offset;
                return Some(EntryLocation {
                    id_position: self.position(id_start),
                    start,
                    end: self.entry_end(line_index),
                });
            }
        }

        None
    }

    pub(super) fn entry_end(&self, start_line: usize) -> usize {
        for line_index in start_line + 1..self.line_starts.len() {
            let line = self.line(line_index);
            let trimmed = line.trim_start();
            if line.len() == trimmed.len() && top_level_entry_start(trimmed) {
                return self.line_starts[line_index];
            }
        }

        self.source.len()
    }

    pub(super) fn position(&self, offset: usize) -> SourcePosition {
        let line_index = self.line_index(offset);
        SourcePosition {
            line: line_index + 1,
            column: offset.saturating_sub(self.line_starts[line_index]) + 1,
        }
    }

    pub(super) fn line_index(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        }
    }

    pub(super) fn line(&self, index: usize) -> &str {
        let start = self.line_starts[index];
        let end = self
            .line_starts
            .get(index + 1)
            .map_or(self.source.len(), |next| next.saturating_sub(1));
        &self.source[start..end]
    }

    pub(super) fn is_variable_boundary(&self, offset: usize) -> bool {
        self.source[offset..]
            .chars()
            .next()
            .is_none_or(|ch| !is_identifier_continue(ch))
    }
}

#[derive(Clone, Copy)]
pub(super) enum EntryKind {
    Message,
    Term,
}

pub(super) fn message_id_offset(line: &str, id: &str) -> Option<usize> {
    let rest = line.strip_prefix(id)?;
    rest.trim_start().starts_with('=').then_some(0)
}

pub(super) fn term_id_offset(line: &str, id: &str) -> Option<usize> {
    let rest = line.strip_prefix('-')?.strip_prefix(id)?;
    rest.trim_start().starts_with('=').then_some(0)
}

pub(super) fn top_level_entry_start(line: &str) -> bool {
    if line.is_empty() || line.starts_with('}') {
        return false;
    }
    line.starts_with('#')
        || term_entry_start(line)
        || line
            .chars()
            .next()
            .is_some_and(|ch| is_identifier_start(ch) && line.contains('='))
}

pub(super) fn term_entry_start(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('-') else {
        return false;
    };
    rest.chars()
        .next()
        .is_some_and(|ch| is_identifier_start(ch) && line.contains('='))
}

pub(super) fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
}

pub(super) fn is_identifier_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
}
