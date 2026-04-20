#[cfg(test)]
mod tests {
    use kg_engine::ingestion_registry::documents::pdf::cleaner::{Cleaner, CleanerConfig};

    use super::*;

    fn cleaner() -> Cleaner {
        Cleaner::new(CleanerConfig::default())
    }

    #[test]
    fn test_basic_cleaning(){
        let input = "Rust is fast. \n\nIt is memory-safe.";
        let output = cleaner().clean(input);
        assert_eq!(output, "Rust is fast. It is memory-safe.");
    }

    #[test]
    fn test_whitespace_normalization(){
        let input = "Rust      is       fast.";
        let output = cleaner().clean(input);

        assert_eq!(output, "Rust is fast.");
    }

    #[test]
    fn test_page_number_removal() {
        let input = "Rust is great.\nPage 3\nIt is safe";
        let output = cleaner().clean(input);

        assert_eq!(output, "Rust is great. It is safe");
    }

    #[test]
    fn test_numeric_page_removal() {
        let input = "Rust is great.\n3\nIt is safe.";
        let output = cleaner().clean(input);

        assert_eq!(output, "Rust is great. It is safe.");
    }

    #[test]
    fn test_line_joining() {
        let input = "Rust is fast\nand memory-safe.";
        let output = cleaner().clean(input);

        assert_eq!(output, "Rust is fast and memory-safe.");
    }
}