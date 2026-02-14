use super::{is_missed_word_event, Test};

use crossterm::event::{KeyCode, KeyEvent};
use std::collections::{HashMap, HashSet};
use std::{cmp, fmt};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fraction {
    pub numerator: usize,
    pub denominator: usize,
}

impl Fraction {
    pub const fn new(numerator: usize, denominator: usize) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

impl From<Fraction> for f64 {
    fn from(f: Fraction) -> Self {
        f.numerator as f64 / f.denominator as f64
    }
}

impl cmp::Ord for Fraction {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        f64::from(*self).partial_cmp(&f64::from(*other)).unwrap()
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Fraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

pub struct TimingData {
    // Instead of storing WPM, we store CPS (clicks per second)
    pub overall_cps: f64,
    pub per_event: Vec<f64>,
    pub per_key: HashMap<KeyEvent, f64>,
}

pub struct AccuracyData {
    pub overall: Fraction,
    pub per_key: HashMap<KeyEvent, Fraction>,
}

pub struct Results {
    pub timing: TimingData,
    pub accuracy: AccuracyData,
    pub missed_words: Vec<String>,
    pub words: Vec<String>,
}

impl From<&Test> for Results {
    fn from(test: &Test) -> Self {
        let events: Vec<&super::TestEvent> =
            test.words.iter().flat_map(|w| w.events.iter()).collect();

        let target_chars: HashSet<char> = test
            .words
            .iter()
            .flat_map(|w| w.text.chars())
            .flat_map(|c| [c.to_ascii_lowercase(), c.to_ascii_uppercase()])
            .collect();

        Self {
            timing: calc_timing(&events),
            accuracy: calc_accuracy(&events, &target_chars),
            missed_words: calc_missed_words(test),
            words: test.words.iter().map(|w| w.text.clone()).collect(),
        }
    }
}

fn calc_timing(events: &[&super::TestEvent]) -> TimingData {
    let mut timing = TimingData {
        overall_cps: -1.0,
        per_event: Vec::new(),
        per_key: HashMap::new(),
    };

    // map of keys to a two-tuple (total time, clicks) for counting average
    let mut keys: HashMap<KeyEvent, (f64, usize)> = HashMap::new();

    for win in events.windows(2) {
        let event_dur = win[1]
            .time
            .checked_duration_since(win[0].time)
            .map(|d| d.as_secs_f64());

        if let Some(event_dur) = event_dur {
            timing.per_event.push(event_dur);

            let key = keys.entry(win[1].key).or_insert((0.0, 0));
            key.0 += event_dur;
            key.1 += 1;
        }
    }

    timing.per_key = keys
        .into_iter()
        .map(|(key, (total, count))| (key, total / count as f64))
        .collect();

    timing.overall_cps = timing.per_event.len() as f64 / timing.per_event.iter().sum::<f64>();

    timing
}

fn calc_accuracy(events: &[&super::TestEvent], target_chars: &HashSet<char>) -> AccuracyData {
    let mut acc = AccuracyData {
        overall: Fraction::new(0, 0),
        per_key: HashMap::new(),
    };

    events
        .iter()
        .filter(|event| event.correct.is_some())
        .for_each(|event| {
            acc.overall.denominator += 1;
            if event.correct.unwrap() {
                acc.overall.numerator += 1;
            }

            // Only track per-key accuracy for characters that appear in the target text.
            // Keys not in the target (e.g. typing 'x' when only 'abc' are expected) would
            // always show 0% accuracy, which is misleading.
            let in_target = match event.key.code {
                KeyCode::Char(c) => target_chars.contains(&c),
                _ => true,
            };

            if in_target {
                let key = acc
                    .per_key
                    .entry(event.key)
                    .or_insert_with(|| Fraction::new(0, 0));

                key.denominator += 1;
                if event.correct.unwrap() {
                    key.numerator += 1;
                }
            }
        });

    acc
}

fn calc_missed_words(test: &Test) -> Vec<String> {
    test.words
        .iter()
        .filter(|word| word.events.iter().any(is_missed_word_event))
        .map(|word| word.text.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::time::Instant;

    fn make_event(c: char, correct: bool) -> super::super::TestEvent {
        super::super::TestEvent {
            time: Instant::now(),
            key: KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            correct: Some(correct),
        }
    }

    fn key_for(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn non_target_key_excluded_from_per_key() {
        let mut test = Test::new(vec!["abc".to_string()], true, false);
        test.words[0].events.push(make_event('a', true));
        test.words[0].events.push(make_event('x', false)); // 'x' not in "abc"
        test.words[0].events.push(make_event('b', true));
        test.words[0].events.push(make_event('c', true));

        let results = Results::from(&test);

        // 'x' should NOT appear in per_key
        assert!(
            !results.accuracy.per_key.contains_key(&key_for('x')),
            "Non-target key 'x' should not be in per_key accuracy"
        );

        // Target keys should be present
        assert!(results.accuracy.per_key.contains_key(&key_for('a')));
        assert!(results.accuracy.per_key.contains_key(&key_for('b')));
        assert!(results.accuracy.per_key.contains_key(&key_for('c')));
    }

    #[test]
    fn non_target_key_still_counted_in_overall() {
        let mut test = Test::new(vec!["ab".to_string()], true, false);
        test.words[0].events.push(make_event('a', true));
        test.words[0].events.push(make_event('x', false)); // wrong key, not in target
        test.words[0].events.push(make_event('b', true));

        let results = Results::from(&test);

        // Overall: 2 correct (a, b) out of 3 total (a, x, b)
        assert_eq!(results.accuracy.overall.numerator, 2);
        assert_eq!(results.accuracy.overall.denominator, 3);
    }

    #[test]
    fn target_key_with_errors_tracked_correctly() {
        let mut test = Test::new(vec!["aa".to_string()], true, false);
        test.words[0].events.push(make_event('a', true));
        test.words[0].events.push(make_event('a', false)); // 'a' is in target but typed wrong position

        let results = Results::from(&test);

        let a_acc = results.accuracy.per_key.get(&key_for('a')).unwrap();
        assert_eq!(a_acc.numerator, 1);
        assert_eq!(a_acc.denominator, 2);
    }

    #[test]
    fn shift_variant_of_target_key_tracked() {
        // Target has lowercase 'e', user types uppercase 'E' (Shift mistake)
        let mut test = Test::new(vec!["hello".to_string()], true, false);
        test.words[0].events.push(make_event('h', true));
        test.words[0].events.push(make_event('E', false)); // Shift-variant of 'e'
        test.words[0].events.push(make_event('l', true));

        let results = Results::from(&test);

        // 'E' should be tracked because 'e' is in the target (case-insensitive)
        assert!(
            results.accuracy.per_key.contains_key(&key_for('E')),
            "Shift-variant 'E' of target key 'e' should be tracked in per_key"
        );
    }

    #[test]
    fn multiple_non_target_keys_all_excluded() {
        let mut test = Test::new(vec!["a".to_string()], true, false);
        test.words[0].events.push(make_event('a', true));
        test.words[0].events.push(make_event('x', false));
        test.words[0].events.push(make_event('y', false));
        test.words[0].events.push(make_event('z', false));

        let results = Results::from(&test);

        assert!(!results.accuracy.per_key.contains_key(&key_for('x')));
        assert!(!results.accuracy.per_key.contains_key(&key_for('y')));
        assert!(!results.accuracy.per_key.contains_key(&key_for('z')));
        assert!(results.accuracy.per_key.contains_key(&key_for('a')));

        // Overall: 1 correct out of 4
        assert_eq!(results.accuracy.overall.numerator, 1);
        assert_eq!(results.accuracy.overall.denominator, 4);
    }

    #[test]
    fn results_preserve_word_list() {
        let words = vec!["hello".to_string(), "world".to_string(), "test".to_string()];
        let test = Test::new(words.clone(), true, false);

        let results = Results::from(&test);

        assert_eq!(results.words, words, "Results should preserve the original word list");
    }

    #[test]
    fn results_preserve_word_order() {
        let words = vec!["zebra".to_string(), "apple".to_string(), "mango".to_string()];
        let test = Test::new(words.clone(), true, false);

        let results = Results::from(&test);

        assert_eq!(
            results.words[0], "zebra",
            "Word order should be preserved exactly"
        );
        assert_eq!(results.words[1], "apple");
        assert_eq!(results.words[2], "mango");
    }
}
