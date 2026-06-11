use clap::{Args, Subcommand, ValueEnum};
use common::{CiteError, ExitCode};
use config::Config;
use serde::Serialize;
use storage::tags::{TagEntityType, TagRecord};

use super::{exit_for_error, CommandContext};
use crate::output::print_json;

#[derive(Args)]
pub struct TagArgs {
    #[command(subcommand)]
    command: TagCommand,
}

#[derive(Subcommand)]
enum TagCommand {
    /// Set one or more local tags on a document or chunk
    Set(TagMutationArgs),
    /// Get local tags for a document or chunk
    Get(TagGetArgs),
    /// Remove one or more local tags from a document or chunk
    Rm(TagMutationArgs),
}

#[derive(Args)]
struct TagMutationArgs {
    /// Entity ID. Current IDs infer doc_* as document and chunk_* as chunk.
    entity_id: String,
    /// Local tags in key:value form
    tags: Vec<String>,
    /// Explicit entity type when ID prefix is ambiguous
    #[arg(long = "entity-type")]
    entity_type: Option<EntityTypeArg>,
}

#[derive(Args)]
struct TagGetArgs {
    /// Entity ID. Current IDs infer doc_* as document and chunk_* as chunk.
    entity_id: String,
    /// Explicit entity type when ID prefix is ambiguous
    #[arg(long = "entity-type")]
    entity_type: Option<EntityTypeArg>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum EntityTypeArg {
    Document,
    Chunk,
}

impl From<EntityTypeArg> for TagEntityType {
    fn from(value: EntityTypeArg) -> Self {
        match value {
            EntityTypeArg::Document => TagEntityType::Document,
            EntityTypeArg::Chunk => TagEntityType::Chunk,
        }
    }
}

#[derive(Serialize)]
struct TagMutationOutput {
    entity_id: String,
    entity_type: String,
    tags: Vec<String>,
}

#[derive(Serialize)]
struct TagGetOutput {
    entity_id: String,
    entity_type: String,
    tags: Vec<String>,
}

pub fn execute(args: &TagArgs, config: &Config, json: bool) -> i32 {
    let ctx = match CommandContext::open_db_only(config, json) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };

    let result = match &args.command {
        TagCommand::Set(args) => execute_set(args, &ctx, json),
        TagCommand::Get(args) => execute_get(args, &ctx, json),
        TagCommand::Rm(args) => execute_rm(args, &ctx, json),
    };

    match result {
        Ok(()) => ExitCode::Success as i32,
        Err(e) => exit_for_error(&e, json),
    }
}

fn execute_set(args: &TagMutationArgs, ctx: &CommandContext, json: bool) -> Result<(), CiteError> {
    let entity_type = resolve_entity_type(&args.entity_id, args.entity_type)?;
    let tags = parse_mutation_tags(&args.tags)?;

    for tag in &tags {
        ctx.db.set_tag_user(entity_type, &args.entity_id, tag)?;
    }

    render_mutation("Set", &args.entity_id, entity_type, &tags, json);
    Ok(())
}

fn execute_get(args: &TagGetArgs, ctx: &CommandContext, json: bool) -> Result<(), CiteError> {
    let entity_type = resolve_entity_type(&args.entity_id, args.entity_type)?;
    let tags = ctx.db.list_tags(entity_type, &args.entity_id)?;
    let tag_strings = tag_strings(&tags);

    if json {
        print_json(&TagGetOutput {
            entity_id: args.entity_id.clone(),
            entity_type: entity_type_label(entity_type).to_string(),
            tags: tag_strings,
        });
    } else if tag_strings.is_empty() {
        println!("No tags found for {}.", args.entity_id);
    } else {
        println!("Tags for {}:", args.entity_id);
        for tag in &tag_strings {
            println!("  {tag}");
        }
    }
    Ok(())
}

fn execute_rm(args: &TagMutationArgs, ctx: &CommandContext, json: bool) -> Result<(), CiteError> {
    let entity_type = resolve_entity_type(&args.entity_id, args.entity_type)?;
    let tags = parse_mutation_tags(&args.tags)?;

    for tag in &tags {
        ctx.db.remove_tag_user(entity_type, &args.entity_id, tag)?;
    }

    render_mutation("Removed", &args.entity_id, entity_type, &tags, json);
    Ok(())
}

