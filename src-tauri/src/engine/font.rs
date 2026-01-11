use log::{debug, trace};
use memmap2::Mmap;
use once_cell::sync::OnceCell;
use std::fs::File;
use std::path::{Path, PathBuf};
use typst::text::{Font, FontBook, FontInfo};
use walkdir::WalkDir;

// Taken from typst-cli

/// Holds details about the location of a font and lazily the font itself.
pub struct FontSlot {
    pub path: PathBuf,
    pub index: u32,
    pub font: OnceCell<Option<Font>>,
}

pub struct FontSearcher {
    pub book: FontBook,
    pub fonts: Vec<FontSlot>,
}

impl FontSearcher {
    /// Create a new, empty system searcher.
    pub fn new() -> Self {
        Self {
            book: FontBook::new(),
            fonts: vec![],
        }
    }

    /// Search everything that is available.
    pub fn search(&mut self, font_paths: &[PathBuf], progress: Option<Box<dyn Fn(String, u32) + Send>>) {
        if let Some(ref p) = progress { p("Searching system fonts...".to_string(), 10); }
        self.search_system();

        if let Some(ref p) = progress { p("Searching embedded fonts...".to_string(), 40); }
        self.search_embedded();

        if let Some(ref p) = progress { p("Searching project fonts...".to_string(), 70); }
        for path in font_paths {
            self.search_dir(path);
        }

        log::info!("discovered {} fonts", self.fonts.len());
        if let Some(ref p) = progress { p("Finalizing fonts...".to_string(), 100); }
    }

    /// Add fonts that are embedded in the binary.
    /// Add fonts that are embedded in the binary.
    fn search_embedded(&mut self) {
        use typst::foundations::Bytes;

        log::info!("searching embedded fonts...");
        
        let mut search = |name: &str, bytes: &'static [u8]| {
            let count_before = self.fonts.len();
            for (i, font) in Font::iter(Bytes::new(bytes)).enumerate() {
                let info = font.info();
                log::info!("Embedded Font: {:?} (Variant: {:?})", info.family, info.variant);
                self.book.push(info.clone());
                self.fonts.push(FontSlot {
                    path: PathBuf::new(),
                    index: i as u32,
                    font: OnceCell::from(Some(font)),
                });
            }
            let added = self.fonts.len() - count_before;
            log::info!("embedded font {}: added {} variants", name, added);
        };

        // Embed default fonts.
        search("LinLibertine_R", include_bytes!("../../assets/fonts/LinLibertine_R.ttf"));
        search("LinLibertine_RB", include_bytes!("../../assets/fonts/LinLibertine_RB.ttf"));
        search("LinLibertine_RBI", include_bytes!("../../assets/fonts/LinLibertine_RBI.ttf"));
        search("LinLibertine_RI", include_bytes!("../../assets/fonts/LinLibertine_RI.ttf"));
        search("NewCMMath-Book", include_bytes!("../../assets/fonts/NewCMMath-Book.otf"));
        search("NewCMMath-Regular", include_bytes!("../../assets/fonts/NewCMMath-Regular.otf"));
        search("DejaVuSansMono", include_bytes!("../../assets/fonts/DejaVuSansMono.ttf"));
        search("DejaVuSansMono-Bold", include_bytes!("../../assets/fonts/DejaVuSansMono-Bold.ttf"));
        search(
            "DejaVuSansMono-Oblique",
            include_bytes!("../../assets/fonts/DejaVuSansMono-Oblique.ttf"),
        );
        search(
            "DejaVuSansMono-BoldOblique",
            include_bytes!("../../assets/fonts/DejaVuSansMono-BoldOblique.ttf"),
        );
        search("NewCM10-Regular", include_bytes!("../../assets/fonts/NewCM10-Regular.otf"));
        search("NewCM10-Bold", include_bytes!("../../assets/fonts/NewCM10-Bold.otf"));
        search("NewCM10-Italic", include_bytes!("../../assets/fonts/NewCM10-Italic.otf"));
        search("NewCM10-BoldItalic", include_bytes!("../../assets/fonts/NewCM10-BoldItalic.otf"));
        search("LibertinusSerif-Regular", include_bytes!("../../assets/fonts/LibertinusSerif-Regular.otf"));
        search("LibertinusSerif-Bold", include_bytes!("../../assets/fonts/LibertinusSerif-Bold.otf"));
        search("LibertinusSerif-Italic", include_bytes!("../../assets/fonts/LibertinusSerif-Italic.otf"));
        search("LibertinusSerif-BoldItalic", include_bytes!("../../assets/fonts/LibertinusSerif-BoldItalic.otf"));
        
