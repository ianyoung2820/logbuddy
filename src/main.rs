use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

fn main() {
    // Immutable variable
    let title = "LogBuddy (simple Rust version)";
    println!("=== {} ===", title);

    // Mutable variable
    let mut folder = String::new();

    print!("Enter a folder path to scan (e.g. ./logs or .): ");
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut folder)
        .expect("Failed to read input");

    let folder = folder.trim();
    if folder.is_empty() {
        println!("No folder provided. Exiting.");
        return;
    }

    let path = Path::new(folder);
    if !path.is_dir() {
        println!("'{}' is not a folder.", folder);
        return;
    }

    // HashMap for word counts (stretch: data structure)
    let mut word_counts: HashMap<String, usize> = HashMap::new();
    let mut files_scanned = 0usize;
    let mut total_lines = 0usize;

    // Loop over entries in the folder (non-recursive, simple)
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(e) => {
            println!("Could not read folder: {}", e);
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                println!("Skipping an entry: {}", e);
                continue;
            }
        };

        let file_path = entry.path();
        // Only handle regular files with a .txt or .log extension
        if !file_path.is_file() {
            continue;
        }

        let ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "txt" && ext != "log" {
            continue;
        }

        let contents = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                println!("Could not read {}: {}", file_path.display(), e);
                continue;
            }
        };

        files_scanned += 1;
        total_lines += contents.lines().count();

        // Function that borrows &str and &mut HashMap (references)
        count_words_in_text(&contents, &mut word_counts);
    }

    println!();
    println!("Scanned folder   : {}", folder);
    println!("Files processed  : {}", files_scanned);
    println!("Total lines read : {}", total_lines);

    if files_scanned == 0 {
        println!("No .txt or .log files found. Nothing to report.");
        return;
    }

    println!("\nTop words:");
    let mut pairs: Vec<(String, usize)> = word_counts
        .into_iter()
        .collect();

    // Sort by count descending
    pairs.sort_by(|a, b| b.1.cmp(&a.1));

    // Show up to 10 most common words
    for (i, (word, count)) in pairs.into_iter().take(10).enumerate() {
        println!("{:>2}. {:<20} {}", i + 1, word, count);
    }
}

/// Function that borrows a &str and &mut HashMap.
/// Demonstrates loops, conditionals, references, and expressions.
fn count_words_in_text(text: &str, counts: &mut HashMap<String, usize>) {
    for line in text.lines() {
        // split_whitespace is an expression that returns an iterator
        for word in line.split_whitespace() {
            let w = word
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();

            if w.is_empty() {
                continue;
            }

            // Conditional + mutation
            if let Some(count) = counts.get_mut(&w) {
                *count += 1;
            } else {
                counts.insert(w, 1);
            }
        }
    }
}
