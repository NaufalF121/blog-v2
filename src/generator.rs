use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use tera::{Context, Tera};

#[derive(Clone)]
pub struct Post {
    pub title: String,
    pub slug: String,
    pub date: String,
    pub excerpt: String,
    pub html_content: String,
}

// Global Tera instance that persists across builds
static TERA_INSTANCE: OnceLock<Arc<Mutex<Tera>>> = OnceLock::new();

fn get_tera() -> Arc<Mutex<Tera>> {
    TERA_INSTANCE
        .get_or_init(|| {
            let mut tera = Tera::default();

            // Load templates manually
            let post_template =
                fs::read_to_string("templates/post.html").expect("Failed to read post.html");
            let index_template =
                fs::read_to_string("templates/index.html").expect("Failed to read index.html");
            let base_css =
                fs::read_to_string("templates/base.css").expect("Failed to read base.css");

            tera.add_raw_template("post.html", &post_template)
                .expect("Failed to add post template");
            tera.add_raw_template("index.html", &index_template)
                .expect("Failed to add index template");
            tera.add_raw_template("base.css", &base_css)
                .expect("Failed to add CSS template");

            Arc::new(Mutex::new(tera))
        })
        .clone()
}

pub fn build_blog() -> std::io::Result<()> {
    fs::create_dir_all("output")?;

    let posts_dir = "posts";
    let mut posts = Vec::new();

    if Path::new(posts_dir).exists() {
        for entry in fs::read_dir(posts_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(post) = parse_post(&path, &content) {
                        posts.push(post);
                    }
                }
            }
        }
    }

    posts.sort_by(|a, b| b.date.cmp(&a.date));

    // Copy images folder if it exists
    let images_src = "posts/images";
    let images_dest = "output/images";
    if Path::new(images_src).exists() {
        fs::create_dir_all(images_dest)?;
        for entry in fs::read_dir(images_src)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().unwrap();
                let dest_path = format!("output/images/{}", file_name.to_string_lossy());
                fs::copy(&path, &dest_path)?;
                println!("🖼️  Copied: {}", dest_path);
            }
        }
    }

    let tera_arc = get_tera();

    for post in &posts {
        let tera = tera_arc.lock().unwrap();
        let html = generate_post_page(&tera, post);
        drop(tera);
        let output_path = format!("output/{}.html", post.slug);
        fs::write(&output_path, html)?;
        println!("📄 Generated: {}", output_path);
    }

    // Generate index page
    let tera = tera_arc.lock().unwrap();
    let index_html = generate_index_page(&tera, &posts);
    drop(tera);
    fs::write("output/index.html", index_html)?;
    println!("🏠 Generated: output/index.html");

    Ok(())
}

fn parse_post(path: &Path, content: &str) -> Option<Post> {
    let mut lines = content.lines();

    // Expect frontmatter: ---
    if lines.next() != Some("---") {
        return None;
    }

    let mut frontmatter = String::new();
    let mut html_content = String::new();
    let mut in_frontmatter = true;

    for line in lines {
        if in_frontmatter {
            if line == "---" {
                in_frontmatter = false;
                continue;
            }
            frontmatter.push_str(line);
            frontmatter.push('\n');
        } else {
            html_content.push_str(line);
            html_content.push('\n');
        }
    }

    // Parse frontmatter as YAML-like key: value
    let mut title = String::new();
    let mut date = String::new();
    let mut excerpt = String::new();

    for line in frontmatter.lines() {
        if let Some(value) = line.strip_prefix("title: ") {
            title = value.trim_matches('"').to_string();
        } else if let Some(value) = line.strip_prefix("date: ") {
            date = value.trim_matches('"').to_string();
        } else if let Some(value) = line.strip_prefix("excerpt: ") {
            excerpt = value.trim_matches('"').to_string();
        }
    }

    let slug = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string();

    let html = markdown_to_html(&html_content);

    Some(Post {
        title,
        slug,
        date,
        excerpt,
        html_content: html,
    })
}

