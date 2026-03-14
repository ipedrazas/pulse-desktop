use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticChunk {
    pub id: i64,
    pub file_path: String,
    pub chunk_type: String,
    pub name: Option<String>,
    pub content: String,
    pub language: Option<String>,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
    pub keywords: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSearchResult {
    pub chunk: SemanticChunk,
    pub score: f64,
}

fn extract_keywords(content: &str) -> String {
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "need", "dare", "ought",
        "used", "to", "of", "in", "for", "on", "with", "at", "by", "from",
        "as", "into", "through", "during", "before", "after", "above", "below",
        "between", "out", "off", "over", "under", "again", "further", "then",
        "once", "and", "but", "or", "nor", "not", "so", "yet", "both",
        "either", "neither", "each", "every", "all", "any", "few", "more",
        "most", "other", "some", "such", "no", "only", "own", "same", "than",
        "too", "very", "just", "because", "if", "else", "return", "let",
        "const", "var", "fn", "func", "def", "class", "struct", "pub",
        "self", "this", "new", "import", "export", "use", "mod", "type",
    ]
    .into_iter()
    .collect();

    let mut word_counts: HashMap<String, usize> = HashMap::new();

    for word in content.split(|c: char| !c.is_alphanumeric() && c != '_') {
        let lower = word.to_lowercase();
        if lower.len() > 2 && !stop_words.contains(lower.as_str()) {
            *word_counts.entry(lower).or_insert(0) += 1;
        }
    }

    let mut words: Vec<(String, usize)> = word_counts.into_iter().collect();
    words.sort_by(|a, b| b.1.cmp(&a.1));
    words.truncate(20);

    words.iter().map(|(w, _)| w.as_str()).collect::<Vec<_>>().join(" ")
}

fn detect_language(path: &str) -> Option<String> {
    let ext = Path::new(path).extension()?.to_str()?;
    Some(
        match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            "vue" => "vue",
            "rb" => "ruby",
            "cpp" | "cc" | "cxx" => "cpp",
            "c" | "h" => "c",
            "cs" => "csharp",
            "swift" => "swift",
            "kt" | "kts" => "kotlin",
            _ => return None,
        }
        .to_string(),
    )
}

fn chunk_file(path: &str, content: &str) -> Vec<(String, Option<String>, String, Option<i64>, Option<i64>)> {
    let mut chunks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() <= 50 {
        chunks.push((
            "file".to_string(),
            Some(path.rsplit('/').next().unwrap_or(path).to_string()),
            content.to_string(),
            Some(1),
            Some(lines.len() as i64),
        ));
        return chunks;
    }

    // Split into ~40 line chunks with overlap
    let chunk_size = 40;
    let overlap = 5;
    let mut start = 0;

    while start < lines.len() {
        let end = (start + chunk_size).min(lines.len());
        let chunk_content = lines[start..end].join("\n");
        chunks.push((
            "block".to_string(),
            None,
            chunk_content,
            Some((start + 1) as i64),
            Some(end as i64),
        ));
        if end >= lines.len() {
            break;
        }
        start += chunk_size - overlap;
    }

    chunks
}

#[tauri::command]
pub async fn index_project_semantics(
    pool: State<'_, SqlitePool>,
    project_id: String,
    root_path: String,
) -> Result<usize, String> {
    let root = Path::new(&root_path);

    // Clear old chunks
    sqlx::query("DELETE FROM semantic_chunks WHERE project_id = ?")
        .bind(&project_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .max_depth(Some(10))
        .build();

    let extensions = [
        "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "vue",
        "rb", "cpp", "cc", "c", "h", "cs", "swift", "kt", "kts",
    ];

    let mut indexed = 0;

    let commit_sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    for entry in walker.flatten() {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }

        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !extensions.contains(&ext) {
            continue;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) if c.len() < 500_000 => c,
            _ => continue,
        };

        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        let language = detect_language(&rel_path);
        let chunks = chunk_file(&rel_path, &content);

        for (chunk_type, name, chunk_content, start_line, end_line) in chunks {
            let keywords = extract_keywords(&chunk_content);

            sqlx::query(
                "INSERT INTO semantic_chunks (project_id, file_path, chunk_type, name, content, language, start_line, end_line, commit_sha, keywords)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&project_id)
            .bind(&rel_path)
            .bind(&chunk_type)
            .bind(&name)
            .bind(&chunk_content)
            .bind(&language)
            .bind(start_line)
            .bind(end_line)
            .bind(&commit_sha)
            .bind(&keywords)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

            indexed += 1;
        }
    }

    Ok(indexed)
}

#[tauri::command]
pub async fn semantic_search(
    pool: State<'_, SqlitePool>,
    project_id: String,
    query: String,
) -> Result<Vec<SemanticSearchResult>, String> {
    let query_keywords = extract_keywords(&query);
    let query_terms: Vec<&str> = query_keywords.split_whitespace().collect();

    if query_terms.is_empty() {
        return Ok(vec![]);
    }

    // Build LIKE conditions for keyword matching
    let conditions: Vec<String> = query_terms
        .iter()
        .map(|t| format!("(keywords LIKE '%{}%' OR content LIKE '%{}%')", t, t))
        .collect();

    let where_clause = conditions.join(" OR ");
    let sql = format!(
        "SELECT id, file_path, chunk_type, name, content, language, start_line, end_line, keywords
         FROM semantic_chunks WHERE project_id = ? AND ({}) LIMIT 50",
        where_clause
    );

    let rows: Vec<(i64, String, String, Option<String>, String, Option<String>, Option<i64>, Option<i64>, Option<String>)> =
        sqlx::query_as(&sql)
            .bind(&project_id)
            .fetch_all(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

    let mut results: Vec<SemanticSearchResult> = rows
        .into_iter()
        .map(|(id, file_path, chunk_type, name, content, language, start_line, end_line, keywords)| {
            let kw = keywords.as_deref().unwrap_or("");
            let score = query_terms.iter().filter(|t| kw.contains(*t) || content.to_lowercase().contains(*t)).count() as f64
                / query_terms.len().max(1) as f64;

            SemanticSearchResult {
                chunk: SemanticChunk {
                    id,
                    file_path,
                    chunk_type,
                    name,
                    content,
                    language,
                    start_line,
                    end_line,
                    keywords: Some(kw.to_string()),
                },
                score,
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(20);
    Ok(results)
}
