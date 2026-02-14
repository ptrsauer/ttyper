mod config;
mod test;
mod ui;

use config::Config;
use test::{results::Results, Test};

use clap::Parser;
use crossterm::{
    self, cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, terminal,
};
use rand::{seq::SliceRandom, thread_rng};
use ratatui::{backend::CrosstermBackend, terminal::Terminal};
use rust_embed::RustEmbed;
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, Write},
    num,
    path::PathBuf,
    str,
};

#[derive(RustEmbed)]
#[folder = "resources/runtime"]
struct Resources;

#[derive(Debug, Parser)]
#[command(about, version)]
struct Opt {
    /// Read test contents from the specified file, or "-" for stdin
    #[arg(value_name = "PATH")]
    contents: Option<PathBuf>,

    #[arg(short, long)]
    debug: bool,

    /// Specify word count
    #[arg(short, long, value_name = "N", default_value = "50")]
    words: num::NonZeroUsize,

    /// Use config file
    #[arg(short, long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Specify test language in file
    #[arg(long, value_name = "PATH")]
    language_file: Option<PathBuf>,

    /// Specify test language
    #[arg(short, long, value_name = "LANG")]
    language: Option<String>,

    /// List installed languages
    #[arg(long)]
    list_languages: bool,

    /// Disable backtracking to completed words
    #[arg(long)]
    no_backtrack: bool,

    /// Enable sudden death mode to restart on first error
    #[arg(long)]
    sudden_death: bool,

    /// Show history of past results
    #[arg(long)]
    history: bool,

    /// Disable saving results to history
    #[arg(long)]
    no_save: bool,
}

impl Opt {
    fn gen_contents(&self) -> Option<Vec<String>> {
        match &self.contents {
            Some(path) => {
                let lines: Vec<String> = if path.as_os_str() == "-" {
                    std::io::stdin()
                        .lock()
                        .lines()
                        .map_while(Result::ok)
                        .collect()
                } else {
                    let file = fs::File::open(path).expect("Error reading language file.");
                    io::BufReader::new(file)
                        .lines()
                        .map_while(Result::ok)
                        .collect()
                };

                Some(lines.iter().map(String::from).collect())
            }
            None => {
                let lang_name = self
                    .language
                    .clone()
                    .unwrap_or_else(|| self.config().default_language);

                let bytes: Vec<u8> = self
                    .language_file
                    .as_ref()
                    .map(fs::read)
                    .and_then(Result::ok)
                    .or_else(|| fs::read(self.language_dir().join(&lang_name)).ok())
                    .or_else(|| {
                        Resources::get(&format!("language/{}", &lang_name))
                            .map(|f| f.data.into_owned())
                    })?;

                let mut rng = thread_rng();

                let mut language: Vec<&str> = str::from_utf8(&bytes)
                    .expect("Language file had non-utf8 encoding.")
                    .lines()
                    .collect();
                language.shuffle(&mut rng);

                let mut contents: Vec<_> = language
                    .into_iter()
                    .cycle()
                    .take(self.words.get())
                    .map(ToOwned::to_owned)
                    .collect();
                contents.shuffle(&mut rng);

                Some(contents)
            }
        }
    }

    /// Configuration
    fn config(&self) -> Config {
        fs::read(
            self.config
                .clone()
                .unwrap_or_else(|| self.config_dir().join("config.toml")),
        )
        .map(|bytes| {
            toml::from_str(str::from_utf8(&bytes).unwrap_or_default())
                .expect("Configuration was ill-formed.")
        })
        .unwrap_or_default()
    }

    /// Installed languages under config directory
    fn languages(&self) -> io::Result<impl Iterator<Item = OsString>> {
        let builtin = Resources::iter().filter_map(|name| {
            name.strip_prefix("language/")
                .map(ToOwned::to_owned)
                .map(OsString::from)
        });

        let configured = self
            .language_dir()
            .read_dir()
            .into_iter()
            .flatten()
            .map_while(Result::ok)
            .map(|e| e.file_name());

        Ok(builtin.chain(configured))
    }

    /// Config directory
    fn config_dir(&self) -> PathBuf {
        dirs::config_dir()
            .expect("Failed to find config directory.")
            .join("ttyper")
    }

    /// Language directory under config directory
    fn language_dir(&self) -> PathBuf {
        self.config_dir().join("language")
    }

    /// History file path
    fn history_file(&self) -> PathBuf {
        self.config_dir().join("history.csv")
    }

    /// Get the effective language name for history logging
    fn effective_language(&self) -> String {
        self.language
            .clone()
            .unwrap_or_else(|| self.config().default_language)
    }
}

enum State {
    Test(Test),
    Results(Results),
}

impl State {
    fn render_into<B: ratatui::backend::Backend>(
        &self,
        terminal: &mut Terminal<B>,
        config: &Config,
    ) -> io::Result<()> {
        match self {
            State::Test(test) => {
                terminal.draw(|f| {
                    f.render_widget(config.theme.apply_to(test), f.size());
                })?;
            }
            State::Results(results) => {
                terminal.draw(|f| {
                    f.render_widget(config.theme.apply_to(results), f.size());
                })?;
            }
        }
        Ok(())
    }
}

const WPM_PER_CPS: f64 = 12.0;