        log::info!("embedded fonts search complete, total embedded: {}", self.fonts.len());
    }

    /// Search for fonts in the linux system font directories.
    #[cfg(all(unix, not(target_os = "macos")))]
    fn search_system(&mut self) {
        self.search_dir("/usr/share/fonts");
        self.search_dir("/usr/local/share/fonts");

        if let Some(dir) = dirs::font_dir() {
            self.search_dir(dir);
        }
    }

    /// Search for fonts in the macOS system font directories.
    #[cfg(target_os = "macos")]
    fn search_system(&mut self) {
        debug!("searching system fonts on macOS...");
        let before = self.fonts.len();
        
        self.search_dir("/Library/Fonts");
        self.search_dir("/Network/Library/Fonts");
        self.search_dir("/System/Library/Fonts");

        if let Some(dir) = dirs::font_dir() {
            debug!("user font dir: {:?}", dir);
            self.search_dir(dir);
        }
        
        log::info!("system fonts search complete, added {} fonts", self.fonts.len() - before);
    }

    /// Search for fonts in the Windows system font directories.
    #[cfg(windows)]
    fn search_system(&mut self) {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());

        self.search_dir(Path::new(&windir).join("Fonts"));

        if let Some(roaming) = dirs::config_dir() {
            self.search_dir(roaming.join("Microsoft\\Windows\\Fonts"));
        }

        if let Some(local) = dirs::cache_dir() {
            self.search_dir(local.join("Microsoft\\Windows\\Fonts"));
        }
    }

    /// Search for all fonts in a directory recursively.
    fn search_dir(&mut self, path: impl AsRef<Path>) {
        for entry in WalkDir::new(path)
            .follow_links(true)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("ttf" | "otf" | "TTF" | "OTF" | "ttc" | "otc" | "TTC" | "OTC"),
            ) {
                self.search_file(path);
            }
        }
    }

    /// Index the fonts in the file at the given path.
    fn search_file(&mut self, path: impl AsRef<Path>) {
        trace!("searching font file {:?}", path.as_ref());
        let path = path.as_ref();
        if let Ok(file) = File::open(path) {
            if let Ok(mmap) = unsafe { Mmap::map(&file) } {
                for (i, info) in FontInfo::iter(&mmap).enumerate() {
                    log::info!("System Font: {:?} (Variant: {:?})", info.family, info.variant);
                    self.book.push(info);
                    self.fonts.push(FontSlot {
                        path: path.into(),
                        index: i as u32,
                        font: OnceCell::new(),
                    });
                }
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_fonts_are_found() {
        let _ = env_logger::builder().is_test(true).try_init();
        let mut searcher = FontSearcher::new();
        searcher.search_embedded();
        
        println!("Found {} fonts", searcher.fonts.len());
        for font in searcher.fonts.iter() {
           // Embedded fonts have empty path
           assert!(font.path.as_os_str().is_empty());
           assert!(font.font.get().is_some());
           let f = font.font.get().unwrap().as_ref().unwrap();
           println!("Embedded: {:?}", f.info().family);
        }
        
        assert!(searcher.fonts.len() > 0, "Should find at least some embedded fonts");
        
        // Assert specific fonts exist
        let families: Vec<_> = searcher.fonts.iter()
            .map(|slot| slot.font.get().unwrap().as_ref().unwrap().info().family.clone())
            .collect();
            
        assert!(families.iter().any(|f| f == "Linux Libertine"), "Linux Libertine should be present, found: {:?}", families);
        assert!(families.iter().any(|f| f == "New Computer Modern"), "New Computer Modern should be present, found: {:?}", families);
        assert!(families.iter().any(|f| f == "Libertinus Serif"), "Libertinus Serif should be present, found: {:?}", families);
    }
}
