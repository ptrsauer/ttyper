pub mod results;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::collections::HashMap;
use std::fmt;
use std::time::Instant;

pub struct TestEvent {
    pub time: Instant,
    pub key: KeyEvent,
    pub correct: Option<bool>,
    pub release_time: Option<Instant>,
}

pub fn is_missed_word_event(event: &TestEvent) -> bool {
    event.correct != Some(true)
}

impl fmt::Debug for TestEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestEvent")
            .field("time", &String::from("Instant { ... }"))
            .field("key", &self.key)
            .finish()
    }
}

#[derive(Debug)]
pub struct TestWord {
    pub text: String,
    pub progress: String,
    pub events: Vec<TestEvent>,
}

impl From<String> for TestWord {
    fn from(string: String) -> Self {
        TestWord {
            text: string,
            progress: String::new(),
            events: Vec::new(),
        }
    }
}

impl From<&str> for TestWord {
    fn from(string: &str) -> Self {
        Self::from(string.to_string())
    }
}

#[derive(Debug)]
pub struct Test {
    pub words: Vec<TestWord>,
    pub current_word: usize,
    pub complete: bool,
    pub backtracking_enabled: bool,
    pub sudden_death_enabled: bool,
    pub case_insensitive: bool,
    pending_presses: HashMap<KeyCode, (usize, usize)>,
}

impl Test {
    pub fn new(
        words: Vec<String>,
        backtracking_enabled: bool,
        sudden_death_enabled: bool,
        case_insensitive: bool,
    ) -> Self {
        Self {
            words: words.into_iter().map(TestWord::from).collect(),
            current_word: 0,
            complete: false,
            backtracking_enabled,
            sudden_death_enabled,
            case_insensitive,
            pending_presses: HashMap::new(),
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            self.record_release(key.code);
            return;
        }
        if key.kind != KeyEventKind::Press {
            return;
        }

        let word_idx = self.current_word;
        let events_before = self.words[word_idx].events.len();

        let word = &mut self.words[self.current_word];
        match key.code {
            KeyCode::Char(' ') | KeyCode::Enter
                if !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if word.text.chars().nth(word.progress.len()) == Some(' ') {
                    word.progress.push(' ');
                    word.events.push(TestEvent {
                        time: Instant::now(),
                        correct: Some(true),
                        key,
                        release_time: None,
                    })
                } else if !word.progress.is_empty() || word.text.is_empty() {
                    let correct = if self.case_insensitive {
                        word.text.to_lowercase() == word.progress.to_lowercase()
                    } else {
                        word.text == word.progress
                    };
                    if self.sudden_death_enabled && !correct {
                        self.reset();
                    } else {
                        word.events.push(TestEvent {
                            time: Instant::now(),
                            correct: Some(correct),
                            key,
                            release_time: None,
                        });
                        self.next_word();
                    }
                }
            }
            KeyCode::Backspace => {
                if word.progress.is_empty() && self.backtracking_enabled {
                    self.last_word();
                } else {
                    let is_error = if self.case_insensitive {
                        !word
                            .text
                            .to_lowercase()
                            .starts_with(&word.progress.to_lowercase())
                    } else {
                        !word.text.starts_with(&word.progress[..])
                    };
                    word.events.push(TestEvent {
                        time: Instant::now(),
                        correct: Some(is_error),
                        key,
                        release_time: None,
                    });
                    word.progress.pop();
                }
            }
            // CTRL-H → delete single character (same as Backspace)
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if word.progress.is_empty() && self.backtracking_enabled {
                    self.last_word();
                } else {
                    let is_error = if self.case_insensitive {
                        !word
                            .text
                            .to_lowercase()
                            .starts_with(&word.progress.to_lowercase())
                    } else {
                        !word.text.starts_with(&word.progress[..])
                    };
                    word.events.push(TestEvent {
                        time: Instant::now(),
                        correct: Some(is_error),
                        key,
                        release_time: None,
                    });
                    word.progress.pop();
                }
            }
            // CTRL-W and CTRL-Backspace → delete entire word
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.words[self.current_word].progress.is_empty() {
                    self.last_word();
                }

                let word = &mut self.words[self.current_word];

