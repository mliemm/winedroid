use fancy_regex::Regex as BacktrackingRegex;
use regex::Regex as AutomatonRegex;

// cite: libcore Pattern.java#compile
pub(super) enum JavaRegex {
    Automaton(AutomatonRegex),
    Backtracking(BacktrackingRegex),
}

pub(super) type JavaCaptures = Vec<Option<(usize, usize)>>;

impl JavaRegex {
    pub(super) fn new(pattern: &str) -> Option<Self> {
        if let Ok(regex) = AutomatonRegex::new(pattern) {
            return Some(Self::Automaton(regex));
        }

        BacktrackingRegex::new(pattern).ok().map(Self::Backtracking)
    }

    pub(super) fn captures_at(&self, input: &str, start: usize) -> Option<Option<JavaCaptures>> {
        match self {
            Self::Automaton(regex) => Some(regex.captures_at(input, start).map(|captures| {
                captures
                    .iter()
                    .map(|group| group.map(|matched| (matched.start(), matched.end())))
                    .collect()
            })),
            Self::Backtracking(regex) => {
                let captures = regex.captures_from_pos(input, start).ok()?;

                Some(captures.map(|captures| {
                    captures
                        .iter()
                        .map(|group| group.map(|matched| (matched.start(), matched.end())))
                        .collect()
                }))
            }
        }
    }

    pub(super) fn captures(&self, input: &str) -> Option<Option<JavaCaptures>> {
        self.captures_at(input, 0)
    }

    pub(super) fn find_at(&self, input: &str, start: usize) -> Option<Option<(usize, usize)>> {
        match self {
            Self::Automaton(regex) => Some(
                regex
                    .find_at(input, start)
                    .map(|found| (found.start(), found.end())),
            ),
            Self::Backtracking(regex) => {
                let found = regex.find_from_pos(input, start).ok()?;

                Some(found.map(|found| (found.start(), found.end())))
            }
        }
    }

    pub(super) fn replace_all(&self, input: &str, replacement: &str) -> Option<String> {
        match self {
            Self::Automaton(regex) => Some(regex.replace_all(input, replacement).into_owned()),
            Self::Backtracking(regex) => regex
                .try_replacen(input, 0, replacement)
                .ok()
                .map(|replaced| replaced.into_owned()),
        }
    }
}
