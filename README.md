# DBML Entity

#### Database Markup Language (DBML) transpiler for SeaORM Entity.

![MSRV](https://img.shields.io/badge/rustc-1.59+-ab6000.svg)
![MIT or Apache 2.0 licensed](https://img.shields.io/crates/l/sea-orm-dbml.svg)
![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)

## Why DBML?

DBML (Database Markup Language) is an open-source DSL language designed to define and document database schemas and structures. It is designed to be simple, consistent and highly-readable.

Read more: [Official docs](https://www.dbml.org/home/)

This project aims to make use of DBML as a language for writing SeaORM entity.

## The taste

Below is the example of transpiling DBML into SeaORM entity.

```dbml
Table user {
  id integer [pk]
  username varchar
  role varchar
}
```

```rust
//! Generated by sea-orm-dbml 0.1.0

use sea_orm::entity::prelude::*;

pub mod user {
  use sea_orm::entity::prelude::*;

  #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
  #[sea_orm(table_name = "user", schema_name = "public")]
  pub struct Model {
    #[sea_orm(column_type = "Integer", primary_key, auto_increment = false)]
    pub id: i32,
    #[sea_orm(column_type = "String(None)")]
    pub username: String,
    #[sea_orm(column_type = "String(None)")]
    pub role: String,
  }

  #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
  pub enum Relation {}

  impl ActiveModelBehavior for ActiveModel {}
}

```

## How to use it?

TBA. Working in progress.

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

Always welcome you to participate, contribute and together.
