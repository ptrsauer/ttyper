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

pub struct DwellData {
    pub per_key: Vec<(char, f64)>,
    pub overall_avg_ms: Option<f64>,
    pub has_data: bool,
}

pub struct Results {
    pub timing: TimingData,
    pub accuracy: AccuracyData,
    pub dwell: DwellData,
    pub missed_words: Vec<String>,
    pub slow_words: Vec<String>,
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
            dwell: calc_dwell(&events),
            missed_words: calc_missed_words(test),
            slow_words: calc_slow_words(test),
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

/// Returns the 5 slowest correctly-typed words, sorted slowest first.
/// Speed is measured as time-per-character (duration / word length).
/// Words with errors (missed words) are excluded.
fn calc_slow_words(test: &Test) -> Vec<String> {
    let mut word_speeds: Vec<(&str, f64)> = test
        .words
        .iter()
        .filter(|word| {
            // Exclude missed words (those with any incorrect event)
            !word.events.iter().any(is_missed_word_event)
        })
        .filter_map(|word| {
            // Need at least 2 events to measure timing
            if word.events.len() < 2 || word.text.is_empty() {
                return None;
            }
            let first = word.events.first().unwrap().time;
            let last = word.events.last().unwrap().time;
            let duration = last.checked_duration_since(first)?;
            let time_per_char = duration.as_secs_f64() / word.text.len() as f64;
            Some((word.text.as_str(), time_per_char))
        })
        .collect();

    // Sort by time_per_char descending (slowest first)
    word_speeds.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    word_speeds
        .into_iter()
        .take(5)
        .map(|(text, _)| text.to_string())
        .collect()
}

/// Calculate keystroke dwelling (key-hold) time statistics.
/// Only includes events where a Release event was captured (auto-detect).
fn calc_dwell(events: &[&super::TestEvent]) -> DwellData {
    let mut key_dwells: HashMap<char, Vec<f64>> = HashMap::new();
    let mut all_dwells: Vec<f64> = Vec::new();

    for event in events {
        if let (Some(release_time), KeyCode::Char(c)) = (event.release_time, event.key.code) {
            if let Some(dwell) = release_time.checked_duration_since(event.time) {
                let dwell_ms = dwell.as_secs_f64() * 1000.0;
                key_dwells.entry(c).or_default().push(dwell_ms);
                all_dwells.push(dwell_ms);
            }
        }
    }

    let has_data = !all_dwells.is_empty();
    let overall_avg_ms = if has_data {
        Some(all_dwells.iter().sum::<f64>() / all_dwells.len() as f64)
    } else {
        None
    };

    let mut per_key: Vec<(char, f64)> = key_dwells
        .into_iter()
        .map(|(c, dwells)| (c, dwells.iter().sum::<f64>() / dwells.len() as f64))
        .collect();
    per_key.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    DwellData {
        per_key,
        overall_avg_ms,
        has_data,
    }
}

#[cfg(test)]
mod tests {
    use super::super::helpers::default_test;
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::time::Instant;

    fn make_event(c: char, correct: bool) -> super::super::TestEvent {
        super::super::TestEvent {
            time: Instant::now(),
            key: KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            correct: Some(correct),
            release_time: None,
        }
    }

