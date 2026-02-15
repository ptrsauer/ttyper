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
/// All other settings match [`default_test`]: backtracking on, no sudden death,
/// backspace allowed, no look-ahead limit.
pub fn case_insensitive_test(words: Vec<String>) -> Test {
    Test::new(words, true, false, true, false, None)
}

/// Create a test with backspace/delete disabled (Backspace, Ctrl+H, Ctrl+W all blocked).
/// All other settings match [`default_test`]: backtracking on, no sudden death,
/// case-sensitive, no look-ahead limit.
pub fn no_backspace_test(words: Vec<String>) -> Test {
    Test::new(words, true, false, false, true, None)
}

/// Create a test with backtracking between words disabled.
/// All other settings match [`default_test`]: no sudden death, case-sensitive,
/// backspace allowed, no look-ahead limit.
pub fn no_backtrack_test(words: Vec<String>) -> Test {
    Test::new(words, false, false, false, false, None)
}

/// Create a test with look-ahead limiting (only the next `n` upcoming words visible).
/// Uses `Some(n)` internally — for no limit, use [`default_test`] instead.
/// All other settings match [`default_test`].
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
    fn no_backtrack_test_has_correct_settings() {
        let test = no_backtrack_test(vec!["hello".to_string()]);
        assert!(!test.backtracking_enabled);
        assert!(!test.sudden_death_enabled);
        assert!(!test.case_insensitive);
        assert!(!test.no_backspace);
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
