use crate::config::Theme;

use super::test::{results, Test, TestWord};

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    symbols::Marker,
    text::{Line, Span, Text},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Widget, Wrap},
};
use results::Fraction;

// Convert CPS to WPM (clicks per second)
const WPM_PER_CPS: f64 = 12.0;

// Width of the moving average window for the WPM chart
const WPM_SMA_WIDTH: usize = 10;

#[derive(Clone)]
struct SizedBlock<'a> {
    block: Block<'a>,
    area: Rect,
}

impl SizedBlock<'_> {
    fn render(self, buf: &mut Buffer) {
        self.block.render(self.area, buf)
    }
}

trait UsedWidget: Widget {}
impl UsedWidget for Paragraph<'_> {}

trait DrawInner<T> {
    fn draw_inner(&self, content: T, buf: &mut Buffer);
}

impl DrawInner<&Line<'_>> for SizedBlock<'_> {
    fn draw_inner(&self, content: &Line, buf: &mut Buffer) {
        let inner = self.block.inner(self.area);
        buf.set_line(inner.x, inner.y, content, inner.width);
    }
}

impl<T: UsedWidget> DrawInner<T> for SizedBlock<'_> {
    fn draw_inner(&self, content: T, buf: &mut Buffer) {
        let inner = self.block.inner(self.area);
        content.render(inner, buf);
    }
}

pub trait ThemedWidget {
    fn render(self, area: Rect, buf: &mut Buffer, theme: &Theme);
}

pub struct Themed<'t, W: ?Sized> {
    theme: &'t Theme,
    widget: W,
}
impl<W: ThemedWidget> Widget for Themed<'_, W> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.widget.render(area, buf, self.theme)
    }
}
impl Theme {
    pub fn apply_to<W>(&self, widget: W) -> Themed<'_, W> {
        Themed {
            theme: self,
            widget,
        }
    }
}

impl ThemedWidget for &Test {
    fn render(self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        buf.set_style(area, theme.default);

        // Chunks
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(6)])
            .split(area);

        // Sections
        let input = SizedBlock {
            block: Block::default()
                .title(Line::from(vec![Span::styled("Input", theme.title)]))
                .borders(Borders::ALL)
                .border_type(theme.border_type)
                .border_style(theme.input_border),
            area: chunks[0],
        };
        input.draw_inner(
            &Line::from(self.words[self.current_word].progress.clone()),
            buf,
        );
        input.render(buf);

        let target_lines: Vec<Line> = {
            let words = words_to_spans(
                &self.words,
                self.current_word,
                theme,
                self.case_insensitive,
                self.look_ahead,
            );

            let mut lines: Vec<Line> = Vec::new();
            let mut current_line: Vec<Span> = Vec::new();
            let mut current_width = 0;
            for word in words {
                let word_width: usize = word.iter().map(|s| s.width()).sum();

                if current_width + word_width > chunks[1].width as usize - 2 {
                    current_line.push(Span::raw("\n"));
                    lines.push(Line::from(current_line.clone()));
                    current_line.clear();
                    current_width = 0;
                }

                current_line.extend(word);
                current_width += word_width;
            }
            lines.push(Line::from(current_line));

            lines
        };
        let target = Paragraph::new(target_lines).block(
            Block::default()
                .title(Span::styled("Prompt", theme.title))
                .borders(Borders::ALL)
                .border_type(theme.border_type)
                .border_style(theme.prompt_border),
        );
        target.render(chunks[1], buf);
    }
}

fn words_to_spans<'a>(
    words: &'a [TestWord],
    current_word: usize,
    theme: &'a Theme,
    case_insensitive: bool,
    look_ahead: Option<usize>,
) -> Vec<Vec<Span<'a>>> {
    let mut spans = Vec::new();

    for word in &words[..current_word] {
        let parts = split_typed_word(word, case_insensitive);
        spans.push(word_parts_to_spans(parts, theme));
    }

    let parts_current = split_current_word(&words[current_word], case_insensitive);
    spans.push(word_parts_to_spans(parts_current, theme));

    let visible_end = match look_ahead {
        Some(n) => (current_word + 1 + n).min(words.len()),
        None => words.len(),
    };

    for word in &words[current_word + 1..visible_end] {
        let parts = vec![(word.text.clone(), Status::Untyped)];
        spans.push(word_parts_to_spans(parts, theme));
    }
    spans
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum Status {
    Correct,
    Incorrect,
    CurrentUntyped,
    CurrentCorrect,
    CurrentIncorrect,
    Cursor,
    Untyped,
    Overtyped,
}

