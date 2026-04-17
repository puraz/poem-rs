use std::collections::HashSet;

use crate::domain::{AiAppreciation, AiRecommendation};

use super::ai::{AiTransportError, RecommendationPayload};

pub const MIN_VALID_RECOMMENDATIONS: usize = 2;
pub const MIN_VALID_RATIO: f32 = 0.5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecommendationSource {
    Ai,
    LocalFallback,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FallbackReason {
    Unconfigured,
    Timeout,
    Transport,
    Parse,
    InsufficientValidIds,
    EmptyResponse,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedRecommendations {
    pub valid: Vec<AiRecommendation>,
    pub discarded_ids: Vec<String>,
    pub valid_ratio: f32,
    pub should_fallback: bool,
    pub fallback_reason: Option<FallbackReason>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecommendationResolution {
    pub source: RecommendationSource,
    pub recommendations: Vec<AiRecommendation>,
    pub discarded_ids: Vec<String>,
    pub valid_ratio: f32,
    pub fallback_reason: Option<FallbackReason>,
}

impl RecommendationResolution {
    pub fn warning_banner(&self) -> Option<&'static str> {
        match self.fallback_reason {
            Some(FallbackReason::Unconfigured) => Some("AI 未配置，已回退到本地推荐。"),
            Some(FallbackReason::Timeout) => Some("AI 请求超时，已回退到本地推荐。"),
            Some(FallbackReason::Transport) => Some("AI 服务暂时不可用，已回退到本地推荐。"),
            Some(FallbackReason::Parse) => Some("AI 返回无法解析，已回退到本地推荐。"),
            Some(FallbackReason::InsufficientValidIds) => {
                Some("AI 返回的诗词匹配不足，已回退到本地推荐。")
            }
            Some(FallbackReason::EmptyResponse) => Some("AI 未返回可用推荐，已回退到本地推荐。"),
            None => None,
        }
    }
}

pub fn normalize_recommendations(
    payload: RecommendationPayload,
    known_poem_ids: &HashSet<String>,
) -> NormalizedRecommendations {
    let total = payload.recommendations.len();
    let mut valid = Vec::new();
    let mut discarded_ids = Vec::new();

    for recommendation in payload.recommendations {
        if known_poem_ids.contains(&recommendation.poem_id) {
            valid.push(recommendation);
        } else {
            discarded_ids.push(recommendation.poem_id);
        }
    }

    let valid_ratio = if total == 0 {
        0.0
    } else {
        valid.len() as f32 / total as f32
    };
    let should_fallback = total == 0
        || valid.is_empty()
        || valid.len() < MIN_VALID_RECOMMENDATIONS
        || valid_ratio < MIN_VALID_RATIO;

    let fallback_reason = if total == 0 {
        Some(FallbackReason::EmptyResponse)
    } else if should_fallback {
        Some(FallbackReason::InsufficientValidIds)
    } else {
        None
    };

    NormalizedRecommendations {
        valid,
        discarded_ids,
        valid_ratio,
        should_fallback,
        fallback_reason,
    }
}

pub fn resolve_recommendations(
    result: Result<RecommendationPayload, AiTransportError>,
    known_poem_ids: &HashSet<String>,
    fallback_recommendations: Vec<AiRecommendation>,
) -> RecommendationResolution {
    match result {
        Ok(payload) => {
            let normalized = normalize_recommendations(payload, known_poem_ids);
            if normalized.should_fallback {
                RecommendationResolution {
                    source: RecommendationSource::LocalFallback,
                    recommendations: fallback_recommendations,
                    discarded_ids: normalized.discarded_ids,
                    valid_ratio: normalized.valid_ratio,
                    fallback_reason: normalized.fallback_reason,
                }
            } else {
                RecommendationResolution {
                    source: RecommendationSource::Ai,
                    recommendations: normalized.valid,
                    discarded_ids: normalized.discarded_ids,
                    valid_ratio: normalized.valid_ratio,
                    fallback_reason: None,
                }
            }
        }
        Err(error) => RecommendationResolution {
            source: RecommendationSource::LocalFallback,
            recommendations: fallback_recommendations,
            discarded_ids: Vec::new(),
            valid_ratio: 0.0,
            fallback_reason: Some(map_error_to_reason(error)),
        },
    }
}

pub fn validate_appreciation(
    appreciation: AiAppreciation,
    known_poem_ids: &HashSet<String>,
) -> Option<AiAppreciation> {
    known_poem_ids
        .contains(&appreciation.poem_id)
        .then_some(appreciation)
}

fn map_error_to_reason(error: AiTransportError) -> FallbackReason {
    match error {
        AiTransportError::Unconfigured => FallbackReason::Unconfigured,
        AiTransportError::Timeout => FallbackReason::Timeout,
        AiTransportError::Transport(_) => FallbackReason::Transport,
        AiTransportError::Parse(_) => FallbackReason::Parse,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known_ids() -> HashSet<String> {
        ["poem-1", "poem-2", "poem-3"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn fallback_items() -> Vec<AiRecommendation> {
        vec![
            AiRecommendation::new("poem-1", "本地精选：春景意象明确。", None),
            AiRecommendation::new("poem-2", "本地精选：与输入情绪最接近。", None),
        ]
    }

    #[test]
    fn keeps_ai_recommendations_when_threshold_is_met() {
        let normalized = normalize_recommendations(
            RecommendationPayload {
                recommendations: vec![
                    AiRecommendation::new("poem-1", "spring dawn", Some(0.93)),
                    AiRecommendation::new("poem-2", "birds at daybreak", Some(0.88)),
                    AiRecommendation::new("missing", "unknown id", Some(0.12)),
                    AiRecommendation::new("poem-3", "quiet mood", Some(0.70)),
                ],
            },
            &known_ids(),
        );

        assert_eq!(normalized.valid.len(), 3);
        assert_eq!(normalized.discarded_ids, vec!["missing".to_string()]);
        assert!(!normalized.should_fallback);
        assert_eq!(normalized.valid_ratio, 0.75);
    }

    #[test]
    fn falls_back_when_valid_count_drops_below_contract() {
        let resolution = resolve_recommendations(
            Ok(RecommendationPayload {
                recommendations: vec![
                    AiRecommendation::new("poem-1", "valid", Some(0.91)),
                    AiRecommendation::new("missing-a", "invalid", Some(0.21)),
                    AiRecommendation::new("missing-b", "invalid", Some(0.11)),
                ],
            }),
            &known_ids(),
            fallback_items(),
        );

        assert_eq!(resolution.source, RecommendationSource::LocalFallback);
        assert_eq!(
            resolution.fallback_reason,
            Some(FallbackReason::InsufficientValidIds)
        );
        assert_eq!(
            resolution.warning_banner(),
            Some("AI 返回的诗词匹配不足，已回退到本地推荐。")
        );
    }

    #[test]
    fn falls_back_on_parse_failure() {
        let resolution = resolve_recommendations(
            Err(AiTransportError::Parse("bad json".into())),
            &known_ids(),
            fallback_items(),
        );

        assert_eq!(resolution.source, RecommendationSource::LocalFallback);
        assert_eq!(resolution.fallback_reason, Some(FallbackReason::Parse));
        assert_eq!(resolution.recommendations.len(), 2);
    }

    #[test]
    fn falls_back_on_timeout() {
        let resolution = resolve_recommendations(
            Err(AiTransportError::Timeout),
            &known_ids(),
            fallback_items(),
        );

        assert_eq!(resolution.source, RecommendationSource::LocalFallback);
        assert_eq!(resolution.fallback_reason, Some(FallbackReason::Timeout));
    }

    #[test]
    fn validates_appreciation_against_local_ids() {
        let appreciation = AiAppreciation::new(
            "poem-1",
            "summary",
            vec!["月".into()],
            vec!["光".into()],
            "notes",
        );
        assert!(validate_appreciation(appreciation, &known_ids()).is_some());

        let unknown = AiAppreciation::new("missing", "summary", vec![], vec![], "notes");
        assert!(validate_appreciation(unknown, &known_ids()).is_none());
    }
}
