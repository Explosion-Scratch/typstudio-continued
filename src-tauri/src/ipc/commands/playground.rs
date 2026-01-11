use crate::ipc::commands::Result;
use std::fs;
use chrono::Local;

const DEMO_CONTENT: &str = r#"#set page(paper: "a4")
#set text(font: "Libertinus Serif", size: 11pt)

#align(center)[
  #block(text(weight: 700, 1.75em, "Welcome to Typstudio"))
  #v(1em)
  #text(style: "italic", "A modern editor for Typst")
]

#v(2em)

= Introduction
Typst is a new markup-based typesetting system that is designed to be as powerful as LaTeX while being much easier to learn and use.

= Features
- *Fast*: Instant preview as you type.
- *Simple*: Intuitive syntax inspired by Markdown.
- *Powerful*: Full programming language support.

= Math
Typst has excellent support for mathematical notation:

$ Q = rho A v + C $
$ frac(a, b) = c $

Enjoy your typesetting!
"#;

fn get_random_name() -> String {
    let adjectives = ["swift", "vibrant", "elegant", "bold", "serene", "crisp", "lucid", "mighty", "gentle", "radiant"];
    let nouns = ["river", "peak", "forest", "cloud", "meadow", "star", "ocean", "valley", "breeze", "stone"];
    
    let now = Local::now().timestamp_millis() as usize;
    let adj_idx = now % adjectives.len();
    let noun_idx = (now / adjectives.len()) % nouns.len();
    
    format!("{}-{}", adjectives[adj_idx], nouns[noun_idx])
}

#[tauri::command]
pub async fn create_playground() -> Result<String> {
    let documents_dir = dirs::document_dir().ok_or(super::Error::Unknown)?;
    let playground_root = documents_dir.join("playground");
    
    if !playground_root.exists() {
        fs::create_dir_all(&playground_root)?;
    }
    
    let project_name = get_random_name();
    let project_path = playground_root.join(&project_name);
    
    fs::create_dir_all(&project_path)?;
    
    let main_typ_path = project_path.join("main.typ");
    fs::write(main_typ_path, DEMO_CONTENT)?;
    
    Ok(project_path.to_string_lossy().to_string())
}
