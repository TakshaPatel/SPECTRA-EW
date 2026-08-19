use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    let scenario_path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("scenarios/basic.yaml");
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(42);

    let yaml = fs::read_to_string(scenario_path).unwrap_or_else(|e| {
        eprintln!("Error reading scenario '{}': {}", scenario_path, e);
        std::process::exit(1);
    });

    if let Err(e) = spectra_ui::app::run_tui(&yaml, seed) {
        eprintln!("TUI error: {}", e);
        std::process::exit(1);
    }
}
