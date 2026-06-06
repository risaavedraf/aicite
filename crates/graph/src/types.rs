use chrono::{DateTime, NaiveDateTime, Utc};
use common::{ConceptId, DocumentId, TopicId};
use serde::{Deserialize, Serialize};

const SQLITE_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

mod sqlite_datetime_serde {
    use super::{DateTime, NaiveDateTime, Utc, SQLITE_DATETIME_FORMAT};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&dt.format(SQLITE_DATETIME_FORMAT).to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&value, SQLITE_DATETIME_FORMAT)
            .map(|dt| dt.and_utc())
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    pub topic_id: TopicId,
    pub document_id: DocumentId,
    pub name: String,
    pub summary: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub chunk_count: i64,
    #[serde(with = "sqlite_datetime_serde")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    pub concept_id: ConceptId,
    pub topic_id: TopicId,
    pub name: String,
    pub summary: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub chunk_count: i64,
    #[serde(with = "sqlite_datetime_serde")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct HeadingSpan {
    pub level: usize,
    pub title: String,
    pub char_offset: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{ConceptId, DocumentId, TopicId};

    fn timestamp() -> DateTime<Utc> {
        NaiveDateTime::parse_from_str("2026-06-05 12:34:56", SQLITE_DATETIME_FORMAT)
            .unwrap()
            .and_utc()
    }

    #[test]
    fn topic_created_at_serializes_in_sqlite_timestamp_format() {
        let topic = Topic {
            topic_id: TopicId::from("topic-1"),
            document_id: DocumentId::from("doc-1"),
            name: "API".to_string(),
            summary: None,
            embedding: None,
            chunk_count: 1,
            created_at: timestamp(),
        };

        let json = serde_json::to_value(&topic).unwrap();
        assert_eq!(json["created_at"], "2026-06-05 12:34:56");
    }

    #[test]
    fn concept_created_at_deserializes_valid_sqlite_timestamp() {
        let concept: Concept = serde_json::from_value(serde_json::json!({
            "concept_id": "concept-1",
            "topic_id": "topic-1",
            "name": "JWT",
            "summary": null,
            "embedding": null,
            "chunk_count": 2,
            "created_at": "2026-06-05 12:34:56"
        }))
        .unwrap();

        assert_eq!(concept.concept_id, ConceptId::from("concept-1"));
        assert_eq!(concept.created_at, timestamp());
    }

    #[test]
    fn topic_created_at_rejects_invalid_timestamp() {
        let err = serde_json::from_value::<Topic>(serde_json::json!({
            "topic_id": "topic-1",
            "document_id": "doc-1",
            "name": "API",
            "summary": null,
            "embedding": null,
            "chunk_count": 1,
            "created_at": "not-a-timestamp"
        }))
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("input contains invalid characters"));
    }
}