fn render_mutation(
    action: &str,
    entity_id: &str,
    entity_type: TagEntityType,
    tags: &[TagRecord],
    json: bool,
) {
    let tag_strings = tag_strings(tags);
    if json {
        print_json(&TagMutationOutput {
            entity_id: entity_id.to_string(),
            entity_type: entity_type_label(entity_type).to_string(),
            tags: tag_strings,
        });
    } else {
        println!(
            "{} {} tag(s) on {} {}:",
            action,
            tags.len(),
            entity_type_label(entity_type),
            entity_id
        );
        for tag in tag_strings {
            println!("  {tag}");
        }
    }
}

fn parse_mutation_tags(inputs: &[String]) -> Result<Vec<TagRecord>, CiteError> {
    if inputs.is_empty() {
        return Err(CiteError::InvalidParameter {
            message: "At least one key:value tag is required".to_string(),
        });
    }
    inputs
        .iter()
        .map(|input| TagRecord::parse_mutation(input))
        .collect()
}

fn resolve_entity_type(
    entity_id: &str,
    explicit: Option<EntityTypeArg>,
) -> Result<TagEntityType, CiteError> {
    if let Some(entity_type) = explicit {
        return Ok(entity_type.into());
    }

    if entity_id.starts_with("doc_") {
        Ok(TagEntityType::Document)
    } else if entity_id.starts_with("chunk_") {
        Ok(TagEntityType::Chunk)
    } else {
        Err(CiteError::InvalidParameter {
            message: "Could not infer entity type from ID; pass --entity-type document|chunk"
                .to_string(),
        })
    }
}

fn tag_strings(tags: &[TagRecord]) -> Vec<String> {
    tags.iter()
        .map(|tag| format!("{}:{}", tag.key, tag.value))
        .collect()
}

fn entity_type_label(entity_type: TagEntityType) -> &'static str {
    match entity_type {
        TagEntityType::Document => "document",
        TagEntityType::Chunk => "chunk",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_parser_accepts_key_value_tags() {
        let tags = parse_mutation_tags(&["status:implemented".to_string(), "tag:auth".to_string()])
            .unwrap();

        assert_eq!(
            tags,
            vec![
                TagRecord {
                    key: "status".to_string(),
                    value: "implemented".to_string(),
                },
                TagRecord {
                    key: "tag".to_string(),
                    value: "auth".to_string(),
                },
            ]
        );
    }

    #[test]
    fn mutation_parser_rejects_key_only_and_empty_tags() {
        assert!(parse_mutation_tags(&["status".to_string()]).is_err());
        assert!(parse_mutation_tags(&["status:".to_string()]).is_err());
        assert!(parse_mutation_tags(&[]).is_err());
    }

    #[test]
    fn entity_type_infers_current_id_prefixes() {
        assert_eq!(
            resolve_entity_type("doc_abc", None).unwrap(),
            TagEntityType::Document
        );
        assert_eq!(
            resolve_entity_type("chunk_abc", None).unwrap(),
            TagEntityType::Chunk
        );
    }

    #[test]
    fn entity_type_allows_explicit_ambiguous_ids() {
        assert_eq!(
            resolve_entity_type("legacy-id", Some(EntityTypeArg::Document)).unwrap(),
            TagEntityType::Document
        );
        assert_eq!(
            resolve_entity_type("legacy-id", Some(EntityTypeArg::Chunk)).unwrap(),
            TagEntityType::Chunk
        );
    }

    #[test]
    fn entity_type_rejects_ambiguous_ids_without_explicit_type() {
        assert!(resolve_entity_type("legacy-id", None).is_err());
    }

    #[test]
    fn user_tag_storage_rejects_reserved_keys_from_cli_path() {
        let db = storage::Database::open_memory().unwrap();
        let tags = parse_mutation_tags(&["workspace:aiharness".to_string()]).unwrap();

        assert!(db
            .set_tag_user(TagEntityType::Document, "doc_abc", &tags[0])
            .is_err());
    }

    #[test]
    fn remove_uses_exact_local_key_value_pair() {
        let db = storage::Database::open_memory().unwrap();
        let planned = TagRecord::parse_mutation("status:planned").unwrap();
        let implemented = TagRecord::parse_mutation("status:implemented").unwrap();

        db.set_tag_user(TagEntityType::Chunk, "chunk_abc", &planned)
            .unwrap();
        db.set_tag_user(TagEntityType::Chunk, "chunk_abc", &implemented)
            .unwrap();

        assert_eq!(
            db.remove_tag_user(TagEntityType::Chunk, "chunk_abc", &planned)
                .unwrap(),
            1
        );
        assert_eq!(
            db.list_tags(TagEntityType::Chunk, "chunk_abc").unwrap(),
            vec![implemented]
        );
    }
}
