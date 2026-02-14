mod config;
mod history;
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
    io::{self, BufRead},
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

    /// Show only the last N history entries
    #[arg(long, value_name = "N")]
    last: Option<usize>,

    /// Disable saving results to history
    #[arg(long)]
    no_save: bool,
}

impl Opt {
    fn gen_contents(&self) -> Result<Vec<String>, String> {
        match &self.contents {
            Some(path) => {
                let lines: Vec<String> = if path.as_os_str() == "-" {
                    std::io::stdin()
                        .lock()
                        .lines()
                        .map_while(Result::ok)
                        .collect()
                } else {
                    let file = fs::File::open(path).map_err(|e| {
                        format!("Error: Cannot read '{}': {}", path.display(), e)
                    })?;
                    io::BufReader::new(file)
                        .lines()
                        .map_while(Result::ok)
                        .collect()
                };

                Ok(lines.iter().map(String::from).collect())
            }
            None => {
                let lang_name = self
                    .language
                    .clone()
                    .unwrap_or_else(|| self.config().default_language);

                let bytes: Vec<u8> = if let Some(lang_file) = &self.language_file {
                    fs::read(lang_file).map_err(|e| {
                        format!(
                            "Error: Cannot read language file '{}': {}",
                            lang_file.display(),
                            e
                        )
                    })?
                } else {
                    fs::read(self.language_dir().join(&lang_name))
                        .ok()
                        .or_else(|| {
                            Resources::get(&format!("language/{}", &lang_name))
                                .map(|f| f.data.into_owned())
                        })
                        .ok_or_else(|| {
                            format!(
                                "Error: Language '{}' not found. Use --list-languages to see available languages.",
                                lang_name
                            )
                        })?
                };

                let mut rng = thread_rng();

                let mut language: Vec<&str> = str::from_utf8(&bytes)
                    .map_err(|_| {
                        if let Some(lang_file) = &self.language_file {
                            format!(
                                "Error: Language file '{}' has invalid UTF-8 encoding.",
                                lang_file.display()
                            )
                        } else {
                            format!("Error: Language '{}' has invalid UTF-8 encoding.", lang_name)
                        }
                    })?
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

                Ok(contents)
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
        history::show_history(&opt.history_file(), opt.last);
        return Ok(());
    }

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let contents = match opt.gen_contents() {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{}", msg);
            return Ok(());
        }
    };

    if contents.is_empty() {
        eprintln!("Error: No words to type. The word list is empty.");
        return Ok(());
    }

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
                        history::save_results(&opt.history_file(), &opt.effective_language(), opt.words.get(), &results);
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
                    // TAB → restart with new words (no save)
                    if key.code == KeyCode::Tab && key.kind == KeyEventKind::Press {
                        match opt.gen_contents() {
                            Ok(contents) if !contents.is_empty() => {
                                state = State::Test(Test::new(
                                    contents,
                                    !opt.no_backtrack,
                                    opt.sudden_death,
                                ));
                            }
                            _ => continue,
                        }
                    } else {
                        test.handle_key(key);
                        if test.complete {
                            let results = Results::from(&*test);
                            if !opt.no_save {
                                history::save_results(&opt.history_file(), &opt.effective_language(), opt.words.get(), &results);
                            }
                            state = State::Results(results);
                        }
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
                    match opt.gen_contents() {
                        Ok(contents) if !contents.is_empty() => {
                            state = State::Test(Test::new(
                                contents,
                                !opt.no_backtrack,
                                opt.sudden_death,
                            ));
                        }
                        _ => continue,
                    }
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
                    code: KeyCode::Char('t'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    if result.words.is_empty() {
                        continue;
                    }
                    state = State::Test(Test::new(
                        result.words.clone(),
                        !opt.no_backtrack,
                        opt.sudden_death,
                    ));
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s'),
                    kind: KeyEventKind::Press,
                    modifiers: KeyModifiers::NONE,
                    ..
                }) => {
                    if result.slow_words.is_empty() {
                        continue;
                    }
                    let mut practice_words: Vec<String> = result
                        .slow_words
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