                word.events.push(TestEvent {
                    time: Instant::now(),
                    correct: None,
                    key,
                    release_time: None,
                });
                word.progress.clear();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let ch = if self.case_insensitive {
                    c.to_lowercase().next().unwrap_or(c)
                } else {
                    c
                };
                word.progress.push(ch);
                let correct = if self.case_insensitive {
                    word.text
                        .to_lowercase()
                        .starts_with(&word.progress.to_lowercase())
                } else {
                    word.text.starts_with(&word.progress[..])
                };
                if self.sudden_death_enabled && !correct {
                    self.reset();
                } else {
                    word.events.push(TestEvent {
                        time: Instant::now(),
                        correct: Some(correct),
                        key,
                        release_time: None,
                    });
                    let words_match = if self.case_insensitive {
                        word.progress.to_lowercase() == word.text.to_lowercase()
                    } else {
                        word.progress == word.text
                    };
                    if words_match && self.current_word == self.words.len() - 1 {
                        self.complete = true;
                        self.current_word = 0;
                    }
                }
            }
            _ => {}
        };

        // Track pending press for dwell time measurement (after match borrow is dropped)
        if self
            .words
            .get(word_idx)
            .is_some_and(|w| w.events.len() > events_before)
        {
            self.pending_presses
                .insert(key.code, (word_idx, self.words[word_idx].events.len() - 1));
        }
    }

    fn record_release(&mut self, code: KeyCode) {
        if let Some((word_idx, event_idx)) = self.pending_presses.remove(&code) {
            if let Some(word) = self.words.get_mut(word_idx) {
                if let Some(event) = word.events.get_mut(event_idx) {
                    event.release_time = Some(Instant::now());
                }
            }
        }
    }

    fn last_word(&mut self) {
        if self.current_word != 0 {
            self.current_word -= 1;
        }
    }

    fn next_word(&mut self) {
        if self.current_word == self.words.len() - 1 {
            self.complete = true;
            self.current_word = 0;
        } else {
            self.current_word += 1;
        }
    }

    fn reset(&mut self) {
        self.words.iter_mut().for_each(|word: &mut TestWord| {
            word.progress.clear();
            word.events.clear();
        });
        self.current_word = 0;
        self.complete = false;
        self.pending_presses.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    fn press_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    fn type_string(test: &mut Test, s: &str) {
        for c in s.chars() {
            test.handle_key(press(KeyCode::Char(c)));
        }
    }

    #[test]
    fn ctrl_h_deletes_single_character() {
        let mut test = Test::new(vec!["hello".to_string()], true, false, false);
        type_string(&mut test, "hel");
        assert_eq!(test.words[0].progress, "hel");

        test.handle_key(press_ctrl(KeyCode::Char('h')));
        assert_eq!(
            test.words[0].progress, "he",
            "Ctrl+H should delete only one character, not the entire word"
        );
    }

    #[test]
    fn ctrl_h_on_empty_word_backtracks() {
        let mut test = Test::new(
            vec!["ab".to_string(), "cd".to_string()],
            true, // backtracking enabled
            false,
            false,
        );
        // Complete word 1, move to word 2
        type_string(&mut test, "ab");
        test.handle_key(press(KeyCode::Char(' ')));
        assert_eq!(test.current_word, 1);

        // Ctrl+H on empty word 2 → should go back to word 1
        test.handle_key(press_ctrl(KeyCode::Char('h')));
        assert_eq!(
            test.current_word, 0,
            "Ctrl+H on empty word should backtrack to previous word"
        );
    }

    #[test]
    fn ctrl_h_no_backtrack_when_disabled() {
        let mut test = Test::new(
            vec!["ab".to_string(), "cd".to_string()],
            false, // backtracking disabled
            false,
            false,
        );
        type_string(&mut test, "ab");
        test.handle_key(press(KeyCode::Char(' ')));
        assert_eq!(test.current_word, 1);

        // Ctrl+H on empty word 2 with backtracking disabled → stay on word 2
        test.handle_key(press_ctrl(KeyCode::Char('h')));
        assert_eq!(
            test.current_word, 1,
            "Ctrl+H should not backtrack when backtracking is disabled"
        );
    }

    #[test]
    fn ctrl_letter_is_ignored() {
        let mut test = Test::new(vec!["hello".to_string()], true, false, false);
        type_string(&mut test, "he");
        assert_eq!(test.words[0].progress, "he");

        // Ctrl+A should not type 'a'
        test.handle_key(press_ctrl(KeyCode::Char('a')));
        assert_eq!(
            test.words[0].progress, "he",
            "Ctrl+A should not type a character"
        );

        // Ctrl+F should not type 'f'
        test.handle_key(press_ctrl(KeyCode::Char('f')));
        assert_eq!(
            test.words[0].progress, "he",
            "Ctrl+F should not type a character"
        );
    }

    #[test]
    fn ctrl_letter_no_event_added() {
        let mut test = Test::new(vec!["hello".to_string()], true, false, false);
        type_string(&mut test, "he");
        let events_before = test.words[0].events.len();

        test.handle_key(press_ctrl(KeyCode::Char('g')));
        assert_eq!(
            test.words[0].events.len(),
            events_before,
            "Ctrl+G should not add a TestEvent"
        );
    }

    #[test]
    fn shift_letter_still_types() {
        let mut test = Test::new(vec!["Hello".to_string()], true, false, false);

        let shift_h = KeyEvent {
            code: KeyCode::Char('H'),
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        test.handle_key(shift_h);
        assert_eq!(
            test.words[0].progress, "H",
            "Shift+letter should still type the uppercase character"
        );
    }

    #[test]
    fn ctrl_shift_letter_is_ignored() {
        let mut test = Test::new(vec!["hello".to_string()], true, false, false);
        type_string(&mut test, "he");

        let ctrl_shift_a = KeyEvent {
            code: KeyCode::Char('A'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        test.handle_key(ctrl_shift_a);
        assert_eq!(
            test.words[0].progress, "he",
            "Ctrl+Shift+A should not type a character"
        );
    }

    #[test]
    fn ctrl_space_does_not_advance_word() {
        let mut test = Test::new(vec!["ab".to_string(), "cd".to_string()], true, false, false);
        type_string(&mut test, "ab");
        assert_eq!(test.current_word, 0);

        // Ctrl+Space should NOT advance to next word
        test.handle_key(press_ctrl(KeyCode::Char(' ')));
        assert_eq!(
            test.current_word, 0,
            "Ctrl+Space should not advance to the next word"
        );
    }

    #[test]
    fn tab_does_not_affect_progress() {
        let mut test = Test::new(vec!["hello".to_string()], true, false, false);
        type_string(&mut test, "he");

        test.handle_key(press(KeyCode::Tab));
        assert_eq!(
            test.words[0].progress, "he",
            "Tab should not modify word progress (handled in main.rs, not handle_key)"
        );
        assert_eq!(test.current_word, 0);
    }

    #[test]
    fn ctrl_w_still_clears_entire_word() {
        let mut test = Test::new(vec!["hello".to_string()], true, false, false);
        type_string(&mut test, "hel");
        assert_eq!(test.words[0].progress, "hel");

        test.handle_key(press_ctrl(KeyCode::Char('w')));
        assert_eq!(
            test.words[0].progress, "",
            "Ctrl+W should clear the entire word progress"
        );
    }

    #[test]
    fn case_insensitive_lowercase_matches_uppercase_word() {
        let mut test = Test::new(vec!["Hello".to_string()], true, false, true);
        type_string(&mut test, "hello");
        assert_eq!(
            test.words[0].progress, "hello",
            "In case-insensitive mode, typed lowercase should be stored as-is"
        );
        // Complete the word
        test.handle_key(press(KeyCode::Char(' ')));
        assert!(
            test.complete,
            "Typing 'hello' for 'Hello' should complete in case-insensitive mode"
        );
    }

    #[test]
    fn case_insensitive_uppercase_matches_lowercase_word() {
        let mut test = Test::new(vec!["hello".to_string()], true, false, true);
        let shift_h = KeyEvent {
            code: KeyCode::Char('H'),
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        test.handle_key(shift_h);
        // In case-insensitive mode, uppercase 'H' should be lowercased to 'h'
        assert_eq!(
            test.words[0].progress, "h",
            "In case-insensitive mode, typed uppercase should be stored as lowercase"
        );
    }

    #[test]
    fn case_insensitive_correct_flag_on_events() {
        let mut test = Test::new(vec!["World".to_string()], true, false, true);
        type_string(&mut test, "world");
        // All events should be marked correct (case-insensitive comparison)
        assert!(
            test.words[0].events.iter().all(|e| e.correct == Some(true)),
            "All keystrokes should be marked correct in case-insensitive mode"
        );
    }

    #[test]
    fn case_sensitive_uppercase_mismatch() {
        let mut test = Test::new(vec!["Hello".to_string()], true, false, false);
        type_string(&mut test, "hello");
        test.handle_key(press(KeyCode::Char(' ')));
        // In case-sensitive mode, 'hello' != 'Hello', so the word event should be incorrect
        let last_event = test.words[0].events.last().unwrap();
        assert_eq!(
            last_event.correct,
            Some(false),
            "In case-sensitive mode, 'hello' should not match 'Hello'"
        );
    }

    #[test]
    fn case_insensitive_auto_complete_last_word() {
        let mut test = Test::new(vec!["ABC".to_string()], true, false, true);
        type_string(&mut test, "abc");
        assert!(
            test.complete,
            "Typing 'abc' for last word 'ABC' should auto-complete in case-insensitive mode"
        );
    }
}