    fn key_for(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn non_target_key_excluded_from_per_key() {
        let mut test = default_test(vec!["abc".to_string()]);
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
        let mut test = default_test(vec!["ab".to_string()]);
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
        let mut test = default_test(vec!["aa".to_string()]);
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
        let mut test = default_test(vec!["hello".to_string()]);
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
        let mut test = default_test(vec!["a".to_string()]);
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

    fn make_timed_event(c: char, correct: bool, time: Instant) -> super::super::TestEvent {
        super::super::TestEvent {
            time,
            key: KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            correct: Some(correct),
            release_time: None,
        }
    }

    fn make_dwell_event(
        c: char,
        correct: bool,
        time: Instant,
        release_time: Instant,
    ) -> super::super::TestEvent {
        super::super::TestEvent {
            time,
            key: KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
            correct: Some(correct),
            release_time: Some(release_time),
        }
    }

    #[test]
    fn slow_words_identifies_slowest() {
        let now = Instant::now();
        let mut test = default_test(vec![
            "fast".to_string(),
            "slow".to_string(),
            "mid".to_string(),
        ]);

        // "fast" — 4 chars in 0.4s = 0.1s/char
        for (i, c) in "fast".chars().enumerate() {
            test.words[0].events.push(make_timed_event(
                c,
                true,
                now + std::time::Duration::from_millis(i as u64 * 100),
            ));
        }

        // "slow" — 4 chars in 2.0s = 0.5s/char
        for (i, c) in "slow".chars().enumerate() {
            test.words[1].events.push(make_timed_event(
                c,
                true,
                now + std::time::Duration::from_millis(i as u64 * 500),
            ));
        }

        // "mid" — 3 chars in 0.6s = 0.2s/char
        for (i, c) in "mid".chars().enumerate() {
            test.words[2].events.push(make_timed_event(
                c,
                true,
                now + std::time::Duration::from_millis(i as u64 * 200),
            ));
        }

        let slow = calc_slow_words(&test);
        assert_eq!(slow[0], "slow", "Slowest word should be first");
        assert_eq!(slow[1], "mid", "Second slowest should be second");
        assert_eq!(slow[2], "fast", "Fastest should be last");
    }

    #[test]
    fn slow_words_excludes_missed() {
        let now = Instant::now();
        let mut test = default_test(vec!["correct".to_string(), "wrong".to_string()]);

        // "correct" — typed correctly
        for (i, c) in "correct".chars().enumerate() {
            test.words[0].events.push(make_timed_event(
                c,
                true,
                now + std::time::Duration::from_millis(i as u64 * 100),
            ));
        }

        // "wrong" — has an error event (should be excluded)
        test.words[1].events.push(make_timed_event('w', true, now));
        test.words[1].events.push(make_timed_event(
            'x',
            false,
            now + std::time::Duration::from_millis(500),
        ));

        let slow = calc_slow_words(&test);
        assert_eq!(
            slow.len(),
            1,
            "Only correctly-typed words should be included"
        );
        assert_eq!(slow[0], "correct");
    }

    #[test]
    fn slow_words_skips_single_event_words() {
        let now = Instant::now();
        let mut test = default_test(vec!["a".to_string(), "hello".to_string()]);

        // "a" — only 1 event (can't measure timing)
        test.words[0].events.push(make_timed_event('a', true, now));

        // "hello" — 5 events
        for (i, c) in "hello".chars().enumerate() {
            test.words[1].events.push(make_timed_event(
                c,
                true,
                now + std::time::Duration::from_millis(i as u64 * 100),
            ));
        }

        let slow = calc_slow_words(&test);
        assert_eq!(slow.len(), 1, "Words with <2 events should be skipped");
        assert_eq!(slow[0], "hello");
    }

    #[test]
    fn slow_words_caps_at_five() {
        let now = Instant::now();
        let words: Vec<String> = (0..10).map(|i| format!("word{}", i)).collect();
        let mut test = default_test(words);

        for (wi, word) in test.words.iter_mut().enumerate() {
            for (ci, c) in word.text.clone().chars().enumerate() {
                word.events.push(make_timed_event(
                    c,
                    true,
                    now + std::time::Duration::from_millis((wi as u64 * 100) + (ci as u64 * 50)),
                ));
            }
        }

        let slow = calc_slow_words(&test);
        assert_eq!(slow.len(), 5, "Should return at most 5 slow words");
    }

    #[test]
    fn results_preserve_word_list() {
        let words = vec!["hello".to_string(), "world".to_string(), "test".to_string()];
        let test = default_test(words.clone());

        let results = Results::from(&test);

        assert_eq!(
            results.words, words,
            "Results should preserve the original word list"
        );
    }

    #[test]
    fn results_preserve_word_order() {
        let words = vec![
            "zebra".to_string(),
            "apple".to_string(),
            "mango".to_string(),
        ];
        let test = default_test(words.clone());

        let results = Results::from(&test);

        assert_eq!(
            results.words[0], "zebra",
            "Word order should be preserved exactly"
        );
        assert_eq!(results.words[1], "apple");
        assert_eq!(results.words[2], "mango");
    }

    // --- Dwell time ---

    #[test]
    fn dwell_no_release_events() {
        let mut test = default_test(vec!["abc".to_string()]);
        test.words[0].events.push(make_event('a', true));
        test.words[0].events.push(make_event('b', true));
        test.words[0].events.push(make_event('c', true));

        let results = Results::from(&test);
        assert!(
            !results.dwell.has_data,
            "No release events → has_data should be false"
        );
        assert!(results.dwell.overall_avg_ms.is_none());
        assert!(results.dwell.per_key.is_empty());
    }

    #[test]
    fn dwell_with_release_events() {
        let now = Instant::now();
        let mut test = default_test(vec!["ab".to_string()]);

        // 'a' held for 80ms, 'b' held for 120ms
        test.words[0].events.push(make_dwell_event(
            'a',
            true,
            now,
            now + std::time::Duration::from_millis(80),
        ));
        test.words[0].events.push(make_dwell_event(
            'b',
            true,
            now + std::time::Duration::from_millis(100),
            now + std::time::Duration::from_millis(220),
        ));

        let results = Results::from(&test);
        assert!(results.dwell.has_data);
        // avg = (80 + 120) / 2 = 100ms
        let avg = results.dwell.overall_avg_ms.unwrap();
        assert!((avg - 100.0).abs() < 1.0, "Expected ~100ms, got {}", avg);
        // 'b' should be first (longer dwell)
        assert_eq!(results.dwell.per_key[0].0, 'b');
        assert_eq!(results.dwell.per_key[1].0, 'a');
    }

    #[test]
    fn dwell_mixed_events() {
        let now = Instant::now();
        let mut test = default_test(vec!["abc".to_string()]);

        // 'a' has release (100ms), 'b' does not, 'c' has release (50ms)
        test.words[0].events.push(make_dwell_event(
            'a',
            true,
            now,
            now + std::time::Duration::from_millis(100),
        ));
        test.words[0].events.push(make_timed_event(
            'b',
            true,
            now + std::time::Duration::from_millis(150),
        ));
        test.words[0].events.push(make_dwell_event(
            'c',
            true,
            now + std::time::Duration::from_millis(200),
            now + std::time::Duration::from_millis(250),
        ));

        let results = Results::from(&test);
        assert!(results.dwell.has_data);
        // Only 'a' and 'c' have dwell data
        assert_eq!(results.dwell.per_key.len(), 2);
        // avg = (100 + 50) / 2 = 75ms
        let avg = results.dwell.overall_avg_ms.unwrap();
        assert!((avg - 75.0).abs() < 1.0);
    }

    #[test]
    fn dwell_per_key_averages() {
        let now = Instant::now();
        let mut test = default_test(vec!["aa".to_string()]);

        // Two presses of 'a': 60ms and 100ms → avg 80ms
        test.words[0].events.push(make_dwell_event(
            'a',
            true,
            now,
            now + std::time::Duration::from_millis(60),
        ));
        test.words[0].events.push(make_dwell_event(
            'a',
            true,
            now + std::time::Duration::from_millis(100),
            now + std::time::Duration::from_millis(200),
        ));

        let results = Results::from(&test);
        assert_eq!(results.dwell.per_key.len(), 1);
        let (ch, avg_ms) = results.dwell.per_key[0];
        assert_eq!(ch, 'a');
        assert!(
            (avg_ms - 80.0).abs() < 1.0,
            "Expected ~80ms, got {}",
            avg_ms
        );
    }
}
