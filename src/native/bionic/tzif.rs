use super::civil::days_from_civil;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalType {
    pub offset: i32,
    pub is_dst: bool,
    pub abbreviation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleDay {
    Julian(i64),
    ZeroBased(i64),
    MonthWeekDay { month: i64, week: i64, weekday: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub day: RuleDay,
    pub seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posix {
    pub standard: LocalType,
    pub daylight: Option<(LocalType, Rule, Rule)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    pub transitions: Vec<(i64, usize)>,
    pub types: Vec<LocalType>,
    pub footer: Option<Posix>,
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn be64(bytes: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_be_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

struct Counts {
    is_ut: usize,
    is_std: usize,
    leap: usize,
    time: usize,
    kind: usize,
    chars: usize,
}

fn counts(bytes: &[u8], at: usize) -> Option<Counts> {
    if bytes.get(at..at + 4)? != b"TZif" {
        return None;
    }
    let count = |index: usize| be32(bytes, at + 20 + index * 4).map(|value| value as usize);
    Some(Counts { is_ut: count(0)?, is_std: count(1)?, leap: count(2)?, time: count(3)?, kind: count(4)?, chars: count(5)? })
}

fn block_size(counts: &Counts, time_width: usize) -> usize {
    counts.time * time_width + counts.time + counts.kind * 6 + counts.chars + counts.leap * (time_width + 4) + counts.is_std + counts.is_ut
}

pub fn parse(bytes: &[u8]) -> Option<Zone> {
    // cite: bionic localtime.c#tzloadbody
    let first = counts(bytes, 0)?;
    let version = *bytes.get(4)?;
    let (header, width, counts) = if version >= b'2' {
        let second = 44 + block_size(&first, 4);
        (second, 8, counts(bytes, second)?)
    } else {
        (0, 4, first)
    };
    let mut at = header + 44;
    let mut times = Vec::with_capacity(counts.time);
    for index in 0..counts.time {
        times.push(if width == 8 { be64(bytes, at + index * 8)? } else { i64::from(be32(bytes, at + index * 4)? as i32) });
    }
    at += counts.time * width;
    let indices = bytes.get(at..at + counts.time)?.to_vec();
    at += counts.time;
    let type_at = at;
    at += counts.kind * 6;
    let characters = bytes.get(at..at + counts.chars)?;
    let mut types = Vec::with_capacity(counts.kind);
    for index in 0..counts.kind {
        let entry = bytes.get(type_at + index * 6..type_at + index * 6 + 6)?;
        let start = usize::from(entry[5]);
        let end = characters.get(start..)?.iter().position(|byte| *byte == 0)? + start;
        types.push(LocalType { offset: i32::from_be_bytes(entry[..4].try_into().ok()?), is_dst: entry[4] != 0, abbreviation: String::from_utf8_lossy(&characters[start..end]).into_owned() });
    }
    let transitions = times.into_iter().zip(indices).map(|(time, index)| (time, usize::from(index))).collect();
    let after = at + counts.chars + counts.leap * (width + 4) + counts.is_std + counts.is_ut;
    let footer = bytes
        .get(after..)
        .and_then(|rest| rest.strip_prefix(b"\n"))
        .and_then(|rest| rest.iter().position(|byte| *byte == b'\n').map(|end| &rest[..end]))
        .filter(|text| !text.is_empty())
        .and_then(|text| parse_posix(std::str::from_utf8(text).ok()?));
    Some(Zone { transitions, types, footer })
}

struct Reader<'a> {
    text: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.at).copied()
    }

    fn number(&mut self) -> Option<i64> {
        let start = self.at;
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.at += 1;
        }
        std::str::from_utf8(&self.text[start..self.at]).ok()?.parse().ok()
    }

    fn name(&mut self) -> Option<String> {
        if self.peek() == Some(b'<') {
            let end = self.text[self.at..].iter().position(|byte| *byte == b'>')? + self.at;
            let name = String::from_utf8_lossy(&self.text[self.at + 1..end]).into_owned();
            self.at = end + 1;
            return Some(name);
        }
        let start = self.at;
        while self.peek().is_some_and(|byte| byte.is_ascii_alphabetic()) {
            self.at += 1;
        }
        (self.at - start >= 3).then(|| String::from_utf8_lossy(&self.text[start..self.at]).into_owned())
    }

    fn clock(&mut self) -> Option<i64> {
        let sign = match self.peek() {
            Some(b'-') => {
                self.at += 1;
                -1
            }
            Some(b'+') => {
                self.at += 1;
                1
            }
            _ => 1,
        };
        let mut seconds = self.number()? * 3600;
        for unit in [60, 1] {
            if self.peek() != Some(b':') {
                break;
            }
            self.at += 1;
            seconds += self.number()? * unit;
        }
        Some(sign * seconds)
    }

    fn rule(&mut self) -> Option<Rule> {
        let day = match self.peek()? {
            b'J' => {
                self.at += 1;
                RuleDay::Julian(self.number()?)
            }
            b'M' => {
                self.at += 1;
                let month = self.number()?;
                self.at += 1;
                let week = self.number()?;
                self.at += 1;
                RuleDay::MonthWeekDay { month, week, weekday: self.number()? }
            }
            _ => RuleDay::ZeroBased(self.number()?),
        };
        let seconds = if self.peek() == Some(b'/') {
            self.at += 1;
            self.clock()?
        } else {
            7200
        };
        Some(Rule { day, seconds })
    }
}

pub fn parse_posix(text: &str) -> Option<Posix> {
    // cite: bionic localtime.c#tzparse
    let mut reader = Reader { text: text.as_bytes(), at: 0 };
    let standard_name = reader.name()?;
    let standard_offset = -reader.clock()?;
    let standard = LocalType { offset: standard_offset as i32, is_dst: false, abbreviation: standard_name };
    if reader.peek().is_none() {
        return Some(Posix { standard, daylight: None });
    }
    let daylight_name = reader.name()?;
    let daylight_offset = if matches!(reader.peek(), Some(b',') | None) { standard_offset + 3600 } else { -reader.clock()? };
    let (start, end) = if reader.peek() == Some(b',') {
        reader.at += 1;
        let start = reader.rule()?;
        if reader.peek() != Some(b',') {
            return None;
        }
        reader.at += 1;
        (start, reader.rule()?)
    } else {
        (Rule { day: RuleDay::MonthWeekDay { month: 3, week: 2, weekday: 0 }, seconds: 7200 }, Rule { day: RuleDay::MonthWeekDay { month: 11, week: 1, weekday: 0 }, seconds: 7200 })
    };
    Some(Posix { standard, daylight: Some((LocalType { offset: daylight_offset as i32, is_dst: true, abbreviation: daylight_name }, start, end)) })
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn rule_seconds(year: i64, rule: &Rule) -> i64 {
    let year_start = days_from_civil(year, 1, 1);
    let day = match &rule.day {
        RuleDay::Julian(number) => year_start + number - 1 + i64::from(is_leap(year) && *number >= 60),
        RuleDay::ZeroBased(number) => year_start + number,
        RuleDay::MonthWeekDay { month, week, weekday } => {
            let first = days_from_civil(year, *month, 1);
            let first_weekday = (first + 4).rem_euclid(7);
            let mut day = first + (weekday - first_weekday).rem_euclid(7) + (week - 1) * 7;
            let next_month = if *month == 12 { days_from_civil(year + 1, 1, 1) } else { days_from_civil(year, month + 1, 1) };
            while day >= next_month {
                day -= 7;
            }
            day
        }
    };
    day * 86_400 + rule.seconds
}

const EPOCH_YEAR: i64 = 1970;

impl Posix {
    pub fn at_from_epoch(&self, seconds: i64) -> &LocalType {
        // cite: bionic localtime.c#tzparse
        if let Some((daylight, start, end)) = &self.daylight {
            let first = (rule_seconds(EPOCH_YEAR, start) - i64::from(self.standard.offset)).min(rule_seconds(EPOCH_YEAR, end) - i64::from(daylight.offset));
            if seconds < first {
                return &self.standard;
            }
        }
        self.at(seconds)
    }

    pub fn at(&self, seconds: i64) -> &LocalType {
        let Some((daylight, start, end)) = &self.daylight else { return &self.standard };
        let year = super::civil::from_seconds(seconds + i64::from(self.standard.offset)).map_or(1970, |parts| i64::from(parts.year) + 1900);
        let start_utc = rule_seconds(year, start) - i64::from(self.standard.offset);
        let end_utc = rule_seconds(year, end) - i64::from(daylight.offset);
        let in_daylight = if start_utc < end_utc { seconds >= start_utc && seconds < end_utc } else { seconds >= start_utc || seconds < end_utc };
        if in_daylight {
            daylight
        } else {
            &self.standard
        }
    }
}

impl Zone {
    pub fn from_posix(posix: Posix) -> Self {
        Zone { transitions: Vec::new(), types: Vec::new(), footer: Some(posix) }
    }

    pub fn utc() -> Self {
        Zone::from_posix(Posix { standard: LocalType { offset: 0, is_dst: false, abbreviation: "GMT".to_string() }, daylight: None })
    }

    fn default_type(&self) -> Option<&LocalType> {
        self.types.iter().find(|entry| !entry.is_dst).or_else(|| self.types.first())
    }

    pub fn at(&self, seconds: i64) -> LocalType {
        // cite: bionic localtime.c#localsub
        if let (Some(posix), Some(last)) = (&self.footer, self.transitions.last()) {
            if seconds >= last.0 {
                return posix.at(seconds).clone();
            }
        }
        if self.transitions.is_empty() {
            return match (&self.footer, self.default_type()) {
                (Some(posix), _) => posix.at_from_epoch(seconds).clone(),
                (None, Some(kind)) => kind.clone(),
                (None, None) => Zone::utc().types.first().cloned().unwrap_or(LocalType { offset: 0, is_dst: false, abbreviation: "GMT".to_string() }),
            };
        }
        let position = self.transitions.partition_point(|(at, _)| *at <= seconds);
        if position == 0 {
            return self.default_type().cloned().expect("a zone with transitions has types");
        }
        self.types[self.transitions[position - 1].1].clone()
    }

    pub fn standard_and_daylight(&self) -> (LocalType, Option<LocalType>) {
        // cite: bionic localtime.c#settzname
        let mut standard = None;
        let mut daylight = None;
        for (_, index) in self.transitions.iter().rev() {
            let kind = &self.types[*index];
            if kind.is_dst && daylight.is_none() {
                daylight = Some(kind.clone());
            } else if !kind.is_dst && standard.is_none() {
                standard = Some(kind.clone());
            }
            if standard.is_some() && daylight.is_some() {
                break;
            }
        }
        if let Some(posix) = &self.footer {
            standard = Some(posix.standard.clone());
            daylight = posix.daylight.as_ref().map(|(kind, _, _)| kind.clone());
        }
        (standard.or_else(|| self.default_type().cloned()).unwrap_or(LocalType { offset: 0, is_dst: false, abbreviation: "GMT".to_string() }), daylight)
    }
}