fn split_current_word(word: &TestWord, case_insensitive: bool) -> Vec<(String, Status)> {
    let mut parts = Vec::new();
    let mut cur_string = String::new();
    let mut cur_status = Status::Untyped;

    let mut progress = word.progress.chars();
    for tc in word.text.chars() {
        let p = progress.next();
        let status = match p {
            None => Status::CurrentUntyped,
            Some(c) => {
                let matches = if case_insensitive {
                    c.to_lowercase().eq(tc.to_lowercase())
                } else {
                    c == tc
                };
                if matches {
                    Status::CurrentCorrect
                } else {
                    Status::CurrentIncorrect
                }
            }
        };

        if status == cur_status {
            cur_string.push(tc);
        } else {
            if !cur_string.is_empty() {
                parts.push((cur_string, cur_status));
                cur_string = String::new();
            }
            cur_string.push(tc);
            cur_status = status;

            // first currentuntyped is cursor
            if status == Status::CurrentUntyped {
                parts.push((cur_string, Status::Cursor));
                cur_string = String::new();
            }
        }
    }
    if !cur_string.is_empty() {
        parts.push((cur_string, cur_status));
    }
    let overtyped = progress.collect::<String>();
    if !overtyped.is_empty() {
        parts.push((overtyped, Status::Overtyped));
    }
    parts
}

fn split_typed_word(word: &TestWord, case_insensitive: bool) -> Vec<(String, Status)> {
    let mut parts = Vec::new();
    let mut cur_string = String::new();
    let mut cur_status = Status::Untyped;

    let mut progress = word.progress.chars();
    for tc in word.text.chars() {
        let p = progress.next();
        let status = match p {
            None => Status::Untyped,
            Some(c) => {
                let matches = if case_insensitive {
                    c.to_lowercase().eq(tc.to_lowercase())
                } else {
                    c == tc
                };
                if matches {
                    Status::Correct
                } else {
                    Status::Incorrect
                }
            }
        };

        if status == cur_status {
            cur_string.push(tc);
        } else {
            if !cur_string.is_empty() {
                parts.push((cur_string, cur_status));
                cur_string = String::new();
            }
            cur_string.push(tc);
            cur_status = status;
        }
    }
    if !cur_string.is_empty() {
        parts.push((cur_string, cur_status));
    }

    let overtyped = progress.collect::<String>();
    if !overtyped.is_empty() {
        parts.push((overtyped, Status::Overtyped));
    }
    parts
}

fn word_parts_to_spans(parts: Vec<(String, Status)>, theme: &Theme) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    for (text, status) in parts {
        let style = match status {
            Status::Correct => theme.prompt_correct,
            Status::Incorrect => theme.prompt_incorrect,
            Status::Untyped => theme.prompt_untyped,
            Status::CurrentUntyped => theme.prompt_current_untyped,
            Status::CurrentCorrect => theme.prompt_current_correct,
            Status::CurrentIncorrect => theme.prompt_current_incorrect,
            Status::Cursor => theme.prompt_current_untyped.patch(theme.prompt_cursor),
            Status::Overtyped => theme.prompt_incorrect,
        };

        spans.push(Span::styled(text, style));
    }
    spans.push(Span::styled(" ", theme.prompt_untyped));
    spans
}

