//! Scraper

mod error;
mod macros;
mod parse;
pub mod process;
mod request;

pub use self::error::{Error, Result};

const PAGES_PER_BLOCK: usize = 100;
const START_PAGE: usize = 1;
const FILE_PATH: &str = "SSKJ_agg.txt";
/// If set to 0 the limit is set to usize::MAX
const BLOCK_RANGE_LIMIT: usize = 0;
const EXPECTED_NUM_OF_ENTRIES: usize = 97669;

// TODO:
//  -   Try figuring out how to deal with empty pages.
//  -   Test
