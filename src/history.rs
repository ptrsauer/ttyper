use crate::test::results::{Fraction, Results};

use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

pub const WPM_PER_CPS: f64 = 12.0;
const CSV_HEADER: &str =
    "datetime,language,words,wpm_raw,wpm_adjusted,accuracy,correct,total,worst_keys,missed_words";

/// Calculate raw and adjusted WPM from characters per second and accuracy (0.0–1.0).
pub fn calculate_wpms(cps: f64, accuracy: f64) -> (f64, f64) {
    let raw = cps * WPM_PER_CPS;
    let adjusted = raw * accuracy;
    (raw, adjusted)
}

/// Format worst keys from per-key accuracy data.
/// Returns semicolon-separated string of up to 5 worst keys, sorted by accuracy ascending.
/// Keys at 100% accuracy are excluded. Format: "y:50%;A:75%;c:81%"
pub fn format_worst_keys(per_key: &HashMap<KeyEvent, Fraction>) -> String {
    let mut worst_keys: Vec<_> = per_key
        .iter()
        .filter(|(key, _)| matches!(key.code, KeyCode::Char(_)))
        .map(|(key, frac)| {
            let ch = if let KeyCode::Char(c) = key.code {
                c
            } else {
                '?'
            };
            (ch, f64::from(*frac) * 100.0)
        })
        .filter(|(_, acc)| *acc < 100.0)
        .collect();
    worst_keys.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    worst_keys
        .iter()
        .take(5)
        .map(|(ch, acc)| format!("{}:{:.0}%", ch, acc))
        .collect::<Vec<_>>()
        .join(";")
}

/// Format a single CSV data line. Timestamp is passed in to keep the function pure/testable.
pub fn format_csv_line(
    timestamp: &str,
    language: &str,
    words: usize,
    results: &Results,
) -> String {
    let accuracy = f64::from(results.accuracy.overall);
    let (raw_wpm, adjusted_wpm) = calculate_wpms(results.timing.overall_cps, accuracy);
    let worst_str = format_worst_keys(&results.accuracy.per_key);
    let missed_str = results.missed_words.join(";");

    format!(
        "{},{},{},{:.1},{:.1},{:.1},{},{},{},{}",
        timestamp,
        language,
        words,
        raw_wpm,
        adjusted_wpm,
        accuracy * 100.0,
        results.accuracy.overall.numerator,
        results.accuracy.overall.denominator,
        worst_str,
        missed_str,
    )
}

/// Save results to history CSV file. Creates header if file is new, appends data line.
pub fn save_results(history_file: &Path, language: &str, words: usize, results: &Results) {
    let is_new = !history_file.exists();

    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_file)
    {
        if is_new {
            let _ = writeln!(file, "{}", CSV_HEADER);
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let line = format_csv_line(&timestamp, language, words, results);
        let _ = writeln!(file, "{}", line);
    }
}

/// Format history data rows into displayable lines.
/// `last` limits output to the most recent N entries. None means all.
fn format_history_rows(data_lines: &[&str], last: Option<usize>) -> Vec<String> {
    let skip = match last {
        Some(n) if n < data_lines.len() => data_lines.len() - n,
        _ => 0,
    };

    data_lines
        .iter()
        .skip(skip)
        .filter_map(|line| {
            let fields: Vec<&str> = line.splitn(10, ',').collect();
            if fields.len() >= 9 {
                Some(format!(
                    "{:<20} {:<15} {:>5} {:>8} {:>8} {:>8} {}",
                    fields[0], fields[1], fields[2], fields[3], fields[4], fields[5], fields[8]
                ))
            } else {
                None
            }
        })
        .collect()
}

