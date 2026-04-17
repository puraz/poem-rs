use std::cmp::Reverse;

use crate::domain::{AiRecommendation, Poem};

pub fn curated_recommendations(poems: &[Poem], limit: usize) -> Vec<AiRecommendation> {
    poems
        .iter()
        .take(limit)
        .map(|poem| {
            AiRecommendation::new(
                poem.id.clone(),
                format!(
                    "本地精选：{}的{}意象清晰。",
                    poem.author,
                    poem.tags_summary()
                ),
                None,
            )
        })
        .collect()
}

pub fn discover_locally(query: &str, poems: &[Poem], limit: usize) -> Vec<AiRecommendation> {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return curated_recommendations(poems, limit);
    }

    let mut scored = poems
        .iter()
        .map(|poem| {
            let haystack = format!(
                "{} {} {} {} {}",
                poem.title,
                poem.author,
                poem.dynasty,
                poem.content,
                poem.tags.join(" ")
            )
            .to_lowercase();
            let score = score_query(&normalized, &haystack);
            (score, poem)
        })
        .filter(|(score, _)| *score > 0)
        .collect::<Vec<_>>();

    scored.sort_by_key(|(score, poem)| (Reverse(*score), poem.title.clone()));

    if scored.is_empty() {
        return curated_recommendations(poems, limit);
    }

    scored
        .into_iter()
        .take(limit)
        .map(|(score, poem)| {
            AiRecommendation::new(
                poem.id.clone(),
                format!("本地匹配分数 {score}：命中了“{}”相关主题。", query.trim()),
                None,
            )
        })
        .collect()
}

fn score_query(query: &str, haystack: &str) -> usize {
    let mut score = 0;
    for token in query
        .split(|ch: char| ch.is_whitespace() || ch == '，' || ch == ',' || ch == '。')
        .filter(|token| !token.is_empty())
    {
        if haystack.contains(token) {
            score += token.chars().count().max(1);
        }
    }
    if haystack.contains(query) {
        score += query.chars().count() * 2;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_poems() -> Vec<Poem> {
        vec![
            Poem {
                id: "1".into(),
                title: "春晓".into(),
                author: "孟浩然".into(),
                dynasty: "唐".into(),
                content: "春眠不觉晓\n处处闻啼鸟".into(),
                tags: vec!["春景".into()],
                source: "x".into(),
                license: "Public Domain".into(),
                is_favorite: false,
            },
            Poem {
                id: "2".into(),
                title: "静夜思".into(),
                author: "李白".into(),
                dynasty: "唐".into(),
                content: "床前明月光\n低头思故乡".into(),
                tags: vec!["思乡".into(), "明月".into()],
                source: "x".into(),
                license: "Public Domain".into(),
                is_favorite: false,
            },
        ]
    }

    #[test]
    fn local_discovery_prefers_matching_poem() {
        let recommendations = discover_locally("思乡 月夜", &sample_poems(), 3);
        assert_eq!(
            recommendations.first().map(|item| item.poem_id.as_str()),
            Some("2")
        );
    }
}
