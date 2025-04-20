
pub fn get_file_icon(file_name: &str) -> &str {
    let extension = file_name
        .split('.')
        .last()
        .unwrap_or("")
        .trim_end_matches('/');
    match extension.to_lowercase().as_str() {
        // Programming
        "rs" => "󱘗", // Rust
        "c" => "", // C
        "cpp" | "cc" | "cxx" => "", // C++
        "py" => "", // Python
        "java" | "class" => "", // Java
        "js" | "jsx" => "", // JavaScript
        "json" => "", // JSON
        "html" => "", // HTML
        "css" => "", // CSS
        "go" => "", // Go
        "php" => "", // PHP
        "rb" => "", // Ruby
        "swift" => "", // Swift
        "ts" | "tsx" => "", // TypeScript
        "sh" | "bash" => "", // Shell
        "lua" => "", // Lua
        "r" => "", // R
        "dart" => "", // Dart
        "kotlin" | "kt" => "", // Kotlin
        "scala" => "", // Scala
        "elixir" | "ex" | "exs" => "", // Elixir
        "hs" => "", // Haskell
        "clj" | "cljs" | "cljc" | "edn" | "cljr" => "", // Clojure
        "erl" => "", // Erlang
        "ml" | "mli" => "", // OCaml
        "sql" => "", // SQL
        "m" => "", // Matlab
        "cs" => "", // C#
        "pl" => "", // Perl
        "asm" | "s" => "", // Assembly
        "ps1" => "", // PowerShell
        "groovy" => "", // Groovy
        "jl" => "", // Julia
        "fs" | "fsx" | "fsi" => "", // F#
        "lisp" | "lsp" => "", // Lisp
        "f" | "for" | "f90" => "󱈚", // Fortran
        "ada" => "", // Ada


        // Config Files
        "yaml" | "yml" | "ini" | "config" | "babelrc" => "", // YAML
        "toml" => "", // TOML
        "lock" => "", // Lock
        "xml" => "", // XML
        "env" => "󰒋", // Env
        "dockerfile" => "󰡨", // Dockerfile
        "makefile" => "", // Makefile



        // Media
        "mp3" | "wav" | "flac" => "", // Audio
        "mp4" | "mkv" | "avi" => "", // Video
        "jpg" | "jpeg" | "png" | "gif" => "", // Image
        "svg" => "", // SVG

        // Documents
        "txt" => {
            if file_name.to_lowercase() == "cmakelists.txt" {
                "" // CMake
            } else {
                "" // Text
            }
        },
        "md" => "", // Markdown
        "pdf" => "", // PDF
        "doc" | "docx" => "", // Word

        // Archives
        "zip" | "rar" | "7z" | "tar" | "gz" => "", // Zip

        // Git
        "git" => "", // Git
        "github" => "",
        "gitignore" => "", // Git Ignore

        _ => if file_name.ends_with("/") {"󰉋"} else {"󰈙"},
    }
}