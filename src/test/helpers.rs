/// Test helper functions to reduce Test::new() boilerplate.
///
/// Most tests use identical default parameters. These helpers provide
/// self-documenting, concise constructors for common configurations.
use super::Test;

/// Create a test with default configuration:
/// backtracking enabled, no sudden death, case-sensitive, backspace allowed, no look-ahead limit.
pub fn default_test(words: Vec<String>) -> Test {
    Test::new(words, true, false, false, false, None)
}

/// Create a test with case-insensitive comparison enabled.
/// All other settings use defaults (backtracking on, no sudden death, backspace allowed).
pub fn case_insensitive_test(words: Vec<String>) -> Test {
    Test::new(words, true, false, true, false, None)
}

/// Create a test with backspace/delete disabled.
/// All other settings use defaults (backtracking on, no sudden death, case-sensitive).
pub fn no_backspace_test(words: Vec<String>) -> Test {
    Test::new(words, true, false, false, true, None)
}

/// Create a test with look-ahead limiting (only N upcoming words visible).
/// All other settings use defaults.
pub fn look_ahead_test(words: Vec<String>, n: usize) -> Test {
    Test::new(words, true, false, false, false, Some(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_test_has_correct_settings() {
        let test = default_test(vec!["hello".to_string()]);
        assert!(test.backtracking_enabled);
        assert!(!test.sudden_death_enabled);
        assert!(!test.case_insensitive);
        assert!(!test.no_backspace);
        assert_eq!(test.look_ahead, None);
    }

    #[test]
    fn case_insensitive_test_has_correct_settings() {
        let test = case_insensitive_test(vec!["hello".to_string()]);
        assert!(test.backtracking_enabled);
        assert!(!test.sudden_death_enabled);
        assert!(test.case_insensitive);
        assert!(!test.no_backspace);
        assert_eq!(test.look_ahead, None);
    }

    #[test]
    fn no_backspace_test_has_correct_settings() {
        let test = no_backspace_test(vec!["hello".to_string()]);
        assert!(test.backtracking_enabled);
        assert!(!test.sudden_death_enabled);
        assert!(!test.case_insensitive);
        assert!(test.no_backspace);
        assert_eq!(test.look_ahead, None);
    }

    #[test]
    fn look_ahead_test_has_correct_settings() {
        let test = look_ahead_test(vec!["a".to_string(), "b".to_string()], 1);
        assert!(test.backtracking_enabled);
        assert!(!test.sudden_death_enabled);
        assert!(!test.case_insensitive);
        assert!(!test.no_backspace);
        assert_eq!(test.look_ahead, Some(1));
    }
}
