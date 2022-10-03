#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct RefBlock {
  pub rel: Relation,
  pub lhs: Option<RefId>,
  pub rhs: RefId
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub enum Relation {
  #[default] Undef,
  One2One,
  One2Many,
  Many2One,
  Many2Many
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct RefId {
  pub schema: Option<String>,
  pub table: String,
  pub compositions: Vec<String>,
}