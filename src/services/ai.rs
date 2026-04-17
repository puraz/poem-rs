use anyhow::Result;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

use crate::config::ai::AiSettings;
use crate::domain::{AiAppreciation, AiRecommendation, PoemCandidate};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecommendationPrompt {
    pub user_prompt: String,
    pub candidates: Vec<PoemCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppreciationPrompt {
    pub poem_id: String,
    pub title: String,
    pub author: String,
    pub dynasty: String,
    pub excerpt: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecommendationPayload {
    pub recommendations: Vec<AiRecommendation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiTransportError {
    Unconfigured,
    Timeout,
    Transport(String),
    Parse(String),
}

pub trait AiTransport {
    fn recommend(
        &self,
        request: &RecommendationPrompt,
    ) -> Result<RecommendationPayload, AiTransportError>;
    fn appreciate(&self, request: &AppreciationPrompt) -> Result<AiAppreciation, AiTransportError>;
}

#[derive(Clone, Debug)]
pub struct OpenAiCompatibleClient<T> {
    transport: T,
}

impl<T> OpenAiCompatibleClient<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }
}

impl<T> OpenAiCompatibleClient<T>
where
    T: AiTransport,
{
    pub fn recommend(
        &self,
        request: &RecommendationPrompt,
    ) -> Result<RecommendationPayload, AiTransportError> {
        self.transport.recommend(request)
    }

    pub fn appreciate(
        &self,
        request: &AppreciationPrompt,
    ) -> Result<AiAppreciation, AiTransportError> {
        self.transport.appreciate(request)
    }
}

#[derive(Clone, Debug)]
pub struct HttpAiTransport {
    settings: AiSettings,
    api_key: Option<String>,
}

impl HttpAiTransport {
    pub fn new(settings: AiSettings, api_key: Option<String>) -> Self {
        Self { settings, api_key }
    }

    fn client(&self) -> Result<Client, AiTransportError> {
        Client::builder()
            .timeout(Duration::from_secs(self.settings.timeout_secs))
            .build()
            .map_err(|err| AiTransportError::Transport(err.to_string()))
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/chat/completions",
            self.settings.base_url.trim_end_matches('/')
        )
    }

    fn require_api_key(&self) -> Result<&str, AiTransportError> {
        self.api_key
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or(AiTransportError::Unconfigured)
    }

    fn post_chat(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, AiTransportError> {
        let api_key = self.require_api_key()?;
        let client = self.client()?;
        let body = ChatRequest {
            model: self.settings.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system_prompt.into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: user_prompt.into(),
                },
            ],
            temperature: 0.2,
        };

        let response = client
            .post(self.endpoint())
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .map_err(map_reqwest_error)?;

        let response = response.error_for_status().map_err(map_reqwest_error)?;
        let payload: ChatResponse = response
            .json()
            .map_err(|err| AiTransportError::Parse(err.to_string()))?;

        payload
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| AiTransportError::Parse("missing chat completion content".into()))
    }
}

impl AiTransport for HttpAiTransport {
    fn recommend(
        &self,
        request: &RecommendationPrompt,
    ) -> Result<RecommendationPayload, AiTransportError> {
        let system_prompt = "你是一个古典诗词推荐助手。只返回 JSON，不要额外解释。JSON 结构必须为 {\"recommendations\":[{\"poem_id\":string,\"reason\":string,\"confidence\":number|null}]}。poem_id 必须从候选列表中选择。";
        let candidates_json = serde_json::to_string(&request.candidates)
            .map_err(|err| AiTransportError::Parse(err.to_string()))?;
        let user_prompt = format!(
            "用户需求：{}\n候选诗词：{}\n请从候选诗词中挑选最合适的 3 首。",
            request.user_prompt, candidates_json
        );
        let content = self.post_chat(system_prompt, &user_prompt)?;
        parse_recommendation_content(&content)
    }

    fn appreciate(&self, request: &AppreciationPrompt) -> Result<AiAppreciation, AiTransportError> {
        let system_prompt = "你是一个古典诗词赏析助手。只返回 JSON，不要额外解释。JSON 结构必须为 {\"poem_id\":string,\"summary\":string,\"themes\":[string],\"imagery\":[string],\"notes_markdown\":string}。";
        let user_prompt = format!(
            "请为下面这首诗生成简洁中文赏析，并保持 poem_id 原样返回。\npoem_id: {}\n标题: {}\n作者: {}\n朝代: {}\n诗句:\n{}",
            request.poem_id, request.title, request.author, request.dynasty, request.excerpt
        );
        let content = self.post_chat(system_prompt, &user_prompt)?;
        parse_appreciation_content(&content)
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

fn parse_recommendation_content(content: &str) -> Result<RecommendationPayload, AiTransportError> {
    let value = parse_json_object(content)?;
    let recommendations = value
        .get("recommendations")
        .and_then(Value::as_array)
        .ok_or_else(|| AiTransportError::Parse("missing recommendations array".into()))?
        .iter()
        .map(|item| {
            let poem_id = item
                .get("poem_id")
                .and_then(Value::as_str)
                .ok_or_else(|| AiTransportError::Parse("missing poem_id".into()))?;
            let reason = item
                .get("reason")
                .and_then(Value::as_str)
                .ok_or_else(|| AiTransportError::Parse("missing reason".into()))?;
            let confidence = item
                .get("confidence")
                .and_then(Value::as_f64)
                .map(|v| v as f32);
            Ok(AiRecommendation::new(poem_id, reason, confidence))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(RecommendationPayload { recommendations })
}

fn parse_appreciation_content(content: &str) -> Result<AiAppreciation, AiTransportError> {
    let value = parse_json_object(content)?;
    let poem_id = value
        .get("poem_id")
        .and_then(Value::as_str)
        .ok_or_else(|| AiTransportError::Parse("missing poem_id".into()))?;
    let summary = value
        .get("summary")
        .and_then(Value::as_str)
        .ok_or_else(|| AiTransportError::Parse("missing summary".into()))?;
    let themes = collect_string_array(&value, "themes")?;
    let imagery = collect_string_array(&value, "imagery")?;
    let notes_markdown = value
        .get("notes_markdown")
        .and_then(Value::as_str)
        .ok_or_else(|| AiTransportError::Parse("missing notes_markdown".into()))?;

    Ok(AiAppreciation::new(
        poem_id,
        summary,
        themes,
        imagery,
        notes_markdown,
    ))
}

fn collect_string_array(value: &Value, key: &str) -> Result<Vec<String>, AiTransportError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| AiTransportError::Parse(format!("missing {key} array")))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| AiTransportError::Parse(format!("{key} must contain strings")))
        })
        .collect()
}

