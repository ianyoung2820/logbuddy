use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Command-line configuration.
/// Example:
///   cargo run -- --path ./logs --ext .log --top 10 --find error
#[derive(Debug)]
struct Config {
    path: PathBuf,
    ext: Option<String>,
    top: usize,
    find: Option<String>,
}

impl Config {
    fn from_args() -> Result<Self, String> {
        let mut path: Option<PathBuf> = None;
        let mut ext: Option<String> = None;
        let mut top: usize = 10;
        let mut find: Option<String> = None;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--path" => {
                    let value = args.next().ok_or("Missing value for --path")?;
                    path = Some(PathBuf::from(value));
                }
                "--ext" => {
                    let value = args.next().ok_or("Missing value for --ext")?;
                    ext = Some(value);
                }
                "--top" => {
                    let value = args.next().ok_or("Missing value for --top")?;
                    top = value
                        .parse::<usize>()
                        .map_err(|_| "Invalid number for --top".to_string())?;
                }
                "--find" => {
                    let value = args.next().ok_or("Missing value for --find")?;
                    find = Some(value);
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => {
                    return Err(format!("Unknown flag: {other}"));
                }
            }
        }

        let path = path.ok_or("You must provide --path <folder>")?;
        Ok(Self { path, ext, top, find })
    }
}

fn print_usage() {
    eprintln!(
        "LogBuddy – tiny Rust log scanner

Usage:
  logbuddy --path <folder> [--ext .log] [--top 10] [--find word]

Options:
  --path   Folder to scan (required)
  --ext    Only include files with this extension (e.g. .log, .txt)
  --top    Show top-N most frequent words (default 10)
  --find   Search for a word/phrase (case-insensitive)
  -h, --help   Show this help
"
    );
}

/// Totals gathered while scanning.
#[derive(Default)]
struct ScanTotals {
    files_scanned: usize,
    total_lines: usize,
    total_bytes: u64,
    hits: usize,
    word_counts: HashMap<String, usize>,
}

/// Main scanner type – keeps config and running totals together.
struct Scanner {
    cfg: Config,
    totals: ScanTotals,
}

impl Scanner {
    fn new(cfg: Config) -> Self {
        Self {
            cfg,
            totals: ScanTotals::default(),
        }
    }

    fn run(&mut self) -> io::Result<()> {
        // Clone the root path so we don't immutably borrow self while also
        // using &mut self inside walk_dir (avoids E0502).
        let root = self.cfg.path.clone();
        self.walk_dir(&root)?;
        self.print_summary();
        Ok(())
    }

    fn walk_dir(&mut self, dir: &Path) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // recursion into subfolders
                self.walk_dir(&path)?;
                continue;
            }

            // If an extension is specified, filter by it.
            if let Some(ref want_ext) = self.cfg.ext {
                let want = want_ext.trim_start_matches('.');
                let actual = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if actual != want {
                    continue;
                }
            }

            self.process_file(&path)?;
        }
        Ok(())
    }

    fn process_file(&mut self, path: &Path) -> io::Result<()> {
        let content = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Could not read {}: {e}", path.display());
                return Ok(());
            }
        };

        let bytes = content.as_bytes().len() as u64;
        let line_count = content.lines().count();

        // Optional search term
        if let Some(ref needle) = self.cfg.find {
            let needle_lower = needle.to_lowercase();
            let mut local_hits = 0usize;

            for (line_no, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&needle_lower) {
                    local_hits += 1;
                    // Print first few hits for context
                    if local_hits <= 5 {
                        println!(
                            "[HIT] {}:{}: {}",
                            path.display(),
                            line_no + 1,
                            trim_preview(line, 120)
                        );
                    }
                }
            }

            self.totals.hits += local_hits;
        }

        // Tokenize and count words
        for word in tokenize_words(&content) {
            *self.totals.word_counts.entry(word).or_insert(0) += 1;
        }

        self.totals.files_scanned += 1;
        self.totals.total_lines += line_count;
        self.totals.total_bytes += bytes;

        Ok(())
    }

    fn print_summary(&self) {
        println!();
        println!("=== LogBuddy Summary ===");
        println!("Path        : {}", self.cfg.path.display());
        if let Some(ref ext) = self.cfg.ext {
            println!("Extension   : {}", ext);
        }
        if let Some(ref f) = self.cfg.find {
            println!("Search term : {}", f);
            println!("Total hits  : {}", self.totals.hits);
        }
        println!("Files       : {}", self.totals.files_scanned);
        println!("Lines       : {}", self.totals.total_lines);
        println!("Bytes       : {}", self.totals.total_bytes);

        println!("\nTop {} words:", self.cfg.top);
        let mut pairs: Vec<(&String, &usize)> = self.totals.word_counts.iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1)); // highest counts first

        for (i, (word, count)) in pairs.into_iter().take(self.cfg.top).enumerate() {
            println!("{:>2}. {:<20} {}", i + 1, word, count);
        }
    }
}

/// Split text into lowercase "words", demonstrating slicing and Vec.
///
/// We walk over the underlying bytes and use slice indices (start..end)
/// to grab &str slices from the original string, then convert to String.
/// This shows borrowing (&str) plus moving to owned String.
fn tokenize_words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        // Skip non-word chars.
        while i < bytes.len() && !is_word_char(bytes[i]) {
            i += 1;
        }
        let start = i;

        // Consume word chars.
        while i < bytes.len() && is_word_char(bytes[i]) {
            i += 1;
        }
        let end = i;

        if end > start {
            // Slice the original string (this is where slicing happens).
            let slice = &text[start..end];
            out.push(slice.to_lowercase());
        }
    }

    out
}

fn is_word_char(b: u8) -> bool {
    (b'A'..=b'Z').contains(&b)
        || (b'a'..=b'z').contains(&b)
        || (b'0'..=b'9').contains(&b)
        || b == b'_'
}

/// Trim a long line for search preview.
fn trim_preview(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut out = s[..max].to_string();
        out.push('…');
        out
    }
}

fn main() {
    let cfg = match Config::from_args() {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("Argument error: {msg}\n");
            print_usage();
            std::process::exit(2);
        }
    };

    let mut scanner = Scanner::new(cfg);
    if let Err(e) = scanner.run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