/// Display history from CSV file in a formatted table.
/// `last` limits output to the most recent N entries. None means show all.
pub fn show_history(history_file: &Path, last: Option<usize>) {
    if !history_file.exists() {
        println!("No history found at {}", history_file.display());
        return;
    }

    let content = fs::read_to_string(history_file).expect("Failed to read history file");
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() <= 1 {
        println!("No results recorded yet.");
        return;
    }

    let data_lines = &lines[1..];
    let rows = format_history_rows(data_lines, last);
    let shown = rows.len();
    let total = data_lines.len();

    println!(
        "{:<20} {:<15} {:>5} {:>8} {:>8} {:>8} {}",
        "Date", "Language", "Words", "Raw WPM", "Adj WPM", "Acc %", "Worst Keys"
    );
    println!("{}", "-".repeat(90));

    for row in &rows {
        println!("{}", row);
    }

    if last.is_some() {
        println!(
            "\nShowing last {} of {} results. History file: {}",
            shown, total, history_file.display()
        );
    } else {
        println!(
            "\n{} results total. History file: {}",
            total, history_file.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::results::{AccuracyData, TimingData};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::collections::HashMap;

    fn make_key_event(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn make_results(
        cps: f64,
        correct: usize,
        total: usize,
        per_key: Vec<(char, usize, usize)>,
        missed: Vec<&str>,
    ) -> Results {
        let mut key_accuracy = HashMap::new();
        let mut key_timing = HashMap::new();
        for (ch, num, den) in per_key {
            let ke = make_key_event(ch);
            key_accuracy.insert(ke, Fraction::new(num, den));
            key_timing.insert(ke, 0.1);
        }

        Results {
            timing: TimingData {
                overall_cps: cps,
                per_event: vec![],
                per_key: key_timing,
            },
            accuracy: AccuracyData {
                overall: Fraction::new(correct, total),
                per_key: key_accuracy,
            },
            missed_words: missed.into_iter().map(String::from).collect(),
            slow_words: vec![],
            words: vec![],
        }
    }

    // --- WPM calculation ---

    #[test]
    fn test_calculate_wpms() {
        let (raw, adjusted) = calculate_wpms(5.0, 0.95);
        assert!((raw - 60.0).abs() < 0.001);
        assert!((adjusted - 57.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_wpms_perfect_accuracy() {
        let (raw, adjusted) = calculate_wpms(7.0, 1.0);
        assert!((raw - 84.0).abs() < 0.001);
        assert!((raw - adjusted).abs() < 0.001);
    }

    // --- Worst keys formatting ---

    #[test]
    fn test_format_worst_keys_sorted_by_accuracy() {
        let mut per_key = HashMap::new();
        per_key.insert(make_key_event('a'), Fraction::new(9, 10)); // 90%
        per_key.insert(make_key_event('b'), Fraction::new(1, 2)); // 50%
        per_key.insert(make_key_event('c'), Fraction::new(3, 4)); // 75%

        let result = format_worst_keys(&per_key);
        assert_eq!(result, "b:50%;c:75%;a:90%");
    }

    #[test]
    fn test_format_worst_keys_max_five() {
        let mut per_key = HashMap::new();
        for (i, ch) in "abcdefgh".chars().enumerate() {
            per_key.insert(make_key_event(ch), Fraction::new(i, 10));
        }

        let result = format_worst_keys(&per_key);
        assert_eq!(result.split(';').count(), 5);
    }

    #[test]
    fn test_format_worst_keys_all_perfect() {
        let mut per_key = HashMap::new();
        per_key.insert(make_key_event('a'), Fraction::new(10, 10));
        per_key.insert(make_key_event('b'), Fraction::new(5, 5));

        let result = format_worst_keys(&per_key);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_worst_keys_preserves_case() {
        let mut per_key = HashMap::new();
        per_key.insert(make_key_event('A'), Fraction::new(1, 4)); // 25%
        per_key.insert(make_key_event('a'), Fraction::new(9, 10)); // 90%

        let result = format_worst_keys(&per_key);
        assert!(result.contains("A:25%"));
        assert!(result.contains("a:90%"));
    }

    // --- CSV line formatting ---

    #[test]
    fn test_format_csv_line_field_order() {
        let results = make_results(
            6.5,
            380,
            400,
            vec![('y', 1, 2)],
            vec!["Architektur", "Frontend"],
        );

        let line = format_csv_line("2026-02-14 12:43:34", "peter1000", 50, &results);
        let fields: Vec<&str> = line.splitn(10, ',').collect();

        assert_eq!(fields.len(), 10);
        assert_eq!(fields[0], "2026-02-14 12:43:34");
        assert_eq!(fields[1], "peter1000");
        assert_eq!(fields[2], "50");
        assert_eq!(fields[6], "380");
        assert_eq!(fields[7], "400");
        assert_eq!(fields[9], "Architektur;Frontend");
    }

    #[test]
    fn test_format_csv_line_wpm_values() {
        let results = make_results(6.5, 380, 400, vec![], vec![]);

        let line = format_csv_line("2026-02-14 12:00:00", "test", 50, &results);
        let fields: Vec<&str> = line.splitn(10, ',').collect();

        assert_eq!(fields[3], "78.0"); // 6.5 * 12 = 78.0
        assert_eq!(fields[4], "74.1"); // 78.0 * 0.95 = 74.1
        assert_eq!(fields[5], "95.0"); // 380/400 = 95%
    }

    #[test]
    fn test_format_csv_line_empty_missed_words() {
        let results = make_results(5.0, 100, 100, vec![], vec![]);

        let line = format_csv_line("2026-02-14 12:00:00", "test", 50, &results);
        let fields: Vec<&str> = line.splitn(10, ',').collect();

        assert_eq!(fields[9], "");
    }

    // --- File I/O integration ---

    #[test]
    fn test_save_creates_header_for_new_file() {
        let dir = std::env::temp_dir().join("ttyper_test_header");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("history.csv");

        let results = make_results(5.0, 100, 100, vec![], vec![]);
        save_results(&file, "test", 50, &results);

        let content = fs::read_to_string(&file).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines[0], CSV_HEADER);
        assert_eq!(lines.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_appends_without_duplicate_header() {
        let dir = std::env::temp_dir().join("ttyper_test_append");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("history.csv");

        let results = make_results(5.0, 100, 100, vec![], vec![]);
        save_results(&file, "test", 50, &results);
        save_results(&file, "test", 50, &results);

        let content = fs::read_to_string(&file).unwrap();
        let header_count = content
            .lines()
            .filter(|l| l.starts_with("datetime,"))
            .count();
        assert_eq!(header_count, 1);
        assert_eq!(content.lines().count(), 3);

        let _ = fs::remove_dir_all(&dir);
    }

    // --- History display limiting ---

    fn sample_csv_lines() -> Vec<&'static str> {
        vec![
            "2026-02-10 10:00:00,english,50,72.0,68.4,95.0,190,200,a:90%,",
            "2026-02-11 10:00:00,english,50,75.0,71.2,95.0,190,200,,",
            "2026-02-12 10:00:00,peter1000,50,78.0,74.1,95.0,380,400,y:50%,hello",
            "2026-02-13 10:00:00,peter1000,50,80.0,76.0,95.0,380,400,,",
            "2026-02-14 10:00:00,peter1000,50,82.0,77.9,95.0,380,400,,world",
        ]
    }

    #[test]
    fn test_last_limits_to_n_entries() {
        let lines = sample_csv_lines();
        let rows = format_history_rows(&lines, Some(2));
        assert_eq!(rows.len(), 2);
        // Should be the last 2 entries (Feb 13 and Feb 14)
        assert!(rows[0].starts_with("2026-02-13"));
        assert!(rows[1].starts_with("2026-02-14"));
    }

    #[test]
    fn test_last_larger_than_total_shows_all() {
        let lines = sample_csv_lines();
        let rows = format_history_rows(&lines, Some(100));
        assert_eq!(rows.len(), 5);
        assert!(rows[0].starts_with("2026-02-10"));
    }

    #[test]
    fn test_last_none_shows_all() {
        let lines = sample_csv_lines();
        let rows = format_history_rows(&lines, None);
        assert_eq!(rows.len(), 5);
    }

    #[test]
    fn test_last_zero_shows_nothing() {
        let lines = sample_csv_lines();
        let rows = format_history_rows(&lines, Some(0));
        assert_eq!(rows.len(), 0);
    }
}
