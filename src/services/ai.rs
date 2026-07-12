use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

use crate::config::ai::AiSettings;
use crate::domain::{AiAppreciation, AiRecommendation, DiscoveredPoem, PoemCandidate};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoetProfilePrompt {
    pub poet_name: String,
    pub dynasty: String,
    pub poem_titles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoetProfilePayload {
    pub poet_name: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecommendationPayload {
    pub recommendations: Vec<AiRecommendation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveryPrompt {
    pub query: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DiscoveryPayload {
    pub poems: Vec<DiscoveredPoem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiTransportError {
    Unconfigured,
    Timeout,
    Transport(String),
    Parse(String),
}

#[allow(async_fn_in_trait)]
pub trait AiTransport {
    async fn discover(
        &self,
        request: &DiscoveryPrompt,
    ) -> Result<DiscoveryPayload, AiTransportError>;
    async fn recommend(
        &self,
        request: &RecommendationPrompt,
    ) -> Result<RecommendationPayload, AiTransportError>;
    async fn appreciate(
        &self,
        request: &AppreciationPrompt,
    ) -> Result<AiAppreciation, AiTransportError>;
    async fn fetch_poet_profile(
        &self,
        request: &PoetProfilePrompt,
    ) -> Result<PoetProfilePayload, AiTransportError>;
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
    pub async fn discover(
        &self,
        request: &DiscoveryPrompt,
    ) -> Result<DiscoveryPayload, AiTransportError> {
        self.transport.discover(request).await
    }

    pub async fn recommend(
        &self,
        request: &RecommendationPrompt,
    ) -> Result<RecommendationPayload, AiTransportError> {
        self.transport.recommend(request).await
    }

    pub async fn appreciate(
        &self,
        request: &AppreciationPrompt,
    ) -> Result<AiAppreciation, AiTransportError> {
        self.transport.appreciate(request).await
    }

    pub async fn fetch_poet_profile(
        &self,
        request: &PoetProfilePrompt,
    ) -> Result<PoetProfilePayload, AiTransportError> {
        self.transport.fetch_poet_profile(request).await
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
            .timeout(Duration::from_secs(self.settings.effective_timeout_secs()))
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

    async fn post_chat(
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
            .await
            .map_err(map_reqwest_error)?;

        let response = response.error_for_status().map_err(map_reqwest_error)?;
        let payload: ChatResponse = response
            .json()
            .await
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
    async fn discover(
        &self,
        request: &DiscoveryPrompt,
    ) -> Result<DiscoveryPayload, AiTransportError> {
        let system_prompt = r#"你是一个专业的古诗词检索助手。用户可能会提供诗词片段、关键词、主题意境描述或模糊记忆。你的任务是理解用户搜索意图，找到最匹配的完整古诗词，并只返回 JSON 数组。

返回要求：
1. 只返回最外层 JSON 数组，不要 markdown 代码块，不要额外解释。
2. 默认返回最推荐的 3 首古诗词。
3. 每个对象必须包含字段：title, content, author, dynasty, category, notes, relevanceScore, matchReason, isRecommendation。
4. title 不要带《》；dynasty 允许为空字符串；content 必须是完整诗词，并保留标点、换行和必要分段。
5. 每句诗词单独成行；确实需要分段的长诗/乐府可使用空行分段。
6. relevanceScore 取值范围 0.0 到 1.0；matchReason 需要具体解释与查询的关联。
7. 优先返回与查询最相关、最完整、最可信的结果。"#;
        let user_prompt = format!(
            "基于我的搜索意图“{}”，请返回最匹配的 3 首古诗词。\n要求：\n- 返回完整准确的标题、作者、朝代、全文\n- content 中的换行使用 \\n 表示，段落之间使用 \\n\\n 表示\n- 结果必须可以被程序直接解析为 JSON 数组\n- isRecommendation 统一返回 true",
            request.query
        );
        let content = self.post_chat(system_prompt, &user_prompt).await?;
        parse_discovery_content(&content)
    }

    async fn recommend(
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
        let content = self.post_chat(system_prompt, &user_prompt).await?;
        parse_recommendation_content(&content)
    }

    async fn appreciate(
        &self,
        request: &AppreciationPrompt,
    ) -> Result<AiAppreciation, AiTransportError> {
        let system_prompt = "你是一个古典诗词赏析助手。只返回 JSON，不要额外解释。JSON 结构必须为 {\"poem_id\":string,\"summary\":string,\"themes\":[string],\"imagery\":[string],\"notes_markdown\":string}。";
        let user_prompt = format!(
            "请为下面这首诗生成简洁中文赏析，并保持 poem_id 原样返回。\npoem_id: {}\n标题: {}\n作者: {}\n朝代: {}\n诗句:\n{}",
            request.poem_id, request.title, request.author, request.dynasty, request.excerpt
        );
        let content = self.post_chat(system_prompt, &user_prompt).await?;
        parse_appreciation_content(&content)
    }

    async fn fetch_poet_profile(
        &self,
        request: &PoetProfilePrompt,
    ) -> Result<PoetProfilePayload, AiTransportError> {
        let system_prompt = r#"你是一个中国古典文学专家。用户会提供一位诗人的名字以及该诗人的部分作品列表。
请为这位诗人生成一份详细的文学传记档案。

要求：
1. 内容应包括：生平简介、字号/别称、生卒年份、籍贯、文学风格、代表作品点评、历史地位与影响
2. 返回纯文本，不要使用 Markdown 格式（不要用 # * - ` > 等标记符号）
3. 内容应当详尽丰富，有实质信息而非泛泛而谈
4. 不要使用 "描述"、"以下是"、"关于" 等引导性语言开头，直接开始正文
5. 只返回文本内容，不要任何 JSON 包裹、不要代码块包裹"#;

        let poem_titles = request.poem_titles.join("、");
        let user_prompt = format!(
            "诗人：{}\n朝代：{}\n代表作品：{}\n\n请为这位诗人生成详细档案。",
            request.poet_name, request.dynasty, poem_titles
        );

        let content = self.post_chat(system_prompt, &user_prompt).await?;
        Ok(PoetProfilePayload {
            poet_name: request.poet_name.clone(),
            content,
        })
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

fn parse_discovery_content(content: &str) -> Result<DiscoveryPayload, AiTransportError> {
    let value = parse_json_array(content)?;
    let poems = serde_json::from_value::<Vec<DiscoveredPoem>>(value)
        .map_err(|err| AiTransportError::Parse(err.to_string()))?;
    let poems = poems.into_iter().take(3).collect::<Vec<_>>();
    if poems.is_empty() {
        return Err(AiTransportError::Parse(
            "empty discovery result array".to_string(),
        ));
    }
    Ok(DiscoveryPayload { poems })
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
    let value = parse_json_value(content)?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(AiTransportError::Parse("expected json object".into()))
    }
}

fn parse_json_array(content: &str) -> Result<Value, AiTransportError> {
    let value = parse_json_value(content)?;
    if value.is_array() {
        Ok(value)
    } else {
        Err(AiTransportError::Parse("expected json array".into()))
    }
}

fn parse_json_value(content: &str) -> Result<Value, AiTransportError> {
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
    extract_json_slice_by_delimiters(content, '[', ']')
        .or_else(|| extract_json_slice_by_delimiters(content, '{', '}'))
}

fn extract_json_slice_by_delimiters(content: &str, open: char, close: char) -> Option<&str> {
    let start = content.find(open)?;
    let end = content.rfind(close)?;
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

pub fn build_discovery_prompt(query: &str) -> DiscoveryPrompt {
    DiscoveryPrompt {
        query: query.trim().to_string(),
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

pub fn build_poet_profile_prompt(
    poet_name: &str,
    dynasty: &str,
    poem_titles: Vec<String>,
) -> PoetProfilePrompt {
    PoetProfilePrompt {
        poet_name: poet_name.to_string(),
        dynasty: dynasty.to_string(),
        poem_titles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;

    #[derive(Clone)]
    struct StubTransport {
        discovery: Result<DiscoveryPayload, AiTransportError>,
        recommendation: Result<RecommendationPayload, AiTransportError>,
        appreciation: Result<AiAppreciation, AiTransportError>,
        poet_profile: Result<PoetProfilePayload, AiTransportError>,
    }

    impl AiTransport for StubTransport {
        async fn discover(
            &self,
            _request: &DiscoveryPrompt,
        ) -> Result<DiscoveryPayload, AiTransportError> {
            self.discovery.clone()
        }

        async fn recommend(
            &self,
            _request: &RecommendationPrompt,
        ) -> Result<RecommendationPayload, AiTransportError> {
            self.recommendation.clone()
        }

        async fn appreciate(
            &self,
            _request: &AppreciationPrompt,
        ) -> Result<AiAppreciation, AiTransportError> {
            self.appreciation.clone()
        }

        async fn fetch_poet_profile(
            &self,
            _request: &PoetProfilePrompt,
        ) -> Result<PoetProfilePayload, AiTransportError> {
            self.poet_profile.clone()
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        iced::futures::executor::block_on(future)
    }

    #[test]
    fn client_returns_transport_payloads() {
        let client = OpenAiCompatibleClient::new(StubTransport {
            discovery: Ok(DiscoveryPayload {
                poems: vec![DiscoveredPoem {
                    title: "登高".into(),
                    content: "风急天高猿啸哀，\n渚清沙白鸟飞回。".into(),
                    author: "杜甫".into(),
                    dynasty: "唐".into(),
                    category: String::new(),
                    notes: String::new(),
                    relevance_score: 0.98,
                    match_reason: "高度匹配".into(),
                    is_recommendation: true,
                }],
            }),
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
            poet_profile: Err(AiTransportError::Unconfigured),
        });

        let discovered = block_on(client.discover(&DiscoveryPrompt {
            query: "登高".into(),
        }))
        .expect("discover");
        assert_eq!(discovered.poems[0].title, "登高");

        let recommendations = block_on(client.recommend(&RecommendationPrompt {
            user_prompt: "find spring poems".into(),
            candidates: vec![PoemCandidate::new(
                "poem-1",
                "春晓",
                "孟浩然",
                "唐",
                "春眠不觉晓",
            )],
        }))
        .expect("recommendations");
        assert_eq!(recommendations.recommendations.len(), 1);

        let appreciation = block_on(client.appreciate(&AppreciationPrompt {
            poem_id: "poem-1".into(),
            title: "春晓".into(),
            author: "孟浩然".into(),
            dynasty: "唐".into(),
            excerpt: "春眠不觉晓".into(),
        }))
        .expect("appreciation");
        assert_eq!(appreciation.poem_id, "poem-1");
    }

    #[test]
    fn client_preserves_timeout_failures() {
        let client = OpenAiCompatibleClient::new(StubTransport {
            discovery: Err(AiTransportError::Timeout),
            recommendation: Err(AiTransportError::Timeout),
            appreciation: Err(AiTransportError::Unconfigured),
            poet_profile: Err(AiTransportError::Unconfigured),
        });

        assert_eq!(
            block_on(client.discover(&DiscoveryPrompt {
                query: "request".into(),
            }))
            .expect_err("timeout expected"),
            AiTransportError::Timeout
        );

        assert_eq!(
            block_on(client.recommend(&RecommendationPrompt {
                user_prompt: "request".into(),
                candidates: vec![],
            }))
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
    fn parses_discovery_array_json() {
        let payload = parse_discovery_content(
            r#"[{"title":"登高","content":"风急天高猿啸哀，\n渚清沙白鸟飞回。","author":"杜甫","dynasty":"唐","category":"","notes":"","relevanceScore":0.99,"matchReason":"与登高主题完全匹配","isRecommendation":true}]"#,
        )
        .expect("parse discovery");
        assert_eq!(payload.poems[0].title, "登高");
        assert_eq!(payload.poems[0].relevance_percent(), "99%");
    }

    #[test]
    fn extracts_discovery_array_from_fenced_output() {
        let payload = parse_discovery_content(
            "```json\n[{\"title\":\"春晓\",\"content\":\"春眠不觉晓，\\n处处闻啼鸟。\",\"author\":\"孟浩然\",\"dynasty\":\"唐\",\"category\":\"\",\"notes\":\"\",\"relevanceScore\":0.91,\"matchReason\":\"春景匹配\",\"isRecommendation\":true}]\n```",
        )
        .expect("parse fenced discovery");
        assert_eq!(payload.poems[0].author, "孟浩然");
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
