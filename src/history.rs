use crate::test::results::{Fraction, Results};

use crossterm::event::{KeyCode, KeyEvent};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

pub const WPM_PER_CPS: f64 = 12.0;
const CSV_HEADER: &str =
    "datetime,language,words,wpm_raw,wpm_adjusted,accuracy,correct,total,worst_keys,missed_words,avg_dwell_ms";

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

    let dwell_str = results
        .dwell
        .overall_avg_ms
        .map_or(String::new(), |ms| format!("{:.1}", ms));

    format!(
        "{},{},{},{:.1},{:.1},{:.1},{},{},{},{},{}",
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
        dwell_str,
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

/// Filtering options for history display.
pub struct Filters<'a> {
    pub language: Option<&'a str>,
    pub since: Option<&'a str>,
    pub until: Option<&'a str>,
}

/// Check if a CSV line matches the given filters.
/// Fields: datetime(0), language(1), ...
fn matches_filters(fields: &[&str], filters: &Filters) -> bool {
    if let Some(lang) = filters.language {
        if fields.len() < 2 || fields[1] != lang {
            return false;
        }
    }
    if let Some(since) = filters.since {
        if fields.is_empty() || fields[0].len() < 10 || &fields[0][..10] < since {
            return false;
        }
    }
    if let Some(until) = filters.until {
        if fields.is_empty() || fields[0].len() < 10 || &fields[0][..10] > until {
            return false;
        }
    }
    true
}

/// Validate date format (YYYY-MM-DD).
pub fn validate_date_format(date: &str) -> Result<(), String> {
    if date.len() != 10
        || date.as_bytes()[4] != b'-'
        || date.as_bytes()[7] != b'-'
        || !date[..4].chars().all(|c| c.is_ascii_digit())
        || !date[5..7].chars().all(|c| c.is_ascii_digit())
        || !date[8..10].chars().all(|c| c.is_ascii_digit())
    {
        return Err(format!(
            "Error: Invalid date format '{}'. Expected YYYY-MM-DD (e.g., 2026-02-14).",
            date
        ));
    }
    Ok(())
}

/// Format history data rows into displayable lines.
/// Applies filters first, then `last` limits output to the most recent N entries.
fn format_history_rows(data_lines: &[&str], last: Option<usize>, filters: &Filters) -> Vec<String> {
    let filtered: Vec<&str> = data_lines
        .iter()
        .filter(|line| {
            let fields: Vec<&str> = line.splitn(10, ',').collect();
            fields.len() >= 9 && matches_filters(&fields, filters)
        })
        .copied()
        .collect();

    let skip = match last {
        Some(n) if n < filtered.len() => filtered.len() - n,
        _ => 0,
    };

    filtered
        .iter()
        .skip(skip)
        .map(|line| {
            let fields: Vec<&str> = line.splitn(10, ',').collect();
            format!(
                "{:<20} {:<15} {:>5} {:>8} {:>8} {:>8} {}",
                fields[0], fields[1], fields[2], fields[3], fields[4], fields[5], fields[8]
            )
        })
        .collect()
}

/// Display history from CSV file in a formatted table.
/// `last` limits output to the most recent N entries. None means show all.
/// `filters` narrows results by language and/or date range.
pub fn show_history(history_file: &Path, last: Option<usize>, filters: &Filters) {
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
    let rows = format_history_rows(data_lines, last, filters);
    let shown = rows.len();
    let total = data_lines.len();

    let has_filters = last.is_some()
        || filters.language.is_some()
        || filters.since.is_some()
        || filters.until.is_some();

    if shown == 0 && has_filters {
        println!("No matching results for the given filters.");
        return;
    }

    println!(
        "{:<20} {:<15} {:>5} {:>8} {:>8} {:>8} {}",
        "Date", "Language", "Words", "Raw WPM", "Adj WPM", "Acc %", "Worst Keys"
    );
    println!("{}", "-".repeat(90));

    for row in &rows {
        println!("{}", row);
    }

    if has_filters {
        println!(
            "\nShowing {} of {} results. History file: {}",
            shown, total, history_file.display()
        );
    } else {
        println!(
            "\n{} results total. History file: {}",
            total, history_file.display()
        );
    }
}

/// A parsed history row for stats computation.
struct HistoryRow {
    date: String,
    language: String,
    wpm_raw: f64,
    wpm_adj: f64,
    accuracy: f64,
    avg_dwell_ms: Option<f64>,
}

