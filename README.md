# ttyper (fork)

[![Upstream](https://img.shields.io/badge/upstream-max--niederman%2Fttyper-blue)](https://github.com/max-niederman/ttyper)
[![License](https://img.shields.io/crates/l/ttyper)](./LICENSE.md)

This is an actively maintained fork of [max-niederman/ttyper](https://github.com/max-niederman/ttyper), a terminal-based typing test built with Rust and Ratatui.

![Recording](./resources/recording.gif)

## Fork Changes

This fork adds the following features on top of upstream ttyper 1.6.0:

- **History tracking**: Results are automatically saved to a CSV file after each test (`--no-save` to disable, `--history` to view past results)
- **Bug fixes**: Addressing open upstream issues (see [issues](https://github.com/ptrsauer/ttyper/issues))

### History

Every completed test appends a row to `TTYPER_CONFIG_DIR/history.csv` with:

| Field | Description |
|-------|-------------|
| `datetime` | Timestamp of the test |
| `language` | Language/word list used |
| `words` | Number of words in the test |
| `wpm_raw` | Raw words per minute |
| `wpm_adjusted` | WPM adjusted for errors |
| `accuracy` | Overall accuracy percentage |
| `correct` | Number of correct keystrokes |
| `total` | Total keystrokes |
| `worst_keys` | Up to 5 worst keys with accuracy (e.g. `y:50%;A:75%`) |
| `missed_words` | Words with errors |

```bash
# View history
ttyper --history

# Run without saving
ttyper --no-save
```

## Upstream

The original project by [Max Niederman](https://github.com/max-niederman) can be found at [max-niederman/ttyper](https://github.com/max-niederman/ttyper).

## installation

### cargo (from GitHub)

```bash
cargo install --git https://github.com/ptrsauer/ttyper.git
```

### from source

```bash
git clone https://github.com/ptrsauer/ttyper.git
cd ttyper
cargo install --path .
```

## usage

For usage instructions, you can run `ttyper --help`:

```
Terminal-based typing test.

Usage: ttyper [OPTIONS] [PATH]

Arguments:
  [PATH]  Read test contents from the specified file, or "-" for stdin

Options:
  -w, --words <N>             Specify word count [default: 50]
  -c, --config <PATH>         Use config file
      --language-file <PATH>  Specify test language in file
  -l, --language <LANG>       Specify test language
      --list-languages        List installed languages
      --no-backtrack          Disable backtracking to completed words
      --sudden-death          Enable sudden death mode to restart on first error
      --history               Show history of past results
      --last <N>              Show only the last N history entries
      --history-lang <LANG>   Filter history by language
      --since <DATE>          Filter history from date (YYYY-MM-DD)
      --until <DATE>          Filter history until date (YYYY-MM-DD)
      --stats                 Show aggregated statistics
      --no-save               Disable saving results to history
  -h, --help                  Print help
  -V, --version               Print version
```

### examples

| command                        |                             test contents |
| :----------------------------- | ----------------------------------------: |
| `ttyper`                       |   50 of the 200 most common english words |
| `ttyper -w 100`                |  100 of the 200 most common English words |
| `ttyper -w 100 -l english1000` | 100 of the 1000 most common English words |
| `ttyper --language-file lang`  |      50 random words from the file `lang` |
| `ttyper text.txt`              |  contents of `text.txt` split at newlines |

## languages

The following languages are available by default:

| name                 |                         description |
| :------------------- | ----------------------------------: |
| `c`                  |          The C programming language |
| `csharp`             |         The C# programming language |
| `english100`         |       100 most common English words |
| `english200`         |       200 most common English words |
| `english1000`        |      1000 most common English words |
| `english-advanced`   |              Advanced English words |
| `english-pirate`     |       50 pirate speak English words |
| `french100`          |        100 most common French words |
| `french200`          |        200 most common French words |
| `french1000`         |       1000 most common French words |
| `german`             |        207 most common German words |
| `german1000`         |       1000 most common German words |
| `german10000`        |      10000 most common German words |
| `go`                 |         The Go programming language |
| `html`               |           HyperText Markup Language |
| `java`               |       The Java programming language |
| `javascript`         | The Javascript programming language |
| `norwegian`          |     200 most common Norwegian words |
| `php`                |        The PHP programming language |
| `portuguese`         |    100 most common Portuguese words |
| `portuguese200`      |    200 most common Portuguese words |
| `portuguese1000`     |   1000 most common Portuguese words |
| `portuguese-advanced`|           Advanced Portuguese words |
| `python`             |     The Python programming language |
| `qt`                 |                The QT GUI framework |
| `ruby`               |       The Ruby programming language |
| `rust`               |       The Rust programming language |
| `spanish`            |       100 most common Spanish words |
| `ukrainian`          |     100 most common Ukrainian words |

Additional languages can be added by creating a file in `TTYPER_CONFIG_DIR/language` with a word on each line. On Linux, the config directory is `$HOME/.config/ttyper`; on Windows, it's `C:\Users\user\AppData\Roaming\ttyper`; and on macOS it's `$HOME/Library/Application Support/ttyper`.

## config

Configuration is specified by the `config.toml` file in the config directory (e.g. `$HOME/.config/ttyper/config.toml`).

The default values with explanations are below:

```toml
# the language used when one is not manually specified
default_language = "english200"

[theme]
# default style (this includes empty cells)
default = "none"

# title text styling
title = "white;bold"

## test styles ##

# input box border
input_border = "cyan"
# prompt box border
prompt_border = "green"

# border type
border_type = "rounded"

# correctly typed words
prompt_correct = "green"
# incorrectly typed words
prompt_incorrect = "red"
# untyped words
prompt_untyped = "gray"

# correctly typed letters in current word
prompt_current_correct = "green;bold"
# incorrectly typed letters in current word
prompt_current_incorrect = "red;bold"
# untyped letters in current word
prompt_current_untyped = "blue;bold"

# cursor character
prompt_cursor = "none;underlined"

## results styles ##

# overview text
results_overview = "cyan;bold"
# overview border
results_overview_border = "cyan"

# worst keys text
results_worst_keys = "cyan;bold"
# worst keys border
results_worst_keys_border = "cyan"

# results chart default (includes plotted data)
results_chart = "cyan"
# results chart x-axis label
results_chart_x = "cyan"
# results chart y-axis label
results_chart_y = "gray;italic"

# restart/quit prompt in results ui
results_restart_prompt = "gray;italic"
```

### style format

The configuration uses a custom style format which can specify most [ANSI escape styling codes](<https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters>), encoded as a string.

Styles begin with the color specification, which can be a single color (the foreground), or two colors seperated by a colon (the foreground and background). Colors can be one of sixteen specified by your terminal, a 24-bit hex color code, `none`, or `reset`.

After the colors, you can optionally specify modifiers seperated by a semicolon. A list of modifiers is below:

- `bold`
- `crossed_out`
- `dim`
- `hidden`
- `italic`
- `rapid_blink`
- `slow_blink`
- `reversed`
- `underlined`

Some examples:

- `blue:white;italic` specifies italic blue text on a white background.
- `none;italic;bold;underlined` specifies underlined, italicized, and bolded text with no set color or background.
- `00ff00:000000` specifies text of color `#00ff00` (pure green) on a background of `#000000` (pure black).

In [extended Backus-Naur form](https://en.wikipedia.org/wiki/Extended_Backus%E2%80%93Naur_form):

```ebnf
style     = colors, { ";", modifier }, [ ";" ] ;

colors    = color, [ ":", color ] ;
color     = "none"
          | "reset"
          | "black"
          | "white"
          | "red"
          | "green"
          | "yellow"
          | "blue"
          | "magenta"
          | "cyan"
          | "gray"
          | "darkgray"
          | "lightred"
          | "lightgreen"
          | "lightyellow"
          | "lightblue"
          | "lightmagenta"
          | "lightcyan"
          | 6 * hex digit ;
hex digit = ? hexadecimal digit; 1-9, a-z, and A-Z ? ;

modifier  = "bold"
          | "crossed_out"
          | "dim"
          | "hidden"
          | "italic"
          | "rapid_blink"
          | "slow_blink"
          | "reversed"
          | "underlined" ;
```

### border types

The following border types are supported in the config file.

- `plain`
- `rounded` (default)
- `double`
- `thick`
- `quadrantinside`
- `quadrantoutside`

If you're familiar with [serde](https://serde.rs), you can also read [the deserialization code](./src/config.rs).