impl ThemedWidget for &results::Results {
    fn render(self, area: Rect, buf: &mut Buffer, theme: &Theme) {
        buf.set_style(area, theme.default);

        // Chunks
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        let res_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1) // Graph looks tremendously better with just a little margin
            .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
            .split(chunks[0]);
        let has_slow_words = !self.slow_words.is_empty();
        let has_dwell = self.dwell.has_data;
        let panel_count = 2 + has_slow_words as u32 + has_dwell as u32;
        let info_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                (0..panel_count)
                    .map(|_| Constraint::Ratio(1, panel_count))
                    .collect::<Vec<_>>(),
            )
            .split(res_chunks[0]);

        let msg = match (self.slow_words.is_empty(), self.missed_words.is_empty()) {
            (true, true) => "Press 'q' to quit, 'r' for new test or 't' to repeat",
            (false, true) => "Press 'q' to quit, 'r' new, 't' repeat or 's' to practice slow",
            (true, false) => "Press 'q' to quit, 'r' new, 't' repeat or 'p' to practice missed",
            (false, false) => "Press 'q' quit, 'r' new, 't' repeat, 's' slow or 'p' missed",
        };

        let exit = Span::styled(msg, theme.results_restart_prompt);
        buf.set_span(chunks[1].x, chunks[1].y, &exit, chunks[1].width);

        // Sections
        let mut overview_text = Text::styled("", theme.results_overview);
        overview_text.extend([
            Line::from(format!(
                "Adjusted WPM: {:.1}",
                self.timing.overall_cps * WPM_PER_CPS * f64::from(self.accuracy.overall)
            )),
            Line::from(format!(
                "Accuracy: {:.1}%",
                f64::from(self.accuracy.overall) * 100f64
            )),
            Line::from(format!(
                "Raw WPM: {:.1}",
                self.timing.overall_cps * WPM_PER_CPS
            )),
            Line::from(format!("Correct Keypresses: {}", self.accuracy.overall)),
        ]);
        let overview = Paragraph::new(overview_text)
            .block(
                Block::default()
                    .title(Span::styled("Overview", theme.title))
                    .borders(Borders::ALL)
                    .border_type(theme.border_type)
                    .border_style(theme.results_overview_border),
            )
            .wrap(Wrap { trim: true });
        overview.render(info_chunks[0], buf);

        let mut worst_keys: Vec<(&KeyEvent, &Fraction)> = self
            .accuracy
            .per_key
            .iter()
            .filter(|(key, _)| matches!(key.code, KeyCode::Char(_)))
            .collect();
        worst_keys.sort_unstable_by_key(|x| x.1);

        let mut worst_text = Text::styled("", theme.results_worst_keys);
        worst_text.extend(
            worst_keys
                .iter()
                .filter_map(|(key, acc)| {
                    if let KeyCode::Char(character) = key.code {
                        let key_accuracy = f64::from(**acc) * 100.0;
                        if key_accuracy != 100.0 {
                            Some(format!("- {} at {:.1}% accuracy", character, key_accuracy))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .take(5)
                .map(Line::from),
        );
        let worst = Paragraph::new(worst_text)
            .block(
                Block::default()
                    .title(Span::styled("Worst Keys", theme.title))
                    .borders(Borders::ALL)
                    .border_type(theme.border_type)
                    .border_style(theme.results_worst_keys_border),
            )
            .wrap(Wrap { trim: true });
        worst.render(info_chunks[1], buf);

        let mut next_chunk = 2;
        if has_slow_words {
            let mut slow_text = Text::styled("", theme.results_worst_keys);
            slow_text.extend(
                self.slow_words
                    .iter()
                    .take(5)
                    .map(|w| Line::from(format!("- {}", w))),
            );
            let slow = Paragraph::new(slow_text)
                .block(
                    Block::default()
                        .title(Span::styled("Slow Words", theme.title))
                        .borders(Borders::ALL)
                        .border_type(theme.border_type)
                        .border_style(theme.results_worst_keys_border),
                )
                .wrap(Wrap { trim: true });
            slow.render(info_chunks[next_chunk], buf);
            next_chunk += 1;
        }

        if has_dwell {
            let mut dwell_text = Text::styled("", theme.results_worst_keys);
            dwell_text.extend(
                self.dwell
                    .per_key
                    .iter()
                    .take(5)
                    .map(|(ch, ms)| Line::from(format!("- {}: {:.0}ms", ch, ms))),
            );
            if let Some(avg) = self.dwell.overall_avg_ms {
                dwell_text.extend([Line::from(format!("avg: {:.0}ms", avg))]);
            }
            let dwell = Paragraph::new(dwell_text)
                .block(
                    Block::default()
                        .title(Span::styled("Key Hold", theme.title))
                        .borders(Borders::ALL)
                        .border_type(theme.border_type)
                        .border_style(theme.results_worst_keys_border),
                )
                .wrap(Wrap { trim: true });
            dwell.render(info_chunks[next_chunk], buf);
        }

        let wpm_sma: Vec<(f64, f64)> = self
            .timing
            .per_event
            .windows(WPM_SMA_WIDTH)
            .enumerate()
            .map(|(i, window)| {
                (
                    (i + WPM_SMA_WIDTH) as f64,
                    window.len() as f64 / window.iter().copied().sum::<f64>() * WPM_PER_CPS,
                )
            })
            .collect();

        // Render the chart if possible
        if !wpm_sma.is_empty() {
            let wpm_sma_min = wpm_sma
                .iter()
                .map(|(_, x)| x)
                .fold(f64::INFINITY, |a, &b| a.min(b));
            let wpm_sma_max = wpm_sma
                .iter()
                .map(|(_, x)| x)
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));

            let wpm_datasets = vec![Dataset::default()
                .name("WPM")
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(theme.results_chart)
                .data(&wpm_sma)];

            let y_label_min = wpm_sma_min as u16;
            let y_label_max = (wpm_sma_max as u16).max(y_label_min + 6);

            let wpm_chart = Chart::new(wpm_datasets)
                .block(Block::default().title(vec![Span::styled("Chart", theme.title)]))
                .x_axis(
                    Axis::default()
                        .title(Span::styled("Keypresses", theme.results_chart_x))
                        .bounds([0.0, self.timing.per_event.len() as f64]),
                )
                .y_axis(
                    Axis::default()
                        .title(Span::styled(
                            "WPM (10-keypress rolling average)",
                            theme.results_chart_y,
                        ))
                        .bounds([wpm_sma_min, wpm_sma_max])
                        .labels(
                            (y_label_min..y_label_max)
                                .step_by(5)
                                .map(|n| Span::raw(format!("{}", n)))
                                .collect(),
                        ),
                );
            wpm_chart.render(res_chunks[1], buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod split_words {
        use super::Status::*;
        use super::*;

        struct TestCase {
            word: &'static str,
            progress: &'static str,
            expected: Vec<(&'static str, Status)>,
        }

        fn setup(test_case: TestCase) -> (TestWord, Vec<(String, Status)>) {
            let mut word = TestWord::from(test_case.word);
            word.progress = test_case.progress.to_string();

            let expected = test_case
                .expected
                .iter()
                .map(|(s, v)| (s.to_string(), *v))
                .collect::<Vec<_>>();

            (word, expected)
        }

        #[test]
        fn typed_words_split() {
            let cases = vec![
                TestCase {
                    word: "monkeytype",
                    progress: "monkeytype",
                    expected: vec![("monkeytype", Correct)],
                },
                TestCase {
                    word: "monkeytype",
                    progress: "monkeXtype",
                    expected: vec![("monke", Correct), ("y", Incorrect), ("type", Correct)],
                },
                TestCase {
                    word: "monkeytype",
                    progress: "monkeas",
                    expected: vec![("monke", Correct), ("yt", Incorrect), ("ype", Untyped)],
                },
            ];

            for case in cases {
                let (word, expected) = setup(case);
                let got = split_typed_word(&word, false);
                assert_eq!(got, expected);
            }
        }

        #[test]
        fn words_to_spans_no_look_ahead_shows_all() {
            let theme = Theme::default();
            let words: Vec<TestWord> = vec!["a", "b", "c", "d", "e"]
                .into_iter()
                .map(TestWord::from)
                .collect();
            let spans = words_to_spans(&words, 0, &theme, false, None);
            assert_eq!(
                spans.len(),
                5,
                "Without look_ahead, all 5 words should be visible"
            );
        }

        #[test]
        fn words_to_spans_look_ahead_limits_visibility() {
            let theme = Theme::default();
            let words: Vec<TestWord> = vec!["a", "b", "c", "d", "e"]
                .into_iter()
                .map(TestWord::from)
                .collect();
            // current_word=0, look_ahead=2: should show word 0 (current) + 2 upcoming = 3 total
            let spans = words_to_spans(&words, 0, &theme, false, Some(2));
            assert_eq!(
                spans.len(),
                3,
                "With look_ahead=2, should show current + 2 upcoming words"
            );
        }

        #[test]
        fn words_to_spans_look_ahead_one() {
            let theme = Theme::default();
            let words: Vec<TestWord> = vec!["a", "b", "c", "d"]
                .into_iter()
                .map(TestWord::from)
                .collect();
            // current_word=1, look_ahead=1: words[0] (typed) + word[1] (current) + word[2] (next) = 3
            let mut word0 = TestWord::from("a");
            word0.progress = "a".to_string();
            let words = vec![
                word0,
                TestWord::from("b"),
                TestWord::from("c"),
                TestWord::from("d"),
            ];
            let spans = words_to_spans(&words, 1, &theme, false, Some(1));
            assert_eq!(
                spans.len(),
                3,
                "With look_ahead=1 at word 1: past(1) + current(1) + upcoming(1) = 3"
            );
        }

        #[test]
        fn words_to_spans_look_ahead_clamps_to_end() {
            let theme = Theme::default();
            let words: Vec<TestWord> = vec!["a", "b"].into_iter().map(TestWord::from).collect();
            // current_word=0, look_ahead=10: only 1 upcoming word exists
            let spans = words_to_spans(&words, 0, &theme, false, Some(10));
            assert_eq!(
                spans.len(),
                2,
                "Look ahead larger than remaining words should clamp to end"
            );
        }

        #[test]
        fn words_to_spans_look_ahead_zero_shows_only_current() {
            let theme = Theme::default();
            let words: Vec<TestWord> = vec!["a", "b", "c", "d"]
                .into_iter()
                .map(TestWord::from)
                .collect();
            // look_ahead=0: show only the current word, no upcoming words
            let spans = words_to_spans(&words, 0, &theme, false, Some(0));
            assert_eq!(
                spans.len(),
                1,
                "With look_ahead=0, only the current word should be visible"
            );
        }

        #[test]
        fn words_to_spans_look_ahead_at_last_word() {
            let theme = Theme::default();
            let mut word0 = TestWord::from("a");
            word0.progress = "a".to_string();
            let mut word1 = TestWord::from("b");
            word1.progress = "b".to_string();
            let words = vec![word0, word1, TestWord::from("c")];
            // current_word=2 (last word), look_ahead=5: no upcoming words to show
            let spans = words_to_spans(&words, 2, &theme, false, Some(5));
            assert_eq!(
                spans.len(),
                3,
                "At last word: past(2) + current(1) + no upcoming = 3"
            );
        }

        #[test]
        fn current_word_split() {
            let cases = vec![
                TestCase {
                    word: "monkeytype",
                    progress: "monkeytype",
                    expected: vec![("monkeytype", CurrentCorrect)],
                },
                TestCase {
                    word: "monkeytype",
                    progress: "monke",
                    expected: vec![
                        ("monke", CurrentCorrect),
                        ("y", Cursor),
                        ("type", CurrentUntyped),
                    ],
                },
                TestCase {
                    word: "monkeytype",
                    progress: "monkeXt",
                    expected: vec![
                        ("monke", CurrentCorrect),
                        ("y", CurrentIncorrect),
                        ("t", CurrentCorrect),
                        ("y", Cursor),
                        ("pe", CurrentUntyped),
                    ],
                },
            ];

            for case in cases {
                let (word, expected) = setup(case);
                let got = split_current_word(&word, false);
                assert_eq!(got, expected);
            }
        }
    }
}
