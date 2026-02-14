pub mod results;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::fmt;
use std::time::Instant;

pub struct TestEvent {
    pub time: Instant,
    pub key: KeyEvent,
    pub correct: Option<bool>,
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
}

impl Test {
    pub fn new(words: Vec<String>, backtracking_enabled: bool, sudden_death_enabled: bool) -> Self {
        Self {
            words: words.into_iter().map(TestWord::from).collect(),
            current_word: 0,
            complete: false,
            backtracking_enabled,
            sudden_death_enabled,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

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
                    })
                } else if !word.progress.is_empty() || word.text.is_empty() {
                    let correct = word.text == word.progress;
                    if self.sudden_death_enabled && !correct {
                        self.reset();
                    } else {
                        word.events.push(TestEvent {
                            time: Instant::now(),
                            correct: Some(correct),
                            key,
                        });
                        self.next_word();
                    }
                }
            }
            KeyCode::Backspace => {
                if word.progress.is_empty() && self.backtracking_enabled {
                    self.last_word();
                } else {
                    word.events.push(TestEvent {
                        time: Instant::now(),
                        correct: Some(!word.text.starts_with(&word.progress[..])),
                        key,
                    });
                    word.progress.pop();
                }
            }
            // CTRL-H → delete single character (same as Backspace)
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if word.progress.is_empty() && self.backtracking_enabled {
                    self.last_word();
                } else {
                    word.events.push(TestEvent {
                        time: Instant::now(),
                        correct: Some(!word.text.starts_with(&word.progress[..])),
                        key,
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
                });
                word.progress.clear();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                word.progress.push(c);
                let correct = word.text.starts_with(&word.progress[..]);
                if self.sudden_death_enabled && !correct {
                    self.reset();
                } else {
                    word.events.push(TestEvent {
                        time: Instant::now(),
                        correct: Some(correct),
                        key,
                    });
                    if word.progress == word.text && self.current_word == self.words.len() - 1 {
                        self.complete = true;
                        self.current_word = 0;
                    }
                }
            }
            _ => {}
        };
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
        let mut test = Test::new(vec!["hello".to_string()], true, false);
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
        let mut test = Test::new(vec!["hello".to_string()], true, false);
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
        let mut test = Test::new(vec!["hello".to_string()], true, false);
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
        let mut test = Test::new(vec!["Hello".to_string()], true, false);

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
        let mut test = Test::new(vec!["hello".to_string()], true, false);
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
        let mut test = Test::new(
            vec!["ab".to_string(), "cd".to_string()],
            true,
            false,
        );
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
    fn ctrl_w_still_clears_entire_word() {
        let mut test = Test::new(vec!["hello".to_string()], true, false);
        type_string(&mut test, "hel");
        assert_eq!(test.words[0].progress, "hel");

        test.handle_key(press_ctrl(KeyCode::Char('w')));
        assert_eq!(
            test.words[0].progress, "",
            "Ctrl+W should clear the entire word progress"
        );
    }
}
