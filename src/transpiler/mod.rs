use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::{Write, Result, Error, ErrorKind};
use std::path::Path;

use crate::{NAME, VERSION};
use crate::generator::{Codegen, Block};

use dbml_rs::*;

use inflector::Inflector;

mod traits;
use traits::*;
mod err;
mod config;

/// Database entity target.
#[derive(Debug, PartialEq, Clone)]
pub enum Target {
  // MySQL,
  Postgres,
  // Sqlite
}

/// Configuration options for the code generation.
#[derive(Debug, PartialEq, Clone)]
pub struct Config  {
  /// Input file path.
  in_path: OsString,
  /// Output file path (optional). The default output path is `OUT_DIR`.
  out_path: Option<OsString>,
  /// Database entity target.
  target: Target
}

impl Config {
  pub fn new(in_path: impl AsRef<Path>, target: Target) -> Self {
    Self {
      in_path: in_path.as_ref().into(),
      out_path: None,
      target
    }
  }

  pub fn set_out_path(mut self, path: impl AsRef<Path>) -> Self {
    self.out_path = Some(path.as_ref().into());

    self
  }

  pub fn transpile(&self) -> Result<()> {
    let sem_ast = dbml_rs::parse_file(&self.in_path)?;
    
    let result = transpile(sem_ast, &self.target).unwrap_or_else(|e| panic!("{}", e));

    let out_path = match self.out_path.clone() {
      Some(out_path) => out_path,
      _ => {
        env::var_os("OUT_DIR")
        .ok_or_else(|| {
          Error::new(ErrorKind::Other, "OUT_DIR environment variable is not set")
        })?
      }
    };

    File::create(out_path)?.write_all(result.as_bytes())?;

    Ok(())
  }
}

