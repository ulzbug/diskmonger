use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use diskmonger_core::scanner;
use ratatui::prelude::*;
use std::{error::Error, io, panic};

mod app;
mod i18n;
mod ui;
mod utils;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(long, value_enum, default_value_t = app::RenderStyle::Nested)]
    style: app::RenderStyle,

    #[arg(short, long)]
    lang: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Initialise le système de traduction
    i18n::init(args.lang.clone());

    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal_on_panic();
        original_hook(panic_info);
    }));

    println!("{}", i18n::t("cli-scanning").replace("{path}", &args.path));
    println!("{}", i18n::t("cli-cancel-hint"));

    // Réinitialise le drapeau de scan annulé
    scanner::CANCEL_SCAN.store(false, std::sync::atomic::Ordering::SeqCst);

    let progress_state = std::sync::Arc::new(std::sync::Mutex::new(None));
    let progress_state_clone = std::sync::Arc::clone(&progress_state);
    let path_arg = args.path.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    // Lance le scan dans un thread d'arrière-plan pour ne pas bloquer l'interception clavier
    std::thread::spawn(move || {
        let res = scanner::scan_directory(&path_arg, Some(&mut |progress| {
            let mut state = progress_state_clone.lock().unwrap();
            *state = Some(progress.clone());
        }));
        let _ = tx.send(res);
    });

    // Active temporairement le mode brut pour écouter la touche Échap immédiatement
    crossterm::terminal::enable_raw_mode()?;

    use std::io::Write;
    let mut last_print = std::time::Instant::now();
    let mut total_scanned;
    let mut root_node;
    let cluster_size;

    loop {
        // 1. Vérifie si le thread de scan s'est terminé
        if let Ok(result) = rx.try_recv() {
            crossterm::terminal::disable_raw_mode()?;
            let res = result?;
            root_node = res.0;
            cluster_size = res.1;
            println!(
                "\r{}",
                i18n::t("cli-scan-complete").replace("{count}", &root_node.count_items().to_string())
            );
            break;
        }

        // 2. Vérifie si l'utilisateur appuie sur Échap
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                if key.code == crossterm::event::KeyCode::Esc {
                    crossterm::terminal::disable_raw_mode()?;
                    scanner::CANCEL_SCAN.store(true, std::sync::atomic::Ordering::SeqCst);
                    println!("\r{}", i18n::t("cli-scan-cancelled"));
                    return Ok(());
                }
            }
        }

        // 3. Affiche la progression en temps réel toutes les 200ms
        let now = std::time::Instant::now();
        if now.duration_since(last_print) >= std::time::Duration::from_millis(200) {
            let progress_opt = progress_state.lock().unwrap().clone();
            if let Some(progress) = progress_opt {
                total_scanned = progress.files + progress.dirs;
                let mut path_str = progress.path;
                if path_str.len() > 50 {
                    path_str = format!("...{}", &path_str[path_str.len() - 47..]);
                }
                let status_line = i18n::t("cli-progress")
                    .replace("{count}", &total_scanned.to_string())
                    .replace("{path}", &path_str);
                print!("\r{}\r", status_line);
                let _ = std::io::stdout().flush();
            }
            last_print = now;
        }
    }

    let mut terminal = setup_terminal()?;
    let app_result = app::run_app(&mut terminal, &mut root_node, cluster_size, &args.path, &args.style);

    restore_terminal(&mut terminal)?;

    if let Err(err) = app_result {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, Box<dyn Error>> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::event::DisableMouseCapture)?;
    Ok(())
}

fn restore_terminal_on_panic() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::event::DisableMouseCapture);
}
