use crate::adb::LogcatLine;
use crate::errors::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Verbose => "V",
            LogLevel::Debug => "D",
            LogLevel::Info => "I",
            LogLevel::Warn => "W",
            LogLevel::Error => "E",
            LogLevel::Fatal => "F",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().chars().next() {
            Some('V') => Some(LogLevel::Verbose),
            Some('D') => Some(LogLevel::Debug),
            Some('I') => Some(LogLevel::Info),
            Some('W') => Some(LogLevel::Warn),
            Some('E') => Some(LogLevel::Error),
            Some('F') => Some(LogLevel::Fatal),
            _ => None,
        }
    }

    pub fn level_value(&self) -> u8 {
        match self {
            LogLevel::Verbose => 1,
            LogLevel::Debug => 2,
            LogLevel::Info => 3,
            LogLevel::Warn => 4,
            LogLevel::Error => 5,
            LogLevel::Fatal => 6,
        }
    }

    pub fn ge(&self, other: &LogLevel) -> bool {
        self.level_value() >= other.level_value()
    }
}

pub trait LogcatFilter: Send + Sync {
    fn matches(&self, line: &str) -> bool;
    fn name(&self) -> &'static str;
}

pub struct KeywordFilter {
    keywords: Vec<String>,
    case_sensitive: bool,
}

impl KeywordFilter {
    pub fn new(keywords: Vec<String>, case_sensitive: bool) -> Self {
        Self {
            keywords,
            case_sensitive,
        }
    }

    pub fn from_csv(csv: &str, case_sensitive: bool) -> Self {
        let keywords = csv
            .split(',')
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())
            .collect();
        Self {
            keywords,
            case_sensitive,
        }
    }
}

impl LogcatFilter for KeywordFilter {
    fn matches(&self, line: &str) -> bool {
        if self.keywords.is_empty() {
            return true;
        }

        for keyword in &self.keywords {
            if self.case_sensitive {
                if line.contains(keyword) {
                    return true;
                }
            } else {
                let lower_line = line.to_lowercase();
                let lower_keyword = keyword.to_lowercase();
                if lower_line.contains(&lower_keyword) {
                    return true;
                }
            }
        }
        false
    }

    fn name(&self) -> &'static str {
        "KeywordFilter"
    }
}

pub struct RegexFilter {
    pattern: Regex,
}

impl RegexFilter {
    pub fn new(pattern: &str) -> Result<Self> {
        let regex = Regex::new(pattern)?;
        Ok(Self { pattern: regex })
    }
}

impl LogcatFilter for RegexFilter {
    fn matches(&self, line: &str) -> bool {
        self.pattern.is_match(line)
    }

    fn name(&self) -> &'static str {
        "RegexFilter"
    }
}

pub struct LevelFilter {
    min_level: LogLevel,
}

impl LevelFilter {
    pub fn new(min_level: LogLevel) -> Self {
        Self { min_level }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        let level = LogLevel::from_str(s).ok_or_else(|| {
            crate::errors::AdbError::InvalidArgument(format!(
                "Invalid log level: {}. Must be V, D, I, W, E, or F",
                s
            ))
        })?;
        Ok(Self { min_level: level })
    }
}

impl LogcatFilter for LevelFilter {
    fn matches(&self, line: &str) -> bool {
        // Extract log level from line (typically 3rd space-separated field)
        if let Some(level_str) = line.split_whitespace().nth(4) {
            if let Some(level) = LogLevel::from_str(level_str) {
                return level.ge(&self.min_level);
            }
        }
        true // Include lines we can't parse
    }

    fn name(&self) -> &'static str {
        "LevelFilter"
    }
}

pub struct TagFilter {
    tags: Vec<String>,
}

impl TagFilter {
    pub fn new(tags: Vec<String>) -> Self {
        Self { tags }
    }

    pub fn from_csv(csv: &str) -> Self {
        let tags = csv
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        Self { tags }
    }
}

