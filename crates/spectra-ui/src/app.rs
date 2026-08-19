use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::prelude::*;
use std::io;
use std::time::Duration;

use spectra_core::observation::ObservationQuality;
use spectra_experiments::runner::{ExperimentConfig, ExperimentRunner};

/// Application state for the TUI.
pub struct App {
    pub runner: ExperimentRunner,
    pub running: bool,
    pub tick_delay: u64,
    pub last_confidence: f64,
    pub observation_log: Vec<ObsEntry>,
    pub error_log: Vec<String>,
    pub ew_log: Vec<String>,
    pub show_help: bool,
}

#[derive(Clone)]
pub struct ObsEntry {
    pub tick: u64,
    pub quality: String,
    pub confidence: f64,
    pub channel: Option<u32>,
    pub freq: Option<f64>,
}

impl App {
    pub fn from_scenario(yaml: &str, seed: u64) -> Result<Self, String> {
        let config = ExperimentConfig {
            name: "TUI Session".to_string(),
            seed,
            ..Default::default()
        };
        let runner = ExperimentRunner::from_scenario(yaml, config)?;
        Ok(Self {
            runner,
            running: false,
            tick_delay: 100,
            last_confidence: 0.0,
            observation_log: Vec::new(),
            error_log: Vec::new(),
            ew_log: Vec::new(),
            show_help: false,
        })
    }

    pub fn tick(&mut self) {
        self.runner.tick();
        let tick = self.runner.world.tick.saturating_sub(1);

        // Record observations
        for obs in &self.runner.last_observations {
            if obs.is_usable() {
                let quality = match obs.quality {
                    ObservationQuality::Clear => "CLR".to_string(),
                    ObservationQuality::Noisy => "NOS".to_string(),
                    ObservationQuality::Degraded => "DGR".to_string(),
                    ObservationQuality::Unreliable => "UNR".to_string(),
                };
                self.observation_log.push(ObsEntry {
                    tick,
                    quality,
                    confidence: obs.confidence,
                    channel: obs.estimated_channel,
                    freq: obs.estimated_frequency_mhz,
                });
            }
        }
        // Keep last 50 observations
        if self.observation_log.len() > 50 {
            self.observation_log
                .drain(0..self.observation_log.len() - 50);
        }

        // Record AI decision
        if let Some(decision) = self.runner.ai.errors().last() {
            if decision.tick() == tick {
                self.error_log.push(format!("{}", decision));
            }
        }
        if self.error_log.len() > 20 {
            self.error_log.drain(0..self.error_log.len() - 20);
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
            }
            KeyCode::Char(' ') | KeyCode::Char('n') => {
                self.tick();
            }
            KeyCode::Char('r') => {
                self.running = !self.running;
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.tick_delay = self.tick_delay.saturating_sub(20).max(20);
            }
            KeyCode::Char('-') => {
                self.tick_delay = self.tick_delay.saturating_add(20).min(500);
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                self.show_help = !self.show_help;
            }
            _ => {}
        }
    }
}

/// Run the TUI event loop.
pub fn run_tui(yaml: &str, seed: u64) -> Result<(), String> {
    let mut app = App::from_scenario(yaml, seed)?;

    // Setup terminal
    crossterm::terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)
        .map_err(|e| e.to_string())?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout)).map_err(|e| e.to_string())?;

    loop {
        // Draw UI
        terminal
            .draw(|f| crate::ui::render(f, &app))
            .map_err(|e| e.to_string())?;

        // Handle input
        if event::poll(Duration::from_millis(app.tick_delay)).map_err(|e| e.to_string())? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code);
                    if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                        break;
                    }
                }
            }
        }

        // Auto-step if running
        if app.running && !app.runner.world.is_complete() {
            app.tick();
        }
    }

    // Restore terminal
    crossterm::terminal::disable_raw_mode().map_err(|e| e.to_string())?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )
    .map_err(|e| e.to_string())?;
    terminal.show_cursor().map_err(|e| e.to_string())?;

    Ok(())
}
