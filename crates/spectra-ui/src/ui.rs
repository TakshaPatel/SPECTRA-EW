use ratatui::prelude::*;
use ratatui::widgets::*;

use spectra_core::emitter::EmitterState;
use spectra_core::observation::ObservationQuality;
use spectra_core::world::Faction;

use crate::app::App;

/// Color palette for the TUI.
mod colors {
    use ratatui::style::Color;

    // Blues (friendly)
    pub const BLUE_BRIGHT: Color = Color::Rgb(100, 180, 255);
    pub const BLUE_DIM: Color = Color::Rgb(50, 100, 180);

    // Reds (hostile)
    pub const RED_BRIGHT: Color = Color::Rgb(255, 100, 100);
    pub const RED_DIM: Color = Color::Rgb(180, 50, 50);

    // Greens (good/active)
    pub const GREEN_BRIGHT: Color = Color::Rgb(100, 255, 150);
    pub const GREEN_DIM: Color = Color::Rgb(50, 180, 80);

    // Yellows (warnings/caution)
    pub const YELLOW_BRIGHT: Color = Color::Rgb(255, 230, 80);
    pub const YELLOW_DIM: Color = Color::Rgb(200, 180, 50);

    // Cyans (info/sensors)
    pub const CYAN_BRIGHT: Color = Color::Rgb(80, 240, 255);
    pub const CYAN_DIM: Color = Color::Rgb(40, 150, 180);

    // Magenta (EW/interference)
    pub const MAGENTA_BRIGHT: Color = Color::Rgb(255, 100, 255);
    pub const MAGENTA_DIM: Color = Color::Rgb(180, 50, 180);

    // Neutrals
    pub const GRAY: Color = Color::Rgb(120, 120, 120);
    pub const DARK: Color = Color::Rgb(30, 30, 40);
}

/// Main render function.
pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Main layout: top bar + content + bottom bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status bar
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Controls bar
        ])
        .split(area);

    // Render status bar
    render_status_bar(f, app, chunks[0]);

    // Content: left (map) + right (panels)
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55), // World map
            Constraint::Percentage(45), // Side panels
        ])
        .split(chunks[1]);

    // World map
    render_world_map(f, app, content[0]);

    // Side panels
    let panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35), // Observations
            Constraint::Percentage(30), // AI
            Constraint::Percentage(35), // EW + Metrics
        ])
        .split(content[1]);

    render_observations_panel(f, app, panels[0]);
    render_ai_panel(f, app, panels[1]);
    render_ew_panel(f, app, panels[2]);

    // Controls bar
    render_controls_bar(f, app, chunks[2]);

    // Help overlay
    if app.show_help {
        render_help_overlay(f, app);
    }
}

