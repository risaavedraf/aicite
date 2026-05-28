use common::types::{
    Chunk, ContextMetadataScaffold, TraceCitationRecord, TraceEnvelope, TraceHeaderInput,
    TraceHeaderRecord,
};
use common::CiteError;
use rusqlite::params;

use crate::util::{parse_dt, storage_err};
use crate::Database;

fn row_to_chunk(row: &rusqlite::Row<'_>) -> Result<Chunk, CiteError> {
    let created_at_str: String = row.get("created_at").map_err(storage_err)?;

    Ok(Chunk {
        chunk_id: row.get("chunk_id").map_err(storage_err)?,
        document_id: row.get("document_id").map_err(storage_err)?,
        section_id: row.get("section_id").map_err(storage_err)?,
        chunk_index: row.get::<_, i64>("chunk_index").map_err(storage_err)? as u32,
        text: row.get("text").map_err(storage_err)?,
        page: row
            .get::<_, Option<i64>>("page")
            .map_err(storage_err)?
            .map(|v| v as u32),
        offset_start: row
            .get::<_, Option<i64>>("offset_start")
            .map_err(storage_err)?
            .map(|v| v as u32),
        offset_end: row
            .get::<_, Option<i64>>("offset_end")
            .map_err(storage_err)?
            .map(|v| v as u32),
        created_at: parse_dt(&created_at_str)?,
    })
}

impl Database {
    /// Persist a trace header and its citations atomically.
    pub fn persist_trace_with_citations(
        &self,
        header: &TraceHeaderInput,
        citations: &[TraceCitationRecord],
    ) -> Result<(), CiteError> {
        let tx = self.conn.unchecked_transaction().map_err(storage_err)?;

        let citation_ids = header.citation_ids.clone().or_else(|| {
            if citations.is_empty() {
                None
            } else {
                Some(
                    citations
                        .iter()
                        .map(|c| c.citation_id.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                )
            }
        });

        tx.execute(
            "INSERT INTO traces (
                trace_id,
                query_id,
                context_pack_id,
                request_type,
                document_ids,
                citation_ids,
                top_k,
                evidence_floor,
                confidence_threshold,
                ranking_method,
                latency_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                header.trace_id,
                header.query_id,
                header.context_pack_id,
                header.request_type,
                header.document_ids,
                citation_ids,
                header.top_k.map(|v| v as i64),
                header.evidence_floor,
                header.confidence_threshold,
                header.ranking_method,
                header.latency_ms.map(|v| v as i64),
            ],
        )
        .map_err(storage_err)?;

        for citation in citations {
            if citation.trace_id != header.trace_id {
                return Err(CiteError::StorageError {
                    message: format!(
                        "Citation {} belongs to trace {}, expected {}",
                        citation.citation_id, citation.trace_id, header.trace_id
                    ),
                });
            }

            tx.execute(
                "INSERT INTO trace_citations (
                    trace_id,
                    citation_id,
                    document_id,
                    display_name,
                    chunk_id,
                    page,
                    offset_start,
                    offset_end,
                    text,
                    score,
                    confidence_label
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    citation.trace_id,
                    citation.citation_id,
                    citation.document_id,
                    citation.display_name,
                    citation.chunk_id,
                    citation.page.map(|v| v as i64),
                    citation.offset_start.map(|v| v as i64),
                    citation.offset_end.map(|v| v as i64),
                    citation.text,
                    citation.score,
                    citation.confidence_label,
                ],
            )
            .map_err(storage_err)?;
        }