/// Parse filtered CSV data lines into HistoryRow structs.
fn parse_history_rows(data_lines: &[&str], filters: &Filters) -> Vec<HistoryRow> {
    data_lines
        .iter()
        .filter_map(|line| {
            let fields: Vec<&str> = line.splitn(12, ',').collect();
            if fields.len() < 9 || !matches_filters(&fields, filters) {
                return None;
            }
            Some(HistoryRow {
                date: fields[0][..10].to_string(),
                language: fields[1].to_string(),
                wpm_raw: fields[3].parse().ok()?,
                wpm_adj: fields[4].parse().ok()?,
                accuracy: fields[5].parse().ok()?,
                avg_dwell_ms: fields.get(10).and_then(|s| s.parse().ok()),
            })
        })
        .collect()
}

/// Compute overall statistics from parsed rows.
fn compute_overall_stats(rows: &[HistoryRow]) -> (f64, f64, f64, String, String, usize) {
    if rows.is_empty() {
        return (0.0, 0.0, 0.0, String::new(), String::new(), 0);
    }

    let avg_raw: f64 = rows.iter().map(|r| r.wpm_raw).sum::<f64>() / rows.len() as f64;
    let avg_adj: f64 = rows.iter().map(|r| r.wpm_adj).sum::<f64>() / rows.len() as f64;
    let avg_acc: f64 = rows.iter().map(|r| r.accuracy).sum::<f64>() / rows.len() as f64;
    let first_date = rows.first().map(|r| r.date.clone()).unwrap_or_default();

    // Most practiced language
    let mut lang_counts: HashMap<&str, usize> = HashMap::new();
    for row in rows {
        *lang_counts.entry(&row.language).or_insert(0) += 1;
    }
    let (most_lang, most_count) = lang_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .unwrap_or(("", 0));

    (avg_raw, avg_adj, avg_acc, first_date, most_lang.to_string(), most_count)
}

/// Compute stats for rows within a date range (inclusive string comparison on YYYY-MM-DD).
fn rows_in_range<'a>(rows: &'a [HistoryRow], since: &str, until: &str) -> Vec<&'a HistoryRow> {
    rows.iter()
        .filter(|r| r.date.as_str() >= since && r.date.as_str() <= until)
        .collect()
}

/// Compute average adjusted WPM for a slice of rows.
fn avg_wpm(rows: &[&HistoryRow]) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    rows.iter().map(|r| r.wpm_adj).sum::<f64>() / rows.len() as f64
}

/// Compute average accuracy for a slice of rows.
fn avg_accuracy(rows: &[&HistoryRow]) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    rows.iter().map(|r| r.accuracy).sum::<f64>() / rows.len() as f64
}

/// Find best session (highest adjusted WPM) from a slice of rows.
fn best_session<'a>(rows: &[&'a HistoryRow]) -> Option<(&'a str, f64)> {
    rows.iter()
        .max_by(|a, b| a.wpm_adj.partial_cmp(&b.wpm_adj).unwrap())
        .map(|r| (r.date.as_str(), r.wpm_adj))
}

/// Compute weekly WPM averages. Returns (ISO week label, avg adjusted WPM) pairs.
fn weekly_trend(rows: &[HistoryRow]) -> Vec<(String, f64)> {
    use chrono::NaiveDate;

    let mut week_data: HashMap<String, Vec<f64>> = HashMap::new();
    for row in rows {
        if let Ok(date) = NaiveDate::parse_from_str(&row.date, "%Y-%m-%d") {
            let week = date.format("W%V").to_string();
            week_data.entry(week).or_default().push(row.wpm_adj);
        }
    }

    let mut weeks: Vec<(String, f64)> = week_data
        .into_iter()
        .map(|(week, wpms)| {
            let avg = wpms.iter().sum::<f64>() / wpms.len() as f64;
            (week, avg)
        })
        .collect();
    weeks.sort_by(|a, b| a.0.cmp(&b.0));

    // Show last 6 weeks max
    if weeks.len() > 6 {
        weeks = weeks.split_off(weeks.len() - 6);
    }
    weeks
}