/// Top status bar with tick info and faction status.
fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let tick = app.runner.world.tick;
    let max_ticks = app.runner.world.config.max_ticks;
    let observations = app.runner.metrics.total_observations;
    let detected = app.runner.metrics.signals_detected;
    let avg_conf = app.runner.metrics.avg_confidence;

    // Count active entities per faction
    let blue_active = app
        .runner
        .world
        .entities
        .iter()
        .filter(|e| e.faction == Faction::Blue && e.is_active())
        .count();
    let red_active = app
        .runner
        .world
        .entities
        .iter()
        .filter(|e| e.faction == Faction::Red && e.is_active())
        .count();
    let blue_total = app
        .runner
        .world
        .entities
        .iter()
        .filter(|e| e.faction == Faction::Blue)
        .count();
    let red_total = app
        .runner
        .world
        .entities
        .iter()
        .filter(|e| e.faction == Faction::Red)
        .count();

    let progress = if max_ticks > 0 {
        tick as f64 / max_ticks as f64
    } else {
        0.0
    };

    let status_line = Line::from(vec![
        Span::styled(
            "  SPECTRA ",
            Style::default()
                .fg(colors::CYAN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(colors::GRAY)),
        Span::styled(
            format!(" TICK {}/{} ", tick, max_ticks),
            Style::default()
                .fg(colors::YELLOW_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(colors::GRAY)),
        Span::styled(
            format!(" Blue: {}/{} ", blue_active, blue_total),
            Style::default().fg(colors::BLUE_BRIGHT),
        ),
        Span::styled("│", Style::default().fg(colors::GRAY)),
        Span::styled(
            format!(" Red: {}/{} ", red_active, red_total),
            Style::default().fg(colors::RED_BRIGHT),
        ),
        Span::styled("│", Style::default().fg(colors::GRAY)),
        Span::styled(
            format!(" Obs: {} ", observations),
            Style::default().fg(colors::GREEN_BRIGHT),
        ),
        Span::styled("│", Style::default().fg(colors::GRAY)),
        Span::styled(
            format!(" Detected: {} ", detected),
            Style::default().fg(colors::CYAN_BRIGHT),
        ),
        Span::styled("│", Style::default().fg(colors::GRAY)),
        Span::styled(
            format!(" Confidence: {:.0}% ", avg_conf * 100.0),
            Style::default().fg(colors::YELLOW_BRIGHT),
        ),
    ]);

    let progress_bar = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors::CYAN_DIM))
                .title(" Progress "),
        )
        .gauge_style(Style::default().fg(colors::CYAN_BRIGHT).bg(colors::DARK))
        .ratio(progress);

    // Combine into a two-line status
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::CYAN_DIM))
        .title(Span::styled(
            " SIMULATION STATUS ",
            Style::default()
                .fg(colors::CYAN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = status_block.inner(area);
    f.render_widget(status_block, area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner_area);

    f.render_widget(Paragraph::new(status_line), inner_chunks[0]);
    f.render_widget(progress_bar, inner_chunks[1]);
}

/// Render the world map — ASCII grid with colored entities.
fn render_world_map(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::GREEN_DIM))
        .title(Span::styled(
            " WORLD MAP ",
            Style::default()
                .fg(colors::GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 5 || inner.height < 5 {
        return;
    }

    let map_w = app.runner.world.config.map_width;
    let map_h = app.runner.world.config.map_height;
    let cols = inner.width as f64;
    let rows = inner.height as f64;

    // Build a grid
    let mut grid: Vec<Vec<char>> = vec![vec![' '; cols as usize]; rows as usize];
    let mut colors_grid: Vec<Vec<Color>> = vec![vec![colors::DARK; cols as usize]; rows as usize];

    // Draw interference zones first (background)
    for ix in &app.runner.world.active_interference {
        let cx = ((ix.affected_center.x / map_w) * cols) as usize;
        let cy = ((ix.affected_center.y / map_h) * rows) as usize;
        let r = ((ix.affected_radius / map_w.min(map_h)) * cols.min(rows)) as usize;

        for dy in 0..r {
            for dx in 0..r {
                if dx * dx + dy * dy <= r * r {
                    let px = cx.saturating_add(dx).min(cols as usize - 1);
                    let py = cy.saturating_add(dy).min(rows as usize - 1);
                    let px2 = cx.saturating_sub(dx).min(cols as usize - 1);
                    let py2 = cy.saturating_sub(dy).min(rows as usize - 1);
                    // Draw interference as dim magenta dots
                    if py < rows as usize && px < cols as usize {
                        colors_grid[py][px] = colors::MAGENTA_DIM;
                        grid[py][px] = '░';
                    }
                    if py2 < rows as usize && px2 < cols as usize {
                        colors_grid[py2][px2] = colors::MAGENTA_DIM;
                        grid[py2][px2] = '░';
                    }
                }
            }
        }
    }

    // Draw emitters
    for em in &app.runner.world.emitters {
        if em.state == EmitterState::Transmitting {
            let x = ((em.position.x / map_w) * cols) as usize;
            let y = ((em.position.y / map_h) * rows) as usize;
            if x < cols as usize && y < rows as usize {
                // Check if owned by blue or red
                let owner = app.runner.world.entity_by_id(em.owner_id);
                let color = match owner.map(|e| e.faction) {
                    Some(Faction::Blue) => colors::BLUE_BRIGHT,
                    Some(Faction::Red) => colors::RED_BRIGHT,
                    _ => colors::YELLOW_BRIGHT,
                };
                grid[y][x] = '◆';
                colors_grid[y][x] = color;
            }
        }
    }

    // Draw entities (overwrite emitters if at same position)
    for ent in &app.runner.world.entities {
        let x = ((ent.position.x / map_w) * cols) as usize;
        let y = ((ent.position.y / map_h) * rows) as usize;
        if x < cols as usize && y < rows as usize {
            let (ch, color) = match (ent.faction, ent.is_active()) {
                (Faction::Blue, true) => ('▲', colors::BLUE_BRIGHT),
                (Faction::Blue, false) => ('△', colors::BLUE_DIM),
                (Faction::Red, true) => ('▼', colors::RED_BRIGHT),
                (Faction::Red, false) => ('▽', colors::RED_DIM),
                (Faction::Neutral, true) => ('●', colors::YELLOW_BRIGHT),
                (Faction::Neutral, false) => ('○', colors::YELLOW_DIM),
            };
            grid[y][x] = ch;
            colors_grid[y][x] = color;

            // Draw entity label if space permits
            if !ent.label.is_empty() && x + 2 < cols as usize {
                for (i, c) in ent.label.chars().enumerate() {
                    let px = x + 1 + i;
                    if px < cols as usize {
                        grid[y][px] = c;
                        colors_grid[y][px] = color;
                    }
                }
            }
        }
    }

    // Draw receiver positions as small circles
    for rx in &app.runner.world.receivers {
        let x = ((rx.position.x / map_w) * cols) as usize;
        let y = ((rx.position.y / map_h) * rows) as usize;
        if x < cols as usize && y < rows as usize && grid[y][x] == ' ' {
            grid[y][x] = '◎';
            colors_grid[y][x] = colors::CYAN_DIM;
        }
    }

    // Render the grid
    let mut lines = Vec::new();
    for (row_idx, row) in grid.iter().enumerate() {
        let mut spans = Vec::new();
        for (col_idx, &ch) in row.iter().enumerate() {
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(colors_grid[row_idx][col_idx]),
            ));
        }
        lines.push(Line::from(spans));
    }

    let map_widget = Paragraph::new(lines);
    f.render_widget(map_widget, inner);
}

/// Render observations panel.
fn render_observations_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::CYAN_DIM))
        .title(Span::styled(
            " SENSOR OBSERVATIONS ",
            Style::default()
                .fg(colors::CYAN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();

    // Header
    lines.push(Line::from(vec![Span::styled(
        " TCK  QLF  CONF   CH   FREQ",
        Style::default()
            .fg(colors::GRAY)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(Span::styled(
        "─────────────────────────────",
        Style::default().fg(colors::GRAY),
    )));

    // Observation entries
    for obs in app.observation_log.iter().rev().take(20) {
        let quality_color = match obs.quality.as_str() {
            "CLR" => colors::GREEN_BRIGHT,
            "NOS" => colors::YELLOW_BRIGHT,
            "DGR" => colors::MAGENTA_BRIGHT,
            _ => colors::RED_BRIGHT,
        };

        let conf_color = if obs.confidence > 0.7 {
            colors::GREEN_BRIGHT
        } else if obs.confidence > 0.4 {
            colors::YELLOW_BRIGHT
        } else {
            colors::RED_BRIGHT
        };

        let ch_str = obs
            .channel
            .map_or("---".to_string(), |c| format!("{:3}", c));
        let freq_str = obs
            .freq
            .map_or("------".to_string(), |f| format!("{:.1}", f));

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {:4} ", obs.tick),
                Style::default().fg(colors::GRAY),
            ),
            Span::styled(
                format!(" {} ", obs.quality),
                Style::default()
                    .fg(quality_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {:5.0}% ", obs.confidence * 100.0),
                Style::default().fg(conf_color),
            ),
            Span::styled(
                format!(" {} ", ch_str),
                Style::default().fg(colors::CYAN_BRIGHT),
            ),
            Span::styled(
                format!(" {} ", freq_str),
                Style::default().fg(colors::CYAN_DIM),
            ),
        ]));
    }

    if app.observation_log.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Waiting for observations...",
            Style::default().fg(colors::GRAY),
        )));
    }

    let widget = Paragraph::new(lines);
    f.render_widget(widget, inner);
}

