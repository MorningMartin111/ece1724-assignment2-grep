use colored::Colorize;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
struct Config {
    pattern: String,
    files: Vec<String>,
    case_insensitive: bool,
    line_numbers: bool,
    invert_match: bool,
    recursive_directory: bool,
    print_filenames: bool,
    colored_output: bool,
}

fn print_help() {
    println!(
        "Usage: grep [OPTIONS] <pattern> <files...>

Options:
-i                Case-insensitive search
-n                Print line numbers
-v                Invert match (exclude lines that match the pattern)
-r                Recursive directory search
-f                Print filenames
-c                Enable colored output
-h, --help        Show help information"
    );
}

fn parse_arguments() -> Result<Config, ()> {

    let mut arguments = env::args().skip(1);

    let mut user_config = Config {
        pattern: String::new(),
        files: Vec::new(),
        case_insensitive: false,
        line_numbers: false,
        invert_match: false,
        recursive_directory: false,
        print_filenames: false,
        colored_output: false,
    };

    let mut found_search_pattern = false;

    while let Some(current_argument) = arguments.next() {
        if current_argument == "-h" || current_argument == "--help" {
            print_help();
            return Err(());
        }

        if current_argument == "-i" {
            user_config.case_insensitive = true;
            continue;
        }
        if current_argument == "-n" {
            user_config.line_numbers = true;
            continue;
        }
        if current_argument == "-v" {
            user_config.invert_match = true;
            continue;
        }
        if current_argument == "-r" {
            user_config.recursive_directory = true;
            continue;
        }
        if current_argument == "-f" {
            user_config.print_filenames = true;
            continue;
        }
        if current_argument == "-c" {
            user_config.colored_output = true;
            continue;
        }

        if !found_search_pattern {
            user_config.pattern = current_argument;
            found_search_pattern = true;
        } else {
            user_config.files.push(current_argument);
        }
    }

    if !found_search_pattern {
        print_help();
        return Err(());
    }

    if user_config.files.is_empty() {
        print_help();
        return Err(());
    }

    Ok(user_config)
}

// Collect the list of files
// Process the file and directory paths entered by the user
fn collect_files(input_paths: &[String], is_recursive_search: bool) -> Vec<PathBuf> {
    let mut file_list = Vec::new();

    for user_input_path in input_paths {
        let path = Path::new(user_input_path);
        if path.is_file() {
            if !is_junk_file(path) {
                file_list.push(path.to_path_buf());
            }
        }
        else if path.is_dir() {
            if is_recursive_search {
                for directory_entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
                    let file_path = directory_entry.path();
                    if file_path.is_file() && !is_junk_file(file_path) {
                        file_list.push(file_path.to_path_buf());
                    }
                }
            } else {

            }
        } else {
            if path.exists() && path.is_file() && !is_junk_file(path) {
                file_list.push(path.to_path_buf());
            }
        }
    }
    // file_list.sort_by(|a, b| b.cmp(a));
    file_list
}


// Filter some common "junk files"
fn is_junk_file(file_path: &Path) -> bool {
    // Get the file name. If it is successfully obtained and can be converted into a string, check whether it is a junk file.
    if let Some(file_name) = file_path.file_name() {
        if let Some(file_name_str) = file_name.to_str() {
            if file_name_str.starts_with("._") {
                return true;
            }
            if file_name_str == ".DS_Store" {
                return true;
            }
        }
    }
    false
}

// Search for matching lines in a single file and print the results
fn search_file(file_path: &Path, config: &Config) -> io::Result<()> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut current_line_number: usize = 0;

    for line_result in reader.lines() {
        current_line_number += 1;

        let line_content = line_result?;

        let matches_found = find_matches_in_line(&line_content, &config.pattern, config.case_insensitive);

        let should_print_line = if config.invert_match {
            // Print this line only if no match is found
            matches_found.is_empty()
        } else {
            // If a match is found, print the line
            !matches_found.is_empty()
        };

        if should_print_line {
            // taking color output options into account
            let text_to_print = if config.colored_output && !matches_found.is_empty() {
                // -c Add red highlight to matching text
                colorize_hits(&line_content, &matches_found)
            } else {
                line_content.clone()
            };

            if config.print_filenames && config.line_numbers {
                // -f + -n Display file name and line number
                println!("{}: {}: {}", file_path.display(), current_line_number, text_to_print);
            } else if config.print_filenames {
                // -f file name
                println!("{}: {}", file_path.display(), text_to_print);
            } else if config.line_numbers {
                // -n line number
                println!("{}: {}", current_line_number, text_to_print);
            } else {
                // print text content
                println!("{}", text_to_print);
            }
        }
    }
    Ok(())
}

// Find all matches of a pattern in a line of text
// Returns a vector of (start, end) byte positions for each match found
fn find_matches_in_line(line_text: &str, search_pattern: &str, ignore_case: bool) -> Vec<(usize, usize)> {
    let mut match_positions = Vec::new();
    if search_pattern.is_empty() {
        return match_positions;
    }

    // convert to lowercase if ignore_case is true
    let (search_text, pattern_to_find) = if ignore_case {
        (line_text.to_ascii_lowercase(), search_pattern.to_ascii_lowercase())
    } else {
        (line_text.to_string(), search_pattern.to_string())
    };

    let text_bytes = search_text.as_bytes();
    let pattern_bytes = pattern_to_find.as_bytes();

    // pattern cannot be empty or longer than text
    if pattern_bytes.is_empty() || pattern_bytes.len() > text_bytes.len() {
        return match_positions;
    }

    // Search through the text from left to right
    let mut current_position = 0;
    let pattern_length = pattern_bytes.len();

    while current_position + pattern_length <= text_bytes.len() {
        // Compare current slice of text with the pattern
        let current_slice = &text_bytes[current_position..current_position + pattern_length];

        if current_slice == pattern_bytes {
            // Found a match! Record the start and end positions
            let match_start = current_position;
            let match_end = current_position + pattern_length;
            match_positions.push((match_start, match_end));
            current_position += pattern_length;
        } else {
            // No match found, move to next position
            current_position += 1;
        }
    }

    match_positions
}

// Add red color to matched text segments
fn colorize_hits(original_line: &str, match_ranges: &[(usize, usize)]) -> String {
    if match_ranges.is_empty() {
        return original_line.to_string();
    }

    // Create a new string to build the colored result
    // Pre-allocate enough space for efficiency
    let mut colored_result = String::with_capacity(original_line.len() + match_ranges.len() * 10);
    let mut last_processed_position = 0;

    for &(match_start, match_end) in match_ranges {
        // Add the normal text before the current match
        if match_start > last_processed_position {
            let normal_text_before_match = &original_line[last_processed_position..match_start];
            colored_result.push_str(normal_text_before_match);
        }

        // Add red color
        let matched_text_segment = &original_line[match_start..match_end];
        let red_colored_text = matched_text_segment.red().to_string();
        colored_result.push_str(&red_colored_text);
        last_processed_position = match_end;
    }

    // Add remaining text after the last match
    if last_processed_position < original_line.len() {
        let remaining_text = &original_line[last_processed_position..];
        colored_result.push_str(remaining_text);
    }
    colored_result
}


// ===== Main function: Coordinate the entire program =====
fn main() {
    let config = match parse_arguments() {
        Ok(config) => config,
        Err(_) => {
            return;
        }
    };

    // Convert user-provided paths into actual file list to search
    let files_to_search = collect_files(&config.files, config.recursive_directory);

    // Search each file
    // If a file can't be read, skip it
    for file_path in files_to_search {
        let _ = search_file(&file_path, &config);
    }
}