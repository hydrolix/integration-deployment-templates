pub fn sanitize_project_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("project name is empty".to_string());
    }
    let mut out = String::with_capacity(trimmed.len());
    for c in trimmed.chars() {
        let mapped = if c == '-' || c.is_whitespace() {
            '_'
        } else if c.is_ascii_alphanumeric() || c == '_' {
            c.to_ascii_lowercase()
        } else {
            return Err(format!(
                "project name has invalid character '{}' (must be ASCII letters, digits, or underscores): {}",
                c, name
            ));
        };
        out.push(mapped);
    }
    match out.chars().next() {
        Some(c) if c.is_ascii_digit() => Err(format!(
            "project name must start with a letter, got '{}': {}",
            c, name
        )),
        _ => Ok(out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_through_clean_name() {
        assert_eq!(
            sanitize_project_name("mcdn_insights").unwrap(),
            "mcdn_insights"
        );
    }

    #[test]
    fn hyphens_become_underscores() {
        assert_eq!(
            sanitize_project_name("mcdn-insights").unwrap(),
            "mcdn_insights"
        );
    }

    #[test]
    fn uppercase_normalized_to_lowercase() {
        assert_eq!(
            sanitize_project_name("MCDN_Insights").unwrap(),
            "mcdn_insights"
        );
    }

    #[test]
    fn internal_whitespace_becomes_underscore() {
        assert_eq!(
            sanitize_project_name("mcdn insights").unwrap(),
            "mcdn_insights"
        );
    }

    #[test]
    fn leading_and_trailing_whitespace_stripped() {
        assert_eq!(
            sanitize_project_name("  mcdn_insights  ").unwrap(),
            "mcdn_insights"
        );
    }

    #[test]
    fn leading_digit_rejected() {
        let err = sanitize_project_name("1mcdn").unwrap_err();
        assert!(
            err.to_lowercase().contains("letter"),
            "error should mention leading-letter requirement, got: {err}"
        );
    }

    #[test]
    fn empty_input_rejected() {
        let err = sanitize_project_name("").unwrap_err();
        assert!(
            err.to_lowercase().contains("empty"),
            "error should mention empty, got: {err}"
        );
    }

    #[test]
    fn whitespace_only_input_rejected() {
        let err = sanitize_project_name("   ").unwrap_err();
        assert!(
            err.to_lowercase().contains("empty"),
            "error should mention empty, got: {err}"
        );
    }

    #[test]
    fn non_ascii_rejected() {
        let err = sanitize_project_name("mcdn_insîghts").unwrap_err();
        let lower = err.to_lowercase();
        assert!(
            lower.contains("ascii") || lower.contains("invalid character"),
            "error should mention invalid/ascii, got: {err}"
        );
    }

    #[test]
    fn punctuation_rejected() {
        let err = sanitize_project_name("mcdn_insights!").unwrap_err();
        let lower = err.to_lowercase();
        assert!(
            lower.contains("invalid character") || lower.contains("ascii"),
            "error should mention invalid character, got: {err}"
        );
    }
}
