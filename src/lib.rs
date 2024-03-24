//! FINAL FRAN SCRAPER
//! MORE WORK IN A SEPARATE REPO
// TODO:
//  -   Split into logical modules
//  -   Document
//  -   Try figuring out how to deal with empty pages.
//  -   Test

mod macros;
mod parse;
pub mod process;
mod request;

const PAGES_PER_BLOCK: usize = 50;
const START_PAGE: usize = 1;
const FILE_PATH: &str = "SSKJ_agg.txt";
/// If set to 0 the limit is set to usize::MAX
const BLOCK_RANGE_LIMIT: usize = 0;
const EXPECTED_NUM_OF_ENTRIES: usize = 97669;