impl LogcatFilter for TagFilter {
    fn matches(&self, line: &str) -> bool {
        if self.tags.is_empty() {
            return true;
        }

        // Try to parse the logcat line to extract the tag
        if let Some(parsed) = LogcatLine::parse(line) {
            return self.tags.iter().any(|tag| parsed.tag.contains(tag));
        }

        // Fallback: check if tag appears in line as "tag:"
        for tag in &self.tags {
            if line.contains(&format!("{}:", tag)) {
                return true;
            }
        }

        false
    }

    fn name(&self) -> &'static str {
        "TagFilter"
    }
}

pub struct ExcludeFilter {
    patterns: Vec<String>,
}

impl ExcludeFilter {
    pub fn new(patterns: Vec<String>) -> Self {
        Self { patterns }
    }

    pub fn from_csv(csv: &str) -> Self {
        let patterns = csv
            .split(',')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        Self { patterns }
    }
}

impl LogcatFilter for ExcludeFilter {
    fn matches(&self, line: &str) -> bool {
        // Return true if NO patterns match (exclude logic)
        !self.patterns.iter().any(|pattern| line.contains(pattern))
    }

    fn name(&self) -> &'static str {
        "ExcludeFilter"
    }
}

pub struct LogcatFilterChain {
    filters: Vec<Box<dyn LogcatFilter>>,
    applied: Vec<String>,
}

impl LogcatFilterChain {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            applied: Vec::new(),
        }
    }

    pub fn add_filter(mut self, filter: Box<dyn LogcatFilter>) -> Self {
        self.applied.push(filter.name().to_string());
        self.filters.push(filter);
        self
    }

    pub fn add_keyword_filter(self, keywords: Vec<String>, case_sensitive: bool) -> Self {
        self.add_filter(Box::new(KeywordFilter::new(keywords, case_sensitive)))
    }

    pub fn add_regex_filter(self, pattern: &str) -> Result<Self> {
        Ok(self.add_filter(Box::new(RegexFilter::new(pattern)?)))
    }

    pub fn add_level_filter(self, min_level: LogLevel) -> Self {
        self.add_filter(Box::new(LevelFilter::new(min_level)))
    }

    pub fn add_tag_filter(self, tags: Vec<String>) -> Self {
        self.add_filter(Box::new(TagFilter::new(tags)))
    }

    pub fn add_exclude_filter(self, patterns: Vec<String>) -> Self {
        self.add_filter(Box::new(ExcludeFilter::new(patterns)))
    }

    pub fn apply(&self, text: &str) -> String {
        if self.filters.is_empty() {
            return text.to_string();
        }

        text.lines()
            .filter(|line| self.filters.iter().all(|f| f.matches(line)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn applied_filters(&self) -> Vec<String> {
        self.applied.clone()
    }
}

impl Default for LogcatFilterChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_filter() {
        let filter = KeywordFilter::new(vec!["error".to_string()], false);
        assert!(filter.matches("This is an ERROR message"));
        assert!(!filter.matches("This is a warning"));
    }

    #[test]
    fn test_level_filter() {
        let filter = LevelFilter::new(LogLevel::Warn);
        assert!(filter.matches("01-15 10:00:00.123 1234 1234 E tag: error"));
        assert!(filter.matches("01-15 10:00:00.123 1234 1234 W tag: warning"));
        assert!(!filter.matches("01-15 10:00:00.123 1234 1234 I tag: info"));
    }

    #[test]
    fn test_filter_chain() {
        let chain = LogcatFilterChain::new()
            .add_keyword_filter(vec!["error".to_string()], false)
            .add_exclude_filter(vec!["ignored".to_string()]);

        let input = "error: this is an error\nignored error: skip this\nERROR: another error";
        let output = chain.apply(input);

        assert!(output.contains("error: this is an error"));
        assert!(!output.contains("ignored error"));
        assert!(output.contains("ERROR: another error"));
    }
}
