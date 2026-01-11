use crate::engine::TypstEngine;
use chrono::Datelike;
use typst::utils::LazyHash;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use typst::diag::{FileError, FileResult, PackageError, PackageResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::Library;
use typst::World;
use typst_ide::IdeWorld;

pub struct ProjectWorld {
    root: PathBuf,
    engine: Arc<TypstEngine>,

    slots: RwLock<HashMap<FileId, PathSlot>>,

    main: Option<FileId>,
}

impl ProjectWorld {
    pub fn slot_update<P: AsRef<Path>>(
        &self,
        path: P,
        content: Option<String>,
    ) -> FileResult<FileId> {
        let vpath = VirtualPath::new(path);
        let id = FileId::new(None, vpath.clone());
        
        let mut slots = self.slots.write().unwrap();
        
        if let Entry::Vacant(_) = &slots.entry(id) {
            let buf;
            let mut root = &self.root;
            if let Some(spec) = id.package() {
                buf = Self::prepare_package(spec)?;
                root = &buf;
            }
            let path = id.vpath().resolve(root).ok_or(FileError::AccessDenied)?;
            slots.insert(id, PathSlot {
                id,
                path,
                source: RwLock::new(None),
                buffer: RwLock::new(None),
            });
        }
        
        let slot = slots.get(&id).unwrap();
        
        if let Some(ref content_str) = content {
            let bytes = Bytes::new(content_str.as_bytes().to_vec());
            *slot.buffer.write().unwrap() = Some(Ok(bytes));
            
            let mut source_guard = slot.source.write().unwrap();
            match source_guard.as_mut() {
                Some(Ok(src)) => {
                    src.replace(content_str);
                }
                _ => {
                    *source_guard = Some(Ok(Source::new(id, content_str.clone())));
                }
            }
        }
        
        Ok(id)
    }

    pub fn set_main(&mut self, id: Option<FileId>) {
        self.main = id
    }

    pub fn set_main_path(&mut self, main: VirtualPath) {
        self.set_main(Some(FileId::new(None, main)))
    }

    pub fn is_main_set(&self) -> bool {
        self.main.is_some()
    }

    pub fn new(root: PathBuf, progress: Option<Box<dyn Fn(String, u32) + Send>>) -> Self {
        Self {
            root,
            engine: Arc::new(TypstEngine::new(progress)),
            slots: RwLock::new(HashMap::new()),
            main: None,
        }
    }

    fn take_or_read(&self, vpath: &VirtualPath, content: Option<String>) -> FileResult<String> {
        if let Some(content) = content {
            return Ok(content);
        }

        let path = vpath.resolve(&self.root).ok_or(FileError::AccessDenied)?;
        fs::read_to_string(&path).map_err(|e| FileError::from_io(e, &path))
    }

    fn prepare_package(spec: &PackageSpec) -> PackageResult<PathBuf> {
        let subdir = format!(
            "typst/packages/{}/{}/{}",
            spec.namespace, spec.name, spec.version
        );

        if let Some(data_dir) = dirs::data_dir() {
            let dir = data_dir.join(&subdir);
            if dir.exists() {
                return Ok(dir);
            }
        }

        if let Some(cache_dir) = dirs::cache_dir() {
            let dir = cache_dir.join(&subdir);
            if dir.exists() {
                return Ok(dir);
            }
        }

        Err(PackageError::NotFound(spec.clone()))
    }
}

unsafe impl Send for ProjectWorld {}
unsafe impl Sync for ProjectWorld {}

impl World for ProjectWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.engine.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.engine.fontbook
    }

    fn main(&self) -> FileId {
        self.main.expect("the main file must be set")
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        let slots = self.slots.read().unwrap();
        if let Some(slot) = slots.get(&id) {
            return slot.source();
        }
        drop(slots);
        
        let mut slots = self.slots.write().unwrap();
        let buf;
        let mut root = &self.root;
        if let Some(spec) = id.package() {
            buf = Self::prepare_package(spec)?;
            root = &buf;
        }
        let path = id.vpath().resolve(root).ok_or(FileError::AccessDenied)?;
        
        let slot = slots.entry(id).or_insert_with(|| PathSlot {
            id,
            path,
            source: RwLock::new(None),
            buffer: RwLock::new(None),
        });
        slot.source()
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let slots = self.slots.read().unwrap();
        if let Some(slot) = slots.get(&id) {
            return slot.file();
        }
        drop(slots);
        
        let mut slots = self.slots.write().unwrap();
        let buf;
        let mut root = &self.root;
        if let Some(spec) = id.package() {
            buf = Self::prepare_package(spec)?;
            root = &buf;
        }
        let path = id.vpath().resolve(root).ok_or(FileError::AccessDenied)?;
        
        let slot = slots.entry(id).or_insert_with(|| PathSlot {
            id,
            path,
            source: RwLock::new(None),
            buffer: RwLock::new(None),
        });
        slot.file()
    }

    fn font(&self, id: usize) -> Option<Font> {
        let slot = &self.engine.fonts[id];
        slot.font
            .get_or_init(|| {
                let data = fs::read(&slot.path).map(|v| Bytes::new(v)).ok()?;
                Font::new(data, slot.index)
            })
            .clone()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let dt = match offset {
            None => chrono::Local::now().naive_local(),
            Some(o) => (chrono::Utc::now() + chrono::Duration::try_hours(o)?).naive_utc(),
        };
        Datetime::from_ymd(
            dt.year(),
            dt.month().try_into().ok()?,
            dt.day().try_into().ok()?,
        )
    }
}

