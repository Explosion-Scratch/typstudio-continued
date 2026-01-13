use siphasher::sip128::{Hasher128, SipHasher};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use typst::layout::{PagedDocument, Page};

#[derive(Clone)]
pub struct PageRenderCache {
    pub frame_hash: u128,
    pub svg: String,
    pub data_tid: String,
}

pub struct IncrementalRenderer {
    page_cache: HashMap<usize, PageRenderCache>,
    render_version: u64,
}

impl Default for IncrementalRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalRenderer {
    pub fn new() -> Self {
        Self {
            page_cache: HashMap::new(),
            render_version: 0,
        }
    }

    pub fn reset(&mut self) {
        self.page_cache.clear();
        self.render_version = 0;
    }

    fn compute_page_hash(page: &Page) -> u128 {
        let mut hasher = SipHasher::new();
        page.frame.hash(&mut hasher);
        hasher.finish128().as_u128()
    }

    fn generate_data_tid(hash: u128, page_index: usize) -> String {
        format!("p{}-{:016x}", page_index, hash & 0xFFFFFFFFFFFFFFFF)
    }

    fn add_data_tid_to_svg(svg: &str, data_tid: &str) -> String {
        if let Some(pos) = svg.find("<svg") {
            if let Some(end_pos) = svg[pos..].find('>') {
                let insert_pos = pos + end_pos;
                let (before, after) = svg.split_at(insert_pos);
                return format!("{} data-tid=\"{}\"{}",  before, data_tid, after);
            }
        }
        svg.to_string()
    }

    pub fn render_page(&mut self, page_index: usize, page: &Page) -> (String, bool) {
        let frame_hash = Self::compute_page_hash(page);
        
        if let Some(cached) = self.page_cache.get(&page_index) {
            if cached.frame_hash == frame_hash {
                return (cached.svg.clone(), false);
            }
        }
        
        let svg = typst_svg::svg(page);
        let data_tid = Self::generate_data_tid(frame_hash, page_index);
        let svg_with_tid = Self::add_data_tid_to_svg(&svg, &data_tid);
        
        self.page_cache.insert(page_index, PageRenderCache {
            frame_hash,
            svg: svg_with_tid.clone(),
            data_tid,
        });
        
        (svg_with_tid, true)
    }

    pub fn get_changed_pages(&self, document: &PagedDocument) -> Vec<usize> {
        let mut changed = Vec::new();
        
        for (i, page) in document.pages.iter().enumerate() {
            let hash = Self::compute_page_hash(page);
            
            match self.page_cache.get(&i) {
                Some(cached) if cached.frame_hash == hash => {}
                _ => changed.push(i),
            }
        }
        
        let current_count = document.pages.len();
        self.page_cache.keys()
            .filter(|&&i| i >= current_count)
            .for_each(|_| changed.push(current_count));
        
        changed
    }

    pub fn is_page_cached(&self, page_index: usize, page: &Page) -> bool {
        if let Some(cached) = self.page_cache.get(&page_index) {
            cached.frame_hash == Self::compute_page_hash(page)
        } else {
            false
        }
    }

    pub fn get_cached_svg(&self, page_index: usize) -> Option<&str> {
        self.page_cache.get(&page_index).map(|c| c.svg.as_str())
    }

    pub fn prune_pages(&mut self, max_page: usize) {
        self.page_cache.retain(|&k, _| k < max_page);
    }

    pub fn increment_version(&mut self) {
        self.render_version = self.render_version.wrapping_add(1);
    }

    pub fn version(&self) -> u64 {
        self.render_version
    }
}