fn markdown_to_html(markdown: &str) -> String {
    let mut html = String::new();
    let mut in_code_block = false;
    let mut code_content = String::new();

    for line in markdown.lines() {
        // Code block handling
        if line.starts_with("```") {
            if in_code_block {
                html.push_str("<pre><code>");
                html.push_str(&escape_html(&code_content));
                html.push_str("</code></pre>\n");
                code_content.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_content.push_str(line);
            code_content.push('\n');
            continue;
        }

        let trimmed = line.trim();

        // Headings
        if let Some(heading_content) = trimmed.strip_prefix("### ") {
            html.push_str("<h3>");
            html.push_str(&process_inline_markdown(heading_content));
            html.push_str("</h3>\n");
        } else if let Some(heading_content) = trimmed.strip_prefix("## ") {
            html.push_str("<h2>");
            html.push_str(&process_inline_markdown(heading_content));
            html.push_str("</h2>\n");
        } else if let Some(heading_content) = trimmed.strip_prefix("# ") {
            html.push_str("<h1>");
            html.push_str(&process_inline_markdown(heading_content));
            html.push_str("</h1>\n");
        }
        // Lists
        else if trimmed.starts_with("- ") {
            let item = trimmed.strip_prefix("- ").unwrap_or("");
            html.push_str("<li>");
            html.push_str(&process_inline_markdown(item));
            html.push_str("</li>\n");
        }
        // Paragraphs
        else if !trimmed.is_empty() {
            html.push_str("<p>");
            html.push_str(&process_inline_markdown(trimmed));
            html.push_str("</p>\n");
        }
    }

    html
}

fn process_inline_markdown(text: &str) -> String {
    let mut result = text.to_string();

    // Process in order: images, links, bold, italic
    result = parse_image(&result);
    result = parse_link(&result);
    result = parse_bold(&result);
    result = parse_italic(&result);

    result
}

fn parse_bold(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '*' && chars.peek() == Some(&'*') {
            chars.next(); // consume second *
            let mut bold_content = String::new();
            let mut found_closing = false;

            while let Some(inner_ch) = chars.next() {
                if inner_ch == '*' && chars.peek() == Some(&'*') {
                    chars.next(); // consume second *
                    found_closing = true;
                    break;
                }
                bold_content.push(inner_ch);
            }

            if found_closing {
                result.push_str("<strong>");
                result.push_str(&escape_html(&bold_content));
                result.push_str("</strong>");
            } else {
                result.push('*');
                result.push('*');
                result.push_str(&bold_content);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn parse_italic(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '*' && chars.peek() != Some(&'*') {
            let mut italic_content = String::new();
            let mut found_closing = false;

            while let Some(inner_ch) = chars.next() {
                if inner_ch == '*' && chars.peek() != Some(&'*') {
                    found_closing = true;
                    break;
                }
                italic_content.push(inner_ch);
            }

            if found_closing {
                result.push_str("<em>");
                result.push_str(&escape_html(&italic_content));
                result.push_str("</em>");
            } else {
                result.push('*');
                result.push_str(&italic_content);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn parse_link(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' {
            let mut link_text = String::new();
            let mut found_close_bracket = false;

            while let Some(inner_ch) = chars.next() {
                if inner_ch == ']' {
                    found_close_bracket = true;
                    break;
                }
                link_text.push(inner_ch);
            }

            if found_close_bracket && chars.peek() == Some(&'(') {
                chars.next(); // consume (
                let mut url = String::new();
                let mut found_close_paren = false;

                while let Some(url_ch) = chars.next() {
                    if url_ch == ')' {
                        found_close_paren = true;
                        break;
                    }
                    url.push(url_ch);
                }

                if found_close_paren {
                    result.push_str("<a href=\"");
                    result.push_str(&escape_html(&url));
                    result.push_str("\">");
                    result.push_str(&escape_html(&link_text));
                    result.push_str("</a>");
                } else {
                    result.push('[');
                    result.push_str(&link_text);
                    result.push(']');
                    result.push('(');
                    result.push_str(&url);
                }
            } else {
                result.push('[');
                result.push_str(&link_text);
                if found_close_bracket {
                    result.push(']');
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn parse_image(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '!' && chars.peek() == Some(&'[') {
            chars.next(); // consume [
            let mut alt_text = String::new();
            let mut found_close_bracket = false;

            while let Some(inner_ch) = chars.next() {
                if inner_ch == ']' {
                    found_close_bracket = true;
                    break;
                }
                alt_text.push(inner_ch);
            }

            if found_close_bracket && chars.peek() == Some(&'(') {
                chars.next(); // consume (
                let mut url = String::new();
                let mut found_close_paren = false;

                while let Some(url_ch) = chars.next() {
                    if url_ch == ')' {
                        found_close_paren = true;
                        break;
                    }
                    url.push(url_ch);
                }

                if found_close_paren {
                    result.push_str("<img src=\"");
                    result.push_str(&escape_html(&url));
                    result.push_str("\" alt=\"");
                    result.push_str(&escape_html(&alt_text));
                    result.push_str("\" />");
                } else {
                    result.push('!');
                    result.push('[');
                    result.push_str(&alt_text);
                    result.push(']');
                    result.push('(');
                    result.push_str(&url);
                }
            } else {
                result.push('!');
                result.push('[');
                result.push_str(&alt_text);
                if found_close_bracket {
                    result.push(']');
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn escape_html(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#39;")
}

fn generate_post_page(tera: &Tera, post: &Post) -> String {
    let mut context = Context::new();
    context.insert("title", &post.title);
    context.insert("date", &post.date);
    context.insert("content", &post.html_content);

    match tera.render("post.html", &context) {
        Ok(html) => html,
        Err(e) => {
            eprintln!("❌ Error rendering post template: {}", e);
            String::new()
        }
    }
}

fn generate_index_page(tera: &Tera, posts: &[Post]) -> String {
    let mut context = Context::new();
    let posts_data: Vec<_> = posts
        .iter()
        .map(|p| {
            serde_json::json!({
                "title": p.title,
                "slug": p.slug,
                "date": p.date,
                "excerpt": p.excerpt,
            })
        })
        .collect();

    context.insert("posts", &posts_data);

    match tera.render("index.html", &context) {
        Ok(html) => html,
        Err(e) => {
            eprintln!("❌ Error rendering index template: {}", e);
            String::new()
        }
    }
}