struct PathSlot {
    id: FileId,
    path: PathBuf,
    source: RwLock<Option<FileResult<Source>>>,
    buffer: RwLock<Option<FileResult<Bytes>>>,
}

impl PathSlot {
    fn source(&self) -> FileResult<Source> {
        let guard = self.source.read().unwrap();
        if let Some(ref result) = *guard {
            return result.clone();
        }
        drop(guard);
        
        let mut guard = self.source.write().unwrap();
        if let Some(ref result) = *guard {
            return result.clone();
        }
        
        let result = fs::read_to_string(&self.path)
            .map_err(|e| FileError::from_io(e, &self.path))
            .map(|text| Source::new(self.id, text));
        *guard = Some(result.clone());
        result
    }

    fn file(&self) -> FileResult<Bytes> {
        let guard = self.buffer.read().unwrap();
        if let Some(ref result) = *guard {
            return result.clone();
        }
        drop(guard);
        
        let mut guard = self.buffer.write().unwrap();
        if let Some(ref result) = *guard {
            return result.clone();
        }
        
        let result = fs::read(&self.path)
            .map(|v| Bytes::new(v))
            .map_err(|e| FileError::from_io(e, &self.path));
        *guard = Some(result.clone());
        result
    }
}

impl IdeWorld for ProjectWorld {
    fn upcast(&self) -> &dyn World {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_compile_with_new_computer_modern() {
        let _ = env_logger::builder().is_test(true).try_init();
        
        // 1. Initialize world
        let root = PathBuf::from(".");
        let mut world = ProjectWorld::new(root, None);
        
        // 2. Set main file content
        let content = r#"#set text(font: "New Computer Modern")
Hello World"#;
        let path = PathBuf::from("main.typ");
        
        world.slot_update(&path, Some(content.to_string())).unwrap();
        world.set_main_path(typst::syntax::VirtualPath::new("main.typ"));
        
        // 3. Compile
        let result = typst::compile::<typst::layout::PagedDocument>(&world);
        
        if !result.warnings.is_empty() {
             println!("Warnings: {:?}", result.warnings);
        }

        // 4. Assert success and NO font warnings
        let font_missing = result.warnings.iter().any(|w| w.message.contains("font family not found") || w.message.contains("default font"));
        assert!(!font_missing, "New Computer Modern should be found. Warnings: {:?}", result.warnings);
        
        assert!(result.output.is_ok(), "Compilation failed");
    }
}
