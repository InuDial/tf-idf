pub mod data;
pub mod lexer;
pub mod map;
pub mod small_str;
pub mod trie;

pub use lexer::MAX_TERM_LENGTH;
pub type Term = small_str::SmallString<MAX_TERM_LENGTH>;