/// Display aggregated statistics from history CSV file.
pub fn show_stats(history_file: &Path, filters: &Filters) {
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
    let rows = parse_history_rows(data_lines, filters);

    if rows.is_empty() {
        println!("No matching results for the given filters.");
        return;
    }

    let (avg_raw, avg_adj, avg_acc, first_date, most_lang, most_count) =
        compute_overall_stats(&rows);

    println!(
        "Overall ({} tests, since {})",
        rows.len(),
        first_date
    );
    println!("  Avg WPM: {:.1} (raw: {:.1})", avg_adj, avg_raw);
    println!("  Avg Accuracy: {:.1}%", avg_acc);
    println!(
        "  Most practiced: {} ({} tests)",
        most_lang, most_count
    );

    // Overall dwell stats (if any rows have dwell data)
    let dwell_values: Vec<f64> = rows.iter().filter_map(|r| r.avg_dwell_ms).collect();
    if !dwell_values.is_empty() {
        let avg_dwell = dwell_values.iter().sum::<f64>() / dwell_values.len() as f64;
        println!("  Avg Key Dwell: {:.0}ms", avg_dwell);
    }

    // Last 7 days stats
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let seven_days_ago = (chrono::Local::now() - chrono::Duration::days(7))
        .format("%Y-%m-%d")
        .to_string();
    let eight_days_ago = (chrono::Local::now() - chrono::Duration::days(8))
        .format("%Y-%m-%d")
        .to_string();
    let fourteen_days_ago = (chrono::Local::now() - chrono::Duration::days(14))
        .format("%Y-%m-%d")
        .to_string();

    let recent = rows_in_range(&rows, &seven_days_ago, &today);
    if !recent.is_empty() {
        let recent_wpm = avg_wpm(&recent);
        let recent_acc = avg_accuracy(&recent);

        println!(
            "\nLast 7 days ({} tests)",
            recent.len()
        );
        // Delta vs prior week (exclusive upper bound to avoid double-counting)
        let prior = rows_in_range(&rows, &fourteen_days_ago, &eight_days_ago);
        if !prior.is_empty() {
            let prior_wpm = avg_wpm(&prior);
            let delta = recent_wpm - prior_wpm;
            let sign = if delta >= 0.0 { "+" } else { "" };
            println!(
                "  Avg WPM: {:.1} ({}{:.1} vs prior week)",
                recent_wpm, sign, delta
            );
        } else {
            println!("  Avg WPM: {:.1}", recent_wpm);
        }
        println!("  Avg Accuracy: {:.1}%", recent_acc);
        if let Some((date, wpm)) = best_session(&recent) {
            println!("  Best session: {:.1} WPM on {}", wpm, date);
        }
    }

    // Weekly trend
    let weeks = weekly_trend(&rows);
    if weeks.len() >= 2 {
        let trend_arrow = if weeks.last().unwrap().1 > weeks[weeks.len() - 2].1 {
            " ^"
        } else if weeks.last().unwrap().1 < weeks[weeks.len() - 2].1 {
            " v"
        } else {
            ""
        };
        println!("\nWeekly Trend (Adj WPM):");
        let trend_str: String = weeks
            .iter()
            .map(|(week, wpm)| format!("  {}: {:.1}", week, wpm))
            .collect::<Vec<_>>()
            .join("  ");
        println!("{}{}", trend_str, trend_arrow);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::results::{AccuracyData, DwellData, TimingData};
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
            dwell: DwellData {
                per_key: vec![],
                overall_avg_ms: None,
                has_data: false,
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
        let fields: Vec<&str> = line.splitn(12, ',').collect();

        assert_eq!(fields.len(), 11);
        assert_eq!(fields[0], "2026-02-14 12:43:34");
        assert_eq!(fields[1], "peter1000");
        assert_eq!(fields[2], "50");
        assert_eq!(fields[6], "380");
        assert_eq!(fields[7], "400");
        assert_eq!(fields[9], "Architektur;Frontend");
        assert_eq!(fields[10], "", "No dwell data → empty field");
    }

    #[test]
    fn test_format_csv_line_wpm_values() {
        let results = make_results(6.5, 380, 400, vec![], vec![]);

        let line = format_csv_line("2026-02-14 12:00:00", "test", 50, &results);
        let fields: Vec<&str> = line.splitn(12, ',').collect();

        assert_eq!(fields[3], "78.0"); // 6.5 * 12 = 78.0
        assert_eq!(fields[4], "74.1"); // 78.0 * 0.95 = 74.1
        assert_eq!(fields[5], "95.0"); // 380/400 = 95%
    }

    #[test]
    fn test_format_csv_line_empty_missed_words() {
        let results = make_results(5.0, 100, 100, vec![], vec![]);

        let line = format_csv_line("2026-02-14 12:00:00", "test", 50, &results);
        let fields: Vec<&str> = line.splitn(12, ',').collect();

        assert_eq!(fields[9], "", "missed_words should be empty");
        assert_eq!(fields[10], "", "dwell should be empty when no data");
    }

    #[test]
    fn test_format_csv_line_with_dwell_data() {
        let mut results = make_results(5.0, 100, 100, vec![], vec![]);
        results.dwell = DwellData {
            per_key: vec![('a', 95.0), ('b', 110.0)],
            overall_avg_ms: Some(102.5),
            has_data: true,
        };

        let line = format_csv_line("2026-02-14 12:00:00", "test", 50, &results);
        let fields: Vec<&str> = line.splitn(12, ',').collect();

        assert_eq!(fields[10], "102.5", "avg_dwell_ms should be present");
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

    // --- History display limiting and filtering ---

    const NO_FILTERS: Filters<'static> = Filters {
        language: None,
        since: None,
        until: None,
    };

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
        let rows = format_history_rows(&lines, Some(2), &NO_FILTERS);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("2026-02-13"));
        assert!(rows[1].starts_with("2026-02-14"));
    }

    #[test]
    fn test_last_larger_than_total_shows_all() {
        let lines = sample_csv_lines();
        let rows = format_history_rows(&lines, Some(100), &NO_FILTERS);
        assert_eq!(rows.len(), 5);
        assert!(rows[0].starts_with("2026-02-10"));
    }

    #[test]
    fn test_last_none_shows_all() {
        let lines = sample_csv_lines();
        let rows = format_history_rows(&lines, None, &NO_FILTERS);
        assert_eq!(rows.len(), 5);
    }

    #[test]
    fn test_last_zero_shows_nothing() {
        let lines = sample_csv_lines();
        let rows = format_history_rows(&lines, Some(0), &NO_FILTERS);
        assert_eq!(rows.len(), 0);
    }

    // --- Language filtering ---

    #[test]
    fn test_filter_by_language() {
        let lines = sample_csv_lines();
        let filters = Filters { language: Some("peter1000"), since: None, until: None };
        let rows = format_history_rows(&lines, None, &filters);
        assert_eq!(rows.len(), 3);
        assert!(rows[0].starts_with("2026-02-12"));
        assert!(rows[2].starts_with("2026-02-14"));
    }

    #[test]
    fn test_filter_by_language_no_match() {
        let lines = sample_csv_lines();
        let filters = Filters { language: Some("german"), since: None, until: None };
        let rows = format_history_rows(&lines, None, &filters);
        assert_eq!(rows.len(), 0);
    }

    // --- Date filtering ---

    #[test]
    fn test_filter_since() {
        let lines = sample_csv_lines();
        let filters = Filters { language: None, since: Some("2026-02-13"), until: None };
        let rows = format_history_rows(&lines, None, &filters);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("2026-02-13"));
        assert!(rows[1].starts_with("2026-02-14"));
    }

    #[test]
    fn test_filter_until() {
        let lines = sample_csv_lines();
        let filters = Filters { language: None, since: None, until: Some("2026-02-11") };
        let rows = format_history_rows(&lines, None, &filters);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("2026-02-10"));
        assert!(rows[1].starts_with("2026-02-11"));
    }

    #[test]
    fn test_filter_date_range() {
        let lines = sample_csv_lines();
        let filters = Filters { language: None, since: Some("2026-02-11"), until: Some("2026-02-13") };
        let rows = format_history_rows(&lines, None, &filters);
        assert_eq!(rows.len(), 3);
        assert!(rows[0].starts_with("2026-02-11"));
        assert!(rows[2].starts_with("2026-02-13"));
    }

    // --- Combined filters ---

    #[test]
    fn test_filter_language_and_date_range() {
        let lines = sample_csv_lines();
        let filters = Filters { language: Some("peter1000"), since: Some("2026-02-13"), until: None };
        let rows = format_history_rows(&lines, None, &filters);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("2026-02-13"));
        assert!(rows[1].starts_with("2026-02-14"));
    }

    #[test]
    fn test_filter_with_last() {
        let lines = sample_csv_lines();
        let filters = Filters { language: Some("peter1000"), since: None, until: None };
        // 3 peter1000 entries, take last 1
        let rows = format_history_rows(&lines, Some(1), &filters);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].starts_with("2026-02-14"));
    }

    #[test]
    fn test_filter_same_day_range() {
        let lines = sample_csv_lines();
        let filters = Filters { language: None, since: Some("2026-02-12"), until: Some("2026-02-12") };
        let rows = format_history_rows(&lines, None, &filters);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].starts_with("2026-02-12"));
    }

    // --- Date validation ---

    #[test]
    fn test_validate_date_valid() {
        assert!(validate_date_format("2026-02-14").is_ok());
        assert!(validate_date_format("2000-01-01").is_ok());
    }

    #[test]
    fn test_validate_date_invalid_format() {
        assert!(validate_date_format("2026-2-5").is_err());
        assert!(validate_date_format("14-02-2026").is_err());
        assert!(validate_date_format("not-a-date").is_err());
        assert!(validate_date_format("2026/02/14").is_err());
        assert!(validate_date_format("20260214").is_err());
    }

    // --- Short timestamp robustness ---

    #[test]
    fn test_filter_skips_malformed_short_timestamp() {
        let lines = vec![
            "short,english,50,72.0,68.4,95.0,190,200,a:90%,",
            "2026-02-14 10:00:00,english,50,75.0,71.2,95.0,190,200,,",
        ];
        let filters = Filters { language: None, since: Some("2026-02-01"), until: None };
        let rows = format_history_rows(&lines, None, &filters);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].starts_with("2026-02-14"));
    }

    // --- Stats aggregation ---

    #[test]
    fn test_parse_history_rows() {
        let lines = sample_csv_lines();
        let rows = parse_history_rows(&lines, &NO_FILTERS);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].date, "2026-02-10");
        assert_eq!(rows[0].language, "english");
        assert!((rows[0].wpm_raw - 72.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_history_rows_with_language_filter() {
        let lines = sample_csv_lines();
        let filters = Filters { language: Some("peter1000"), since: None, until: None };
        let rows = parse_history_rows(&lines, &filters);
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|r| r.language == "peter1000"));
    }

    #[test]
    fn test_compute_overall_stats() {
        let lines = sample_csv_lines();
        let rows = parse_history_rows(&lines, &NO_FILTERS);
        let (avg_raw, avg_adj, avg_acc, first_date, most_lang, most_count) =
            compute_overall_stats(&rows);

        // (72 + 75 + 78 + 80 + 82) / 5 = 77.4
        assert!((avg_raw - 77.4).abs() < 0.01);
        // (68.4 + 71.2 + 74.1 + 76.0 + 77.9) / 5 = 73.52
        assert!((avg_adj - 73.52).abs() < 0.01);
        assert!((avg_acc - 95.0).abs() < 0.01);
        assert_eq!(first_date, "2026-02-10");
        assert_eq!(most_lang, "peter1000");
        assert_eq!(most_count, 3);
    }

    #[test]
    fn test_compute_overall_stats_empty() {
        let (avg_raw, _, _, _, _, _) = compute_overall_stats(&[]);
        assert!((avg_raw - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_weekly_trend() {
        let lines = sample_csv_lines();
        let rows = parse_history_rows(&lines, &NO_FILTERS);
        let weeks = weekly_trend(&rows);

        assert!(!weeks.is_empty());
        // All dates are in the same week (KW07 of 2026)
        // Feb 10 = Mon of KW07, Feb 14 = Fri of KW07
        assert_eq!(weeks.len(), 1);
        assert_eq!(weeks[0].0, "W07");
    }

    #[test]
    fn test_weekly_trend_multiple_weeks() {
        let lines = vec![
            "2026-01-27 10:00:00,english,50,70.0,66.5,95.0,190,200,,",
            "2026-02-03 10:00:00,english,50,75.0,71.2,95.0,190,200,,",
            "2026-02-10 10:00:00,english,50,80.0,76.0,95.0,190,200,,",
        ];
        let rows = parse_history_rows(&lines, &NO_FILTERS);
        let weeks = weekly_trend(&rows);

        assert_eq!(weeks.len(), 3);
        // Should be sorted chronologically
        assert!(weeks[0].1 < weeks[1].1);
        assert!(weeks[1].1 < weeks[2].1);
    }

    // --- Dwell CSV backward compatibility ---

    #[test]
    fn test_parse_old_csv_without_dwell() {
        // Old format: 10 fields, no dwell column
        let lines = sample_csv_lines();
        let rows = parse_history_rows(&lines, &NO_FILTERS);
        assert!(rows.iter().all(|r| r.avg_dwell_ms.is_none()));
    }

    #[test]
    fn test_parse_new_csv_with_dwell() {
        let lines = vec![
            "2026-02-14 10:00:00,english,50,82.0,77.9,95.0,380,400,,world,98.5",
        ];
        let rows = parse_history_rows(&lines, &NO_FILTERS);
        assert_eq!(rows.len(), 1);
        assert!((rows[0].avg_dwell_ms.unwrap() - 98.5).abs() < 0.01);
    }

    #[test]
    fn test_parse_mixed_csv_old_and_new() {
        let lines = vec![
            "2026-02-13 10:00:00,english,50,80.0,76.0,95.0,380,400,,",
            "2026-02-14 10:00:00,english,50,82.0,77.9,95.0,380,400,,,102.3",
        ];
        let rows = parse_history_rows(&lines, &NO_FILTERS);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].avg_dwell_ms.is_none());
        assert!((rows[1].avg_dwell_ms.unwrap() - 102.3).abs() < 0.01);
    }
}