fn save_results(opt: &Opt, results: &Results) {
    let history_file = opt.history_file();
    let is_new = !history_file.exists();

    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&history_file)
    {
        if is_new {
            let _ = writeln!(file, "datetime,language,words,wpm_raw,wpm_adjusted,accuracy,correct,total,worst_keys,missed_words");
        }

        let raw_wpm = results.timing.overall_cps * WPM_PER_CPS;
        let accuracy = f64::from(results.accuracy.overall);
        let adjusted_wpm = raw_wpm * accuracy;

        let mut worst_keys: Vec<_> = results
            .accuracy
            .per_key
            .iter()
            .filter(|(key, _)| matches!(key.code, KeyCode::Char(_)))
            .map(|(key, frac)| {
                let ch = if let KeyCode::Char(c) = key.code { c } else { '?' };
                (ch, f64::from(*frac) * 100.0)
            })
            .filter(|(_, acc)| *acc < 100.0)
            .collect();
        worst_keys.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        let worst_str: String = worst_keys
            .iter()
            .take(5)
            .map(|(ch, acc)| format!("{}:{:.0}%", ch, acc))
            .collect::<Vec<_>>()
            .join(";");

        let missed_str = results.missed_words.join(";");
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

        let _ = writeln!(
            file,
            "{},{},{},{:.1},{:.1},{:.1},{},{},{},{}",
            now,
            opt.effective_language(),
            opt.words,
            raw_wpm,
            adjusted_wpm,
            accuracy * 100.0,
            results.accuracy.overall.numerator,
            results.accuracy.overall.denominator,
            worst_str,
            missed_str,
        );
    }
}

fn show_history(opt: &Opt) {
    let history_file = opt.history_file();
    if !history_file.exists() {
        println!("No history found at {}", history_file.display());
        return;
    }

    let content = fs::read_to_string(&history_file).expect("Failed to read history file");
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() <= 1 {
        println!("No results recorded yet.");
        return;
    }

    println!("{:<20} {:<15} {:>5} {:>8} {:>8} {:>8} {}",
        "Date", "Language", "Words", "Raw WPM", "Adj WPM", "Acc %", "Worst Keys");
    println!("{}", "-".repeat(90));

    for line in lines.iter().skip(1) {
        let fields: Vec<&str> = line.splitn(10, ',').collect();
        if fields.len() >= 9 {
            println!("{:<20} {:<15} {:>5} {:>8} {:>8} {:>8} {}",
                fields[0], fields[1], fields[2], fields[3], fields[4], fields[5], fields[8]);
        }
    }

    println!("\n{} results total. History file: {}", lines.len() - 1, history_file.display());
}

fn main() -> io::Result<()> {
    let opt = Opt::parse();
    if opt.debug {
        dbg!(&opt);
    }

    let config = opt.config();
    if opt.debug {
        dbg!(&config);
    }

    if opt.list_languages {
        opt.languages()
            .unwrap()
            .for_each(|name| println!("{}", name.to_str().expect("Ill-formatted language name.")));

        return Ok(());
    }

    if opt.history {
        show_history(&opt);
        return Ok(());
    }

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let contents = opt
        .gen_contents()
        .expect("Couldn't get test contents. Make sure the specified language actually exists.");

    terminal::enable_raw_mode()?;
    execute!(
        io::stdout(),
        cursor::Hide,
        cursor::SavePosition,
        terminal::EnterAlternateScreen,
    )?;
    terminal.clear()?;

    let mut state = State::Test(Test::new(contents, !opt.no_backtrack, opt.sudden_death));

    state.render_into(&mut terminal, &config)?;
    loop {
        let event = event::read()?;

        // handle exit controls
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                kind: KeyEventKind::Press,
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => break,
            Event::Key(KeyEvent {
                code: KeyCode::Esc,
                kind: KeyEventKind::Press,
                modifiers: KeyModifiers::NONE,
                ..
            }) => match state {
                State::Test(ref test) => {
                    let results = Results::from(test);
                    if !opt.no_save {
                        save_results(&opt, &results);
                    }
                    state = State::Results(results);
                }
                State::Results(_) => break,
            },
            _ => {}
        }

        match state {
            State::Test(ref mut test) => {
                if let Event::Key(key) = event {
                    test.handle_key(key);
                    if test.complete {
                        let results = Results::from(&*test);
                        if !opt.no_save {
                            save_results(&opt, &results);
                        }
                        state = State::Results(results);
                    }
                }
            }
            State::Results(ref result) => match event {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('r'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    state = State::Test(Test::new(
                        opt.gen_contents().expect(
                            "Couldn't get test contents. Make sure the specified language actually exists.",
                        ),
                        !opt.no_backtrack,
                        opt.sudden_death
                    ));
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('p'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    if result.missed_words.is_empty() {
                        continue;
                    }
                    // repeat each missed word 5 times
                    let mut practice_words: Vec<String> = (result.missed_words)
                        .iter()
                        .flat_map(|w| vec![w.clone(); 5])
                        .collect();
                    practice_words.shuffle(&mut thread_rng());
                    state = State::Test(Test::new(
                        practice_words,
                        !opt.no_backtrack,
                        opt.sudden_death,
                    ));
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => break,
                _ => {}
            },
        }

        state.render_into(&mut terminal, &config)?;
    }

    terminal::disable_raw_mode()?;
    execute!(
        io::stdout(),
        cursor::RestorePosition,
        cursor::Show,
        terminal::LeaveAlternateScreen,
    )?;

    Ok(())
}
