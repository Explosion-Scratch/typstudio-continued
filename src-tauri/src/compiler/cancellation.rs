use crate::project::ProjectWorld;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

/// A wrapper around `ProjectWorld` that checks for cancellation
/// before performing expensive operations.
pub struct CancellableWorld<'a> {
    pub world: &'a ProjectWorld,
    pub token: Arc<AtomicBool>,
}

impl<'a> CancellableWorld<'a> {
    pub fn new(world: &'a ProjectWorld, token: Arc<AtomicBool>) -> Self {
        Self { world, token }
    }

    fn check_cancellation(&self) -> FileResult<()> {
        if self.token.load(Ordering::Relaxed) {
            return Err(FileError::Other(Some("compilation cancelled".into())));
        }
        Ok(())
    }
}

impl<'a> World for CancellableWorld<'a> {
    fn library(&self) -> &LazyHash<Library> {
        self.world.library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.world.book()
    }

    fn main(&self) -> FileId {
        self.world.main()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.world.source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.world.file(id)
    }

    fn font(&self, id: usize) -> Option<Font> {
        self.world.font(id)
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        self.world.today(offset)
    }
}
