use crate::engine::{FontSearcher, FontSlot};
use typst::utils::LazyHash;
use typst::text::FontBook;
use typst::{Library, LibraryExt};

pub struct TypstEngine {
    pub library: LazyHash<Library>,
    pub fontbook: LazyHash<FontBook>,
    pub fonts: Vec<FontSlot>,
}

impl TypstEngine {
    pub fn new(progress: Option<Box<dyn Fn(String, u32) + Send>>) -> Self {
        let mut searcher = FontSearcher::new();
        searcher.search(&[], progress);



        Self {
            library: LazyHash::new(Library::default()),
            fontbook: LazyHash::new(searcher.book),
            fonts: searcher.fonts,
        }
    }
}