/// Render AI panel.
fn render_ai_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::YELLOW_DIM))
        .title(Span::styled(
            " AI DECISION ENGINE ",
            Style::default()
                .fg(colors::YELLOW_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Current decision
            Constraint::Length(3), // Error rate
            Constraint::Min(3),    // Error log
        ])
        .split(inner);

    // Current decision
    let last_obs = app.runner.last_observations.iter().find(|o| o.is_usable());
    let (action_str, action_color) = if let Some(obs) = last_obs {
        let action = if obs.confidence < 0.3 {
            "OBSERVE"
        } else if obs.confidence < 0.6 {
            if obs.quality == ObservationQuality::Noisy
                || obs.quality == ObservationQuality::Degraded
            {
                "PROTECT_CHANNEL"
            } else {
                "MONITOR"
            }
        } else {
            match obs.quality {
                ObservationQuality::Clear => "SUPPRESS_SIGNAL",
                ObservationQuality::Noisy => "PROTECT_CHANNEL",
                ObservationQuality::Degraded => "CHANGE_CHANNEL",
                _ => "OBSERVE",
            }
        };
        let color = match action {
            "SUPPRESS_SIGNAL" => colors::RED_BRIGHT,
            "PROTECT_CHANNEL" => colors::MAGENTA_BRIGHT,
            "CHANGE_CHANNEL" => colors::YELLOW_BRIGHT,
            "DEPLOY_DECOY" => colors::CYAN_BRIGHT,
            "MONITOR" => colors::GREEN_BRIGHT,
            _ => colors::GRAY,
        };
        (action.to_string(), color)
    } else {
        ("OBSERVE".to_string(), colors::GRAY)
    };

    let confidence = last_obs.map_or(0.0, |o| o.confidence);
    let conf_bar_len = 20;
    let filled = (confidence * conf_bar_len as f64) as usize;
    let conf_bar: String = "█".repeat(filled) + &"░".repeat(conf_bar_len - filled);

    let decision_lines = vec![
        Line::from(vec![
            Span::styled("  Decision: ", Style::default().fg(colors::GRAY)),
            Span::styled(
                &action_str,
                Style::default()
                    .fg(action_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Confidence: ", Style::default().fg(colors::GRAY)),
            Span::styled(
                format!("{} {:.0}%", conf_bar, confidence * 100.0),
                Style::default().fg(action_color),
            ),
        ]),
    ];

    f.render_widget(Paragraph::new(decision_lines), chunks[0]);

    // Error rate
    let error_rate = app.runner.experiment_metrics.ai_error_rate();
    let decisions = app.runner.experiment_metrics.ai_decisions;
    let errors = app.runner.experiment_metrics.ai_errors;

    let error_color = if error_rate > 0.3 {
        colors::RED_BRIGHT
    } else if error_rate > 0.1 {
        colors::YELLOW_BRIGHT
    } else {
        colors::GREEN_BRIGHT
    };

    let error_lines = vec![Line::from(vec![
        Span::styled("  Errors: ", Style::default().fg(colors::GRAY)),
        Span::styled(
            format!("{}/{} ({:.0}%)", errors, decisions, error_rate * 100.0),
            Style::default().fg(error_color),
        ),
    ])];
    f.render_widget(Paragraph::new(error_lines), chunks[1]);

    // Error log
    let mut error_log_lines = vec![Line::from(Span::styled(
        "  Recent Errors:",
        Style::default()
            .fg(colors::GRAY)
            .add_modifier(Modifier::BOLD),
    ))];
    for err in app.error_log.iter().rev().take(5) {
        error_log_lines.push(Line::from(Span::styled(
            format!("    {}", err),
            Style::default().fg(colors::RED_DIM),
        )));
    }
    if app.error_log.is_empty() {
        error_log_lines.push(Line::from(Span::styled(
            "    (none)",
            Style::default().fg(colors::GREEN_DIM),
        )));
    }
    f.render_widget(Paragraph::new(error_log_lines), chunks[2]);
}

/// Render EW + Metrics panel.
fn render_ew_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::MAGENTA_DIM))
        .title(Span::styled(
            " EW EFFECTS & METRICS ",
            Style::default()
                .fg(colors::MAGENTA_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Active effects
            Constraint::Min(4),    // Metrics
        ])
        .split(inner);

    // Active EW effects
    let mut ew_lines = vec![Line::from(Span::styled(
        "  Active Effects:",
        Style::default()
            .fg(colors::GRAY)
            .add_modifier(Modifier::BOLD),
    ))];

    let active_count = app.runner.ew_manager.active_count(app.runner.world.tick);
    if active_count == 0 {
        ew_lines.push(Line::from(Span::styled(
            "    (none active)",
            Style::default().fg(colors::GRAY),
        )));
    } else {
        for name in app.runner.ew_manager.effect_names() {
            ew_lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled("●", Style::default().fg(colors::MAGENTA_BRIGHT)),
                Span::styled(
                    format!(" {} ", name),
                    Style::default().fg(colors::MAGENTA_BRIGHT),
                ),
            ]));
        }
    }

    let interference_count = app.runner.world.active_interference.len();
    ew_lines.push(Line::from(vec![
        Span::styled("  Interference zones: ", Style::default().fg(colors::GRAY)),
        Span::styled(
            format!("{}", interference_count),
            Style::default()
                .fg(colors::MAGENTA_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    f.render_widget(Paragraph::new(ew_lines), chunks[0]);

    // Metrics
    let m = &app.runner.experiment_metrics;
    let metrics_lines = vec![
        Line::from(Span::styled(
            "  Experiment Metrics",
            Style::default()
                .fg(colors::CYAN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("  Ticks:       ", Style::default().fg(colors::GRAY)),
            Span::styled(
                format!("{}", m.total_ticks),
                Style::default().fg(colors::CYAN_BRIGHT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Decisions:    ", Style::default().fg(colors::GRAY)),
            Span::styled(
                format!("{}", m.ai_decisions),
                Style::default().fg(colors::YELLOW_BRIGHT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  EW Deployed:  ", Style::default().fg(colors::GRAY)),
            Span::styled(
                format!("{}", m.ew_effects_deployed),
                Style::default().fg(colors::MAGENTA_BRIGHT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Class. Acc:   ", Style::default().fg(colors::GRAY)),
            Span::styled(
                format!("{:.0}%", m.classification_accuracy() * 100.0),
                Style::default().fg(colors::GREEN_BRIGHT),
            ),
        ]),
    ];

    f.render_widget(Paragraph::new(metrics_lines), chunks[1]);
}

/// Bottom controls bar.
fn render_controls_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = if app.running {
        Span::styled(
            " RUNNING ",
            Style::default()
                .fg(colors::DARK)
                .bg(colors::GREEN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        )
    } else if app.runner.world.is_complete() {
        Span::styled(
            " COMPLETE ",
            Style::default()
                .fg(colors::DARK)
                .bg(colors::YELLOW_BRIGHT)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " PAUSED ",
            Style::default()
                .fg(colors::DARK)
                .bg(colors::RED_BRIGHT)
                .add_modifier(Modifier::BOLD),
        )
    };

    let controls = Line::from(vec![
        status,
        Span::styled(" │ ", Style::default().fg(colors::GRAY)),
        Span::styled("[Space]", Style::default().fg(colors::YELLOW_BRIGHT)),
        Span::styled(" Step ", Style::default().fg(colors::GRAY)),
        Span::styled("[R]", Style::default().fg(colors::GREEN_BRIGHT)),
        Span::styled(" Run/Pause ", Style::default().fg(colors::GRAY)),
        Span::styled("[+/-]", Style::default().fg(colors::CYAN_BRIGHT)),
        Span::styled(" Speed ", Style::default().fg(colors::GRAY)),
        Span::styled("[?]", Style::default().fg(colors::MAGENTA_BRIGHT)),
        Span::styled(" Help ", Style::default().fg(colors::GRAY)),
        Span::styled("[Q]", Style::default().fg(colors::RED_BRIGHT)),
        Span::styled(" Quit ", Style::default().fg(colors::GRAY)),
    ]);

    let bar = Paragraph::new(controls).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::GRAY))
            .title(" Controls "),
    );
    f.render_widget(bar, area);
}

/// Help overlay.
fn render_help_overlay(f: &mut Frame, _app: &App) {
    let area = f.area();
    let popup_area = centered_rect(50, 60, area);

    let help_text = vec![
        Line::from(Span::styled(
            " SPECTRA TUI Controls ",
            Style::default()
                .fg(colors::CYAN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Space / N  ", Style::default().fg(colors::YELLOW_BRIGHT)),
            Span::styled("  Step one tick", Style::default().fg(colors::GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  R          ", Style::default().fg(colors::GREEN_BRIGHT)),
            Span::styled("  Toggle auto-run", Style::default().fg(colors::GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  + / -      ", Style::default().fg(colors::CYAN_BRIGHT)),
            Span::styled("  Adjust speed", Style::default().fg(colors::GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  ? / H      ", Style::default().fg(colors::MAGENTA_BRIGHT)),
            Span::styled("  Toggle this help", Style::default().fg(colors::GRAY)),
        ]),
        Line::from(vec![
            Span::styled("  Q / Esc    ", Style::default().fg(colors::RED_BRIGHT)),
            Span::styled("  Quit", Style::default().fg(colors::GRAY)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Press any key to close ",
            Style::default().fg(colors::GRAY),
        )),
    ];

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors::CYAN_BRIGHT))
        .title(Span::styled(
            " Help ",
            Style::default()
                .fg(colors::CYAN_BRIGHT)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(colors::DARK));

    let paragraph = Paragraph::new(help_text).block(help_block);
    f.render_widget(Clear, popup_area);
    f.render_widget(paragraph, popup_area);
}

/// Create a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
