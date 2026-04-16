#![allow(dead_code)]
// src/processing/cleaner.rs

/**
 * This file responsible of cleaning texts/pages from naunance. 
 * It is modular so developer using this will be able to modify it directly on their code base. 
 * 
 * 
 */

 #[derive(Debug, Clone)]
pub struct CleanerConfig {
    pub normalize_whitespaces: bool,
    pub fix_line_breaks: bool,
    pub remove_page_numbers: bool,
    pub remove_bullets: bool
}

impl Default for CleanerConfig {
    fn default() -> Self {
        Self {
            normalize_whitespaces: true,
            fix_line_breaks: true,
            remove_page_numbers: true,
            remove_bullets: true,
        }
    }
}

pub struct Cleaner {
    config: CleanerConfig,
}

impl Cleaner {
    pub fn new(config: CleanerConfig) -> Self {
        Self { config }
    }

    pub fn clean(&self, input: &str) -> String {
        let mut text = input.to_string();

        if self.config.fix_line_breaks {
            text = self.fix_line_breaks(&text);
        }

        if self.config.remove_page_numbers {
            text = self.remove_page_numbers(&text);
        }

        if self.config.normalize_whitespaces {
            text = self.normalize_whitespaces(&text);
        }

        if self.config.remove_bullets {
            text = self.remove_bulletpoints(&text);
        }
        text
    }
    /**
     * Fix broken pdf line breaks:
     * - Joins lines that should not be split.
     * - Preserves paragraphs boundaries.
     * 
     */
    fn fix_line_breaks(&self, input: &str) -> String {
        let mut result = String::new();

        let mut lines = input.lines().peekable();

        while let Some(line) = lines.next() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                result.push_str("\n\n"); // preserve paragraph
                continue;
            }

            result.push_str(trimmed);

            if let Some(next_line) = lines.peek() {
                let next_trimmed = next_line.trim();

                // if next line starts lowercase -> likely same sentence
                if next_trimmed.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
                    result.push(' ');
                }else{
                    result.push(' ');
                }
            }
        }

        result
    }

    /**
     * Remove simple page numbers like: 
     * - Page 3, 3, - 3 -
     */
    fn remove_page_numbers(&self, input: &str) -> String {
        input
            .split_whitespace()
            .filter(|word| {
                let lower = word.to_lowercase();
    
                // remove "page"
                if lower == "page" {
                    return false;
                }
    
                // remove pure numbers
                if word.chars().all(|c| c.is_numeric()) {
                    return false;
                }
    
                true
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /**
     * collapse multiple spaces/newlines into clean spacing
     * 
     */
    fn normalize_whitespaces(&self, input: &str) -> String {
        input.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
    }

    /***
     * Remove bullet points and replace them with something cheaper.
     */
    fn remove_bulletpoints(&self, input: &str) -> String {
        // println!("input: {}", input.lines());
        let heavy_bullets: &[char] = &['*', '+', '•', '●', '▪', '◦', '‣'];

        input.lines()
        .map(|line| {
            line.trim()
                .trim_start_matches(heavy_bullets)
                .trim_start_matches('-')
                .trim()
        }) 
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
    }
}