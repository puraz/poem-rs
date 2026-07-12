use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Poem {
    pub id: String,
    pub title: String,
    pub author: String,
    pub dynasty: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source: String,
    pub license: String,
    pub is_favorite: bool,
}

impl Poem {
    pub fn snippet(&self) -> String {
        self.content.lines().take(2).collect::<Vec<_>>().join(" · ")
    }

    pub fn metadata(&self) -> String {
        format!("{} · {}", self.author, self.dynasty)
    }

    pub fn tags_summary(&self) -> String {
        self.tags.join(" / ")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoemCandidate {
    pub poem_id: String,
    pub title: String,
    pub author: String,
    pub dynasty: String,
    pub snippet: String,
}

impl PoemCandidate {
    pub fn new(
        poem_id: impl Into<String>,
        title: impl Into<String>,
        author: impl Into<String>,
        dynasty: impl Into<String>,
        snippet: impl Into<String>,
    ) -> Self {
        Self {
            poem_id: poem_id.into(),
            title: title.into(),
            author: author.into(),
            dynasty: dynasty.into(),
            snippet: snippet.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportPoem {
    pub title: String,
    pub author: String,
    pub dynasty: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub is_favorite: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoetryExport {
    pub version: u32,
    pub app: String,
    pub exported_at: String,
    pub total: usize,
    pub poems: Vec<ExportPoem>,
}

impl From<&Poem> for ExportPoem {
    fn from(poem: &Poem) -> Self {
        Self {
            title: poem.title.clone(),
            author: poem.author.clone(),
            dynasty: poem.dynasty.clone(),
            content: poem.content.clone(),
            tags: poem.tags.clone(),
            source: poem.source.clone(),
            license: poem.license.clone(),
            is_favorite: poem.is_favorite,
        }
    }
}

impl From<&Poem> for PoemCandidate {
    fn from(value: &Poem) -> Self {
        Self::new(
            value.id.clone(),
            value.title.clone(),
            value.author.clone(),
            value.dynasty.clone(),
            value.snippet(),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AiRecommendation {
    pub poem_id: String,
    pub reason: String,
    pub confidence: Option<f32>,
}

impl AiRecommendation {
    pub fn new(
        poem_id: impl Into<String>,
        reason: impl Into<String>,
        confidence: Option<f32>,
    ) -> Self {
        Self {
            poem_id: poem_id.into(),
            reason: reason.into(),
            confidence,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredPoem {
    pub title: String,
    pub content: String,
    pub author: String,
    pub dynasty: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub notes: String,
    #[serde(rename = "relevanceScore")]
    pub relevance_score: f32,
    #[serde(rename = "matchReason")]
    pub match_reason: String,
    #[serde(rename = "isRecommendation", default)]
    pub is_recommendation: bool,
}

impl DiscoveredPoem {
    pub fn snippet(&self) -> String {
        self.content.lines().take(3).collect::<Vec<_>>().join(" · ")
    }

    pub fn relevance_percent(&self) -> String {
        let score = self.relevance_score.clamp(0.0, 1.0) * 100.0;
        format!("{score:.0}%")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiAppreciation {
    pub poem_id: String,
    pub summary: String,
    pub themes: Vec<String>,
    pub imagery: Vec<String>,
    pub notes_markdown: String,
}

impl AiAppreciation {
    pub fn new(
        poem_id: impl Into<String>,
        summary: impl Into<String>,
        themes: Vec<String>,
        imagery: Vec<String>,
        notes_markdown: impl Into<String>,
    ) -> Self {
        Self {
            poem_id: poem_id.into(),
            summary: summary.into(),
            themes,
            imagery,
            notes_markdown: notes_markdown.into(),
        }
    }

    pub fn display_text(&self) -> String {
        let mut parts = Vec::new();
        if !self.summary.is_empty() {
            parts.push(self.summary.clone());
        }
        if !self.themes.is_empty() {
            parts.push(format!("主题：{}", self.themes.join("、")));
        }
        if !self.imagery.is_empty() {
            parts.push(format!("意象：{}", self.imagery.join("、")));
        }
        if !self.notes_markdown.is_empty() {
            parts.push(self.notes_markdown.clone());
        }
        parts.join("\n\n")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoetProfile {
    pub poet_name: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Strip common Markdown markers from text for plain-text display.
pub fn strip_markdown(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for line in text.lines() {
        let trimmed = line.trim();

        // Skip horizontal rules
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            continue;
        }

        let mut cleaned = trimmed;

        // Remove heading markers
        if cleaned.starts_with('#') {
            cleaned = cleaned.trim_start_matches(['#', ' ']);
        }

        // Remove blockquote prefix
        if cleaned.starts_with('>') {
            cleaned = cleaned.trim_start_matches("> ").trim_start_matches('>');
        }

        // Remove unordered list markers
        if cleaned.starts_with("- ") || cleaned.starts_with("* ") || cleaned.starts_with("+ ") {
            cleaned = &cleaned[2..];
        }

        // Remove bold/italic markers and inline code backticks
        let cleaned = cleaned
            .replace("**", "")
            .replace("__", "")
            .replace(['*', '`'], "")
            .replace(['[', ']'], "");

        // Handle markdown links: [text](url) → remove (url) part
        let cleaned = if let Some(open) = cleaned.find('(') {
            if cleaned.contains(')') {
                let before = &cleaned[..open];
                let after = &cleaned[cleaned.rfind(')').unwrap() + 1..];
                format!("{}{}", before, after)
            } else {
                cleaned
            }
        } else {
            cleaned
        };

        // Remove image markers
        let cleaned = cleaned.replace("![", "[");

        if !cleaned.trim().is_empty() {
            result.push_str(cleaned.trim());
            result.push('\n');
        }
    }

    result.trim().to_string()
}
