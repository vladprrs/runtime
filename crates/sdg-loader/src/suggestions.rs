use strsim::normalized_damerau_levenshtein;

/// Find the closest match from candidates, returning a formatted suggestion string.
/// Returns None if no candidate is similar enough (>0.6 threshold).
/// Example: suggest_similar("Creatd", &["Created", "InProgress"]) -> Some(". Did you mean 'Created'?")
pub fn suggest_similar(input: &str, candidates: &[&str]) -> Option<String> {
    // TODO: implement
    let _ = (input, candidates, normalized_damerau_levenshtein);
    None
}

/// Format suggestion or return empty string for error message interpolation.
pub fn suggestion_or_empty(input: &str, candidates: &[&str]) -> String {
    suggest_similar(input, candidates).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_close_match() {
        let result = suggest_similar("InProgres", &["Created", "InProgress", "Done"]);
        assert!(result.is_some(), "should suggest for close match");
        let suggestion = result.unwrap();
        assert!(
            suggestion.contains("InProgress"),
            "suggestion should contain 'InProgress', got: {suggestion}"
        );
    }

    #[test]
    fn test_suggest_no_match() {
        let result = suggest_similar("xyz", &["Created", "InProgress", "Done"]);
        assert!(result.is_none(), "should not suggest for very dissimilar input");
    }

    #[test]
    fn test_suggest_exact_match() {
        let result = suggest_similar("Created", &["Created", "InProgress"]);
        assert!(result.is_some(), "exact match should suggest (similarity = 1.0)");
        let suggestion = result.unwrap();
        assert!(
            suggestion.contains("Created"),
            "suggestion should contain 'Created', got: {suggestion}"
        );
    }

    #[test]
    fn test_suggest_case_variation() {
        let result = suggest_similar("inprogress", &["InProgress"]);
        assert!(
            result.is_some(),
            "case variation should still suggest (normalized_damerau_levenshtein gives >0.6)"
        );
    }
}
