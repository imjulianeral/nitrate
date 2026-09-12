mod app;
mod effects;
mod engine;
mod theme;
mod ui;
mod update;
mod util;

use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
};
use crossterm::execute;
use std::io::{self, stdout};
use std::process;

fn main() -> io::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("update") | Some("--update") => match update::run_cli_update() {
            Ok(msg) => {
                println!("{msg}");
                Ok(())
            }
            Err(e) => {
                eprintln!("UPDATE  {e}");
                process::exit(1);
            }
        },
        Some("version") | Some("--version") | Some("-V") => {
            println!("nitrate {}", update::VERSION);
            Ok(())
        }
        Some("-h") | Some("--help") | Some("help") => {
            print_help();
            Ok(())
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            print_help();
            process::exit(2);
        }
        None => run_tui(),
    }
}

fn print_help() {
    println!("nitrate {}", update::VERSION);
    println!("  nitrate              start console");
    println!("  nitrate update       install latest GitHub release");
    println!("  nitrate version      print version");
    println!();
    println!(
        "  curl -fsSL https://github.com/{}/releases/latest/download/install.sh | sh",
        update::github_repo()
    );
}

fn run_tui() -> io::Result<()> {
    let mut app = app::App::new();
    app.start_update_check();
    let mut terminal = setup()?;
    let result = app::run(&mut terminal, &mut app);
    let apply_update = app.apply_update;
    app.cancel_job();
    teardown();
    result?;
    if apply_update {
        match update::run_cli_update() {
            Ok(msg) => println!("{msg}"),
            Err(e) => {
                eprintln!("UPDATE  {e}");
                process::exit(1);
            }
        }
    }
    Ok(())
}

fn setup() -> io::Result<ratatui::DefaultTerminal> {
    let terminal = ratatui::try_init()?;
    execute!(stdout(), EnableMouseCapture, EnableBracketedPaste)?;
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(stdout(), DisableMouseCapture, DisableBracketedPaste);
        original(info);
    }));
    Ok(terminal)
}

fn teardown() {
    let _ = execute!(stdout(), DisableMouseCapture, DisableBracketedPaste);
    ratatui::restore();
}