fn parse_json_object(content: &str) -> Result<Value, AiTransportError> {
    serde_json::from_str::<Value>(content)
        .or_else(|_| {
            extract_json_slice(content)
                .and_then(|slice| serde_json::from_str::<Value>(slice).ok())
                .ok_or_else(|| {
                    serde_json::Error::io(std::io::Error::other("json extraction failed"))
                })
        })
        .map_err(|err| AiTransportError::Parse(err.to_string()))
}

fn extract_json_slice(content: &str) -> Option<&str> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    (end > start).then_some(&content[start..=end])
}

fn map_reqwest_error(err: reqwest::Error) -> AiTransportError {
    if err.is_timeout() {
        AiTransportError::Timeout
    } else {
        AiTransportError::Transport(err.to_string())
    }
}

pub fn build_recommendation_prompt(
    query: &str,
    candidates: &[PoemCandidate],
) -> RecommendationPrompt {
    RecommendationPrompt {
        user_prompt: query.trim().to_string(),
        candidates: candidates.to_vec(),
    }
}

pub fn build_appreciation_prompt(
    poem_id: &str,
    title: &str,
    author: &str,
    dynasty: &str,
    excerpt: &str,
) -> AppreciationPrompt {
    AppreciationPrompt {
        poem_id: poem_id.to_string(),
        title: title.to_string(),
        author: author.to_string(),
        dynasty: dynasty.to_string(),
        excerpt: excerpt.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct StubTransport {
        recommendation: Result<RecommendationPayload, AiTransportError>,
        appreciation: Result<AiAppreciation, AiTransportError>,
    }

    impl AiTransport for StubTransport {
        fn recommend(
            &self,
            _request: &RecommendationPrompt,
        ) -> Result<RecommendationPayload, AiTransportError> {
            self.recommendation.clone()
        }

        fn appreciate(
            &self,
            _request: &AppreciationPrompt,
        ) -> Result<AiAppreciation, AiTransportError> {
            self.appreciation.clone()
        }
    }

    #[test]
    fn client_returns_transport_payloads() {
        let client = OpenAiCompatibleClient::new(StubTransport {
            recommendation: Ok(RecommendationPayload {
                recommendations: vec![AiRecommendation::new(
                    "poem-1",
                    "Fits the requested spring imagery.",
                    Some(0.91),
                )],
            }),
            appreciation: Ok(AiAppreciation::new(
                "poem-1",
                "Bright, concise spring framing.",
                vec!["春景".into()],
                vec!["花".into()],
                "- imagery",
            )),
        });

        let recommendations = client
            .recommend(&RecommendationPrompt {
                user_prompt: "find spring poems".into(),
                candidates: vec![PoemCandidate::new(
                    "poem-1",
                    "春晓",
                    "孟浩然",
                    "唐",
                    "春眠不觉晓",
                )],
            })
            .expect("recommendations");
        assert_eq!(recommendations.recommendations.len(), 1);

        let appreciation = client
            .appreciate(&AppreciationPrompt {
                poem_id: "poem-1".into(),
                title: "春晓".into(),
                author: "孟浩然".into(),
                dynasty: "唐".into(),
                excerpt: "春眠不觉晓".into(),
            })
            .expect("appreciation");
        assert_eq!(appreciation.poem_id, "poem-1");
    }

    #[test]
    fn client_preserves_timeout_failures() {
        let client = OpenAiCompatibleClient::new(StubTransport {
            recommendation: Err(AiTransportError::Timeout),
            appreciation: Err(AiTransportError::Unconfigured),
        });

        assert_eq!(
            client
                .recommend(&RecommendationPrompt {
                    user_prompt: "request".into(),
                    candidates: vec![],
                })
                .expect_err("timeout expected"),
            AiTransportError::Timeout
        );
    }

    #[test]
    fn parses_recommendation_json() {
        let payload = parse_recommendation_content(
            r#"{"recommendations":[{"poem_id":"poem-1","reason":"good","confidence":0.88}]}"#,
        )
        .expect("parse recommendations");
        assert_eq!(payload.recommendations[0].poem_id, "poem-1");
    }

    #[test]
    fn parses_appreciation_json() {
        let appreciation = parse_appreciation_content(
            r#"{"poem_id":"poem-1","summary":"s","themes":["月"],"imagery":["光"],"notes_markdown":"x"}"#,
        )
        .expect("parse appreciation");
        assert_eq!(appreciation.summary, "s");
    }
}
