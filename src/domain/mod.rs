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
        let mut parts = vec![format!("摘要：{}", self.summary)];
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