        tx.commit().map_err(storage_err)?;
        Ok(())
    }

    /// Get a citation by scoped `(trace_id, citation_id)`.
    pub fn get_citation_by_trace(
        &self,
        trace_id: &str,
        citation_id: &str,
    ) -> Result<TraceCitationRecord, CiteError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT
                    trace_id,
                    citation_id,
                    document_id,
                    display_name,
                    chunk_id,
                    page,
                    offset_start,
                    offset_end,
                    text,
                    score,
                    confidence_label
                 FROM trace_citations
                 WHERE trace_id = ?1 AND citation_id = ?2",
            )
            .map_err(storage_err)?;

        let mut rows = stmt
            .query(params![trace_id, citation_id])
            .map_err(storage_err)?;

        if let Some(row) = rows.next().map_err(storage_err)? {
            Ok(TraceCitationRecord {
                trace_id: row.get(0).map_err(storage_err)?,
                citation_id: row.get(1).map_err(storage_err)?,
                document_id: row.get(2).map_err(storage_err)?,
                display_name: row.get(3).map_err(storage_err)?,
                chunk_id: row.get(4).map_err(storage_err)?,
                page: row
                    .get::<_, Option<i64>>(5)
                    .map_err(storage_err)?
                    .map(|v| v as u32),
                offset_start: row
                    .get::<_, Option<i64>>(6)
                    .map_err(storage_err)?
                    .map(|v| v as u32),
                offset_end: row
                    .get::<_, Option<i64>>(7)
                    .map_err(storage_err)?
                    .map(|v| v as u32),
                text: row.get(8).map_err(storage_err)?,
                score: row.get(9).map_err(storage_err)?,
                confidence_label: row.get(10).map_err(storage_err)?,
            })
        } else {
            Err(CiteError::CitationNotFound {
                citation_id: citation_id.to_string(),
            })
        }
    }

    /// Get a chunk by `(document_id, chunk_id)` only when the document is ready.
    pub fn get_ready_chunk_by_document(
        &self,
        document_id: &str,
        chunk_id: &str,
    ) -> Result<Chunk, CiteError> {
        let mut status_stmt = self
            .conn
            .prepare("SELECT status FROM documents WHERE document_id = ?1")
            .map_err(storage_err)?;
        let mut status_rows = status_stmt
            .query(params![document_id])
            .map_err(storage_err)?;

        let status = match status_rows.next().map_err(storage_err)? {
            Some(row) => row.get::<_, String>(0).map_err(storage_err)?,
            None => {
                return Err(CiteError::DocumentNotFound {
                    document_id: document_id.to_string(),
                });
            }
        };

        if status != "ready" {
            return Err(CiteError::DocumentNotReady {
                document_id: document_id.to_string(),
            });
        }

        let mut stmt = self
            .conn
            .prepare(
                "SELECT chunk_id, document_id, section_id, chunk_index, text, page, offset_start, offset_end, created_at
                 FROM chunks
                 WHERE document_id = ?1 AND chunk_id = ?2",
            )
            .map_err(storage_err)?;
        let mut rows = stmt
            .query(params![document_id, chunk_id])
            .map_err(storage_err)?;

        if let Some(row) = rows.next().map_err(storage_err)? {
            row_to_chunk(row)
        } else {
            Err(CiteError::ChunkNotFound {
                chunk_id: chunk_id.to_string(),
            })
        }
    }

    /// Get a trace envelope by trace id.
    pub fn get_trace_envelope(&self, trace_id: &str) -> Result<TraceEnvelope, CiteError> {
        let mut trace_stmt = self
            .conn
            .prepare(
                "SELECT
                    trace_id,
                    query_id,
                    context_pack_id,
                    request_type,
                    document_ids,
                    citation_ids,
                    top_k,
                    evidence_floor,
                    confidence_threshold,
                    ranking_method,
                    latency_ms,
                    created_at
                 FROM traces
                 WHERE trace_id = ?1",
            )
            .map_err(storage_err)?;

        let mut trace_rows = trace_stmt.query(params![trace_id]).map_err(storage_err)?;

        let header = if let Some(row) = trace_rows.next().map_err(storage_err)? {
            let created_at: String = row.get(11).map_err(storage_err)?;
            TraceHeaderRecord {
                trace_id: row.get(0).map_err(storage_err)?,
                query_id: row.get(1).map_err(storage_err)?,
                context_pack_id: row.get(2).map_err(storage_err)?,
                request_type: row.get(3).map_err(storage_err)?,
                document_ids: row.get(4).map_err(storage_err)?,
                citation_ids: row.get(5).map_err(storage_err)?,
                top_k: row
                    .get::<_, Option<i64>>(6)
                    .map_err(storage_err)?
                    .map(|v| v as u32),
                evidence_floor: row.get(7).map_err(storage_err)?,
                confidence_threshold: row.get(8).map_err(storage_err)?,
                ranking_method: row.get(9).map_err(storage_err)?,
                latency_ms: row
                    .get::<_, Option<i64>>(10)
                    .map_err(storage_err)?
                    .map(|v| v as u64),
                created_at: parse_dt(&created_at)?,
            }
        } else {
            return Err(CiteError::TraceNotFound {
                trace_id: trace_id.to_string(),
            });
        };

        let mut citation_stmt = self
            .conn
            .prepare(
                "SELECT
                    trace_id,
                    citation_id,
                    document_id,
                    display_name,
                    chunk_id,
                    page,
                    offset_start,
                    offset_end,
                    text,
                    score,
                    confidence_label
                 FROM trace_citations
                 WHERE trace_id = ?1
                 ORDER BY citation_id ASC",
            )
            .map_err(storage_err)?;
        let mut citation_rows = citation_stmt
            .query(params![trace_id])
            .map_err(storage_err)?;
        let mut citations = Vec::new();

        while let Some(row) = citation_rows.next().map_err(storage_err)? {
            citations.push(TraceCitationRecord {
                trace_id: row.get(0).map_err(storage_err)?,
                citation_id: row.get(1).map_err(storage_err)?,
                document_id: row.get(2).map_err(storage_err)?,
                display_name: row.get(3).map_err(storage_err)?,
                chunk_id: row.get(4).map_err(storage_err)?,
                page: row
                    .get::<_, Option<i64>>(5)
                    .map_err(storage_err)?
                    .map(|v| v as u32),
                offset_start: row
                    .get::<_, Option<i64>>(6)
                    .map_err(storage_err)?
                    .map(|v| v as u32),
                offset_end: row
                    .get::<_, Option<i64>>(7)
                    .map_err(storage_err)?
                    .map(|v| v as u32),
                text: row.get(8).map_err(storage_err)?,
                score: row.get(9).map_err(storage_err)?,
                confidence_label: row.get(10).map_err(storage_err)?,
            });
        }

        let excluded_non_ready_document_ids = self.list_non_ready_document_ids()?;
        let context_metadata = ContextMetadataScaffold {
            excluded_non_ready_document_count: excluded_non_ready_document_ids.len() as u32,
            excluded_non_ready_document_ids,
        };

        Ok(TraceEnvelope {
            header,
            citations,
            context_metadata,
        })
    }

    /// List all non-ready document IDs.
    pub fn list_non_ready_document_ids(&self) -> Result<Vec<String>, CiteError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT document_id
                 FROM documents
                 WHERE status != 'ready'
                 ORDER BY document_id ASC",
            )
            .map_err(storage_err)?;

        let mut rows = stmt.query([]).map_err(storage_err)?;
        let mut ids = Vec::new();

        while let Some(row) = rows.next().map_err(storage_err)? {
            ids.push(row.get(0).map_err(storage_err)?);
        }

        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use common::types::{Document, DocumentStatus, FileType};
    use std::path::PathBuf;

    fn insert_document(db: &Database, document_id: &str, status: DocumentStatus) {
        let doc = Document {
            document_id: document_id.to_string(),
            display_name: format!("{document_id}.txt"),
            file_path: PathBuf::from(format!("/docs/{document_id}.txt")),
            file_type: FileType::Txt,
            file_size_bytes: 100,
            status,
            chunk_count: 0,
            retry_count: 0,
            max_retry_count: 3,
            next_retry_at: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.insert_document(&doc).unwrap();
    }

    fn insert_chunk(db: &Database, document_id: &str, chunk_id: &str) {
        db.insert_chunks(
            document_id,
            &[Chunk {
                chunk_id: chunk_id.to_string(),
                document_id: document_id.to_string(),
                section_id: None,
                chunk_index: 0,
                text: "chunk text".to_string(),
                page: None,
                offset_start: None,
                offset_end: None,
                created_at: Utc::now(),
            }],
        )
        .unwrap();
    }

    fn make_trace_header(trace_id: &str) -> TraceHeaderInput {
        TraceHeaderInput {
            trace_id: trace_id.to_string(),
            query_id: Some("qry-1".to_string()),
            context_pack_id: Some("ctx-1".to_string()),
            request_type: "context".to_string(),
            document_ids: Some("doc-1".to_string()),
            citation_ids: None,
            top_k: Some(5),
            evidence_floor: Some(0.5),
            confidence_threshold: Some(0.7),
            ranking_method: Some("vector_cosine_v1".to_string()),
            latency_ms: Some(123),
        }
    }

    #[test]
    fn test_get_citation_by_trace_disambiguates_same_citation_id() {
        let db = Database::open_memory().unwrap();

        db.persist_trace_with_citations(
            &make_trace_header("trace-a"),
            &[TraceCitationRecord {
                trace_id: "trace-a".to_string(),
                citation_id: "c1".to_string(),
                document_id: "doc-a".to_string(),
                display_name: "a.txt".to_string(),
                chunk_id: "chunk-a".to_string(),
                page: Some(1),
                offset_start: Some(0),
                offset_end: Some(10),
                text: "from trace a".to_string(),
                score: Some(0.91),
                confidence_label: Some("high".to_string()),
            }],
        )
        .unwrap();

        db.persist_trace_with_citations(
            &make_trace_header("trace-b"),
            &[TraceCitationRecord {
                trace_id: "trace-b".to_string(),
                citation_id: "c1".to_string(),
                document_id: "doc-b".to_string(),
                display_name: "b.txt".to_string(),
                chunk_id: "chunk-b".to_string(),
                page: Some(2),
                offset_start: Some(11),
                offset_end: Some(20),
                text: "from trace b".to_string(),
                score: Some(0.65),
                confidence_label: Some("medium".to_string()),
            }],
        )
        .unwrap();

        let citation_a = db.get_citation_by_trace("trace-a", "c1").unwrap();
        let citation_b = db.get_citation_by_trace("trace-b", "c1").unwrap();

        assert_eq!(citation_a.text, "from trace a");
        assert_eq!(citation_b.text, "from trace b");
        assert_eq!(citation_a.chunk_id, "chunk-a");
        assert_eq!(citation_b.chunk_id, "chunk-b");
    }

    #[test]
    fn test_get_ready_chunk_by_document_only_returns_ready_documents() {
        let db = Database::open_memory().unwrap();

        insert_document(&db, "doc-ready", DocumentStatus::Ready);
        insert_document(&db, "doc-processing", DocumentStatus::Processing);
        insert_chunk(&db, "doc-ready", "chunk-ready");
        insert_chunk(&db, "doc-processing", "chunk-processing");

        let ready_chunk = db
            .get_ready_chunk_by_document("doc-ready", "chunk-ready")
            .unwrap();
        assert_eq!(ready_chunk.chunk_id, "chunk-ready");

        let not_ready_err = db
            .get_ready_chunk_by_document("doc-processing", "chunk-processing")
            .unwrap_err();
        assert!(matches!(
            not_ready_err,
            CiteError::DocumentNotReady { .. }
        ));
    }

    #[test]
    fn test_trace_citation_and_chunk_not_found_behavior() {
        let db = Database::open_memory().unwrap();

        insert_document(&db, "doc-ready", DocumentStatus::Ready);
        db.persist_trace_with_citations(&make_trace_header("trace-existing"), &[])
            .unwrap();

        let trace_err = db.get_trace_envelope("trace-missing").unwrap_err();
        assert!(matches!(trace_err, CiteError::TraceNotFound { .. }));

        let citation_err = db
            .get_citation_by_trace("trace-existing", "citation-missing")
            .unwrap_err();
        assert!(matches!(
            citation_err,
            CiteError::CitationNotFound { .. }
        ));

        let chunk_err = db
            .get_ready_chunk_by_document("doc-ready", "chunk-missing")
            .unwrap_err();
        assert!(matches!(chunk_err, CiteError::ChunkNotFound { .. }));
    }
}