fn transpile(ast: analyzer::SemanticSchemaBlock, target: &Target) -> Result<String> {
  let codegen = Codegen::new()
    .line(format!("//! Generated by {NAME} {VERSION}"))
    .line_skip(1)
    .line("use sea_orm::entity::prelude::*;");

  let codegen = ast.tables.iter().fold(codegen, |acc, table| {
    let ast::table::TableBlock {
      ident,
      cols: fields,
      indexes,
      ..
    } = table.clone();

    let table_block = Block::new(2, Some("pub struct Model"));
    let rel_block = Block::new(2, Some("pub enum Relation"));
    let mut rel_entity_blocks: Vec<_> = vec![];

    // field listing
    let table_block = fields.into_iter().fold(table_block,|acc, field| {
      let mut out_fields = vec![];

      if let Some(exp_type) = field.r#type.to_col_type() {
        out_fields.push(format!(r#"column_type = "{}""#, exp_type))
      }
      if field.settings.is_pk {
        out_fields.push(format!("primary_key"));

        if !field.settings.is_incremental {
          out_fields.push(format!("auto_increment = false"))
        }
      }
      else if table.meta_indexer.pk_list.contains(&field.name) {
        out_fields.push(format!("primary_key"));
      }
      if field.settings.is_nullable {
        out_fields.push(format!("nullable"))
      }
      if field.settings.is_unique || table.meta_indexer.unique_list.contains(&field.name) {
        out_fields.push(format!("unique"))
      }
      if let Some(default) = &field.settings.default {
        let default = if let ast::table::Value::String(val) = default {
          format!(r#""{}""#, val)
        } else {
          default.to_string()
        };

        out_fields.push(format!(r#"default_value = {}"#, default.to_string()))
      }

      let field_rust_type = field.r#type.to_rust_type();
      let field_string = if field.settings.is_nullable {
        format!("Option<{}>", field_rust_type)
      } else {
        field_rust_type
      };
      
      acc
        .line_cond(!out_fields.is_empty(), format!("#[sea_orm({})]", out_fields.join(", ")))
        .line(format!("pub {}: {},", field.name, field_string))
    });

    // relation listing
    let (rto_vec, rby_vec, rself_vec) = ast.get_table_refs(&ident);

    let rel_block = rself_vec.into_iter().fold(rel_block, |acc, rto| {
      let from_field_pascal = rto.lhs.compositions.get(0).unwrap().to_pascal_case();
      let to_field_pascal = rto.rhs.compositions.get(0).unwrap().to_pascal_case();

      let derive = {
        let mut attrs = vec![
            format!(r#"belongs_to = "Entity""#),
            format!(r#"from = "Column::{}""#, from_field_pascal),
            format!(r#"to = "Column::{}""#, to_field_pascal),
          ];

          if let Some(settings) = rto.settings {
            if let Some(action) = settings.on_delete {
              attrs.push(format!(r#"on_delete = "{}""#, action.to_string().to_pascal_case()))
            }
            if let Some(action) = settings.on_update {
              attrs.push(format!(r#"on_update = "{}""#, action.to_string().to_pascal_case()))
            }
          }

          format!(r#"#[sea_orm({})]"#, attrs.join(", "))
      };

      rel_entity_blocks.push(
        Block::new(2, Some("pub struct SelfReferencingLink"))
      );

      rel_entity_blocks.push(
        Block::new(2, Some("impl Linked for SelfReferencingLink"))
          .line("type FromEntity = Entity;")
          .line("type ToEntity = Entity;")
          .line_skip(1)
          .block(
            Block::new(3, Some("fn link(&self) -> Vec<RelationDef>"))
              .line("vec![Relation::SelfReferencing.def()]")
          )
      );

      acc
        .line(derive)
        .line("SelfReferencing,")
    });

    let rel_block = rto_vec.into_iter().fold(rel_block, |acc, rto| {
      let from_field_pascal = rto.lhs.compositions.get(0).unwrap().to_pascal_case();
      let to_field_pascal = rto.rhs.compositions.get(0).unwrap().to_pascal_case();
      let name_pascal = rto.rhs.table.to_pascal_case();
      let name_snake = rto.rhs.table.to_snake_case();

      let derive = match rto.rel {
        ast::refs::Relation::One2One
        | ast::refs::Relation::Many2One => {
          let mut attrs = vec![
            format!(r#"belongs_to = "super::{}::Entity""#, name_snake),
            format!(r#"from = "Column::{}""#, from_field_pascal),
            format!(r#"to = "super::{}::Column::{}""#, name_snake, to_field_pascal),
          ];

          if let Some(settings) = rto.settings {
            if let Some(action) = settings.on_delete {
              attrs.push(format!(r#"on_delete = "{}""#, action.to_string().to_pascal_case()))
            }
            if let Some(action) = settings.on_update {
              attrs.push(format!(r#"on_update = "{}""#, action.to_string().to_pascal_case()))
            }
          }

          format!(r#"#[sea_orm({})]"#, attrs.join(", "))
        },
        _ => panic!("unsupported_rel")
      };

      rel_entity_blocks.push(
        Block::new(2, Some(format!("impl Related<super::{}::Entity> for Entity", name_snake)))
          .block(
            Block::new(3, Some("fn to() -> RelationDef"))
              .line(format!("Relation::{}.def()", name_pascal))
          )
      );

      acc
        .line(derive)
        .line(format!("{},", name_pascal))
    });

    let rel_block = rby_vec.into_iter().fold(rel_block, |acc, rby| {
      let name_pascal = rby.lhs.table.to_pascal_case();
      let name_snake = rby.lhs.table.to_snake_case();

      let derive = match rby.rel {
        ast::refs::Relation::One2One => {
          format!(r#"#[sea_orm(has_one = "super::{}::Entity")]"#, name_snake)
        },
        ast::refs::Relation::Many2One => {
          format!(r#"#[sea_orm(has_many = "super::{}::Entity")]"#, name_snake)
        },
        _ => panic!("unsupported_rel")
      };
      
      rel_entity_blocks.push(
        Block::new(2, Some(format!("impl Related<super::{}::Entity> for Entity", name_snake)))
          .block(
            Block::new(3, Some("fn to() -> RelationDef"))
              .line(format!("Relation::{}.def()", name_pascal))
          )
      );

      acc
        .line(derive)
        .line(format!("{},", name_pascal))
    });

    // construct mod block
    let mod_block = Block::new(1, Some(format!("pub mod {}", &ident.name.to_snake_case())))
      .line("use sea_orm::entity::prelude::*;")
      .line_skip(1)
      .line(format!("#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]"))
      .line(format!(r#"#[sea_orm(table_name = "{}", schema_name = "{}")]"#, &ident.name, &ident.schema.unwrap_or_else(|| "public".into())))
      .block(table_block)
      .line_skip(1)
      .line("#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]")
      .block(rel_block)
      .block_vec(rel_entity_blocks)
      .line_skip(1)
      .line("impl ActiveModelBehavior for ActiveModel {}");

    acc
      .line_skip(1)
      .block(mod_block)
  });

  let codegen = ast.enums.into_iter().fold(codegen, |acc, r#enum| {
    let ast::enums::EnumBlock {
      ident: ast::enums::EnumIdent {
        name,
        schema,
      },
      values,
    } = r#enum;

    let enum_block = Block::new(1, Some(format!("pub enum {}", name.to_pascal_case())));

    let enum_block = values.into_iter().fold(enum_block,|acc, value| {
      let value_name = value.value;

      acc
        .line(format!(r#"#[sea_orm(string_value = "{}")]"#, value_name))
        .line(format!("{},", value_name.to_pascal_case()))
    });

    acc
      .line_skip(1)
      .line("#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum)]")
      .line(
        format!(r#"#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "{}", schema_name = "{}")]"#, name, schema.unwrap_or("public".into()))
      )
      .block(enum_block)
  });

  Ok(codegen.to_string())
}
