//! CLI Dashboard using ratatui
//!
//! Provides a terminal UI for monitoring bot performance in real-time.
//!
//! ## Features
//! - Live metrics display (balance, P&L, win rate)
//! - Market data (FCPO price, RSI, sentiment)
//! - Open positions overview
//! - Trade history
//! - Auto-refresh every second
//! - Graceful exit on Ctrl+C

use crate::modules::monitoring::metrics::MetricsHandle;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    Terminal,
};
use std::io::{self, Stdout};
use std::time::Duration;

/// CLI Dashboard
pub struct Dashboard {
    metrics: MetricsHandle,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    should_quit: bool,
}

impl Dashboard {
    /// Create new dashboard
    pub fn new(metrics: MetricsHandle) -> io::Result<Self> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            metrics,
            terminal,
            should_quit: false,
        })
    }

    /// Run the dashboard (blocking)
    pub fn run(&mut self) -> io::Result<()> {
        loop {
            // Draw UI - extract metrics snapshot before draw to avoid borrow conflict
            let metrics_snapshot = self.metrics.snapshot();
            self.terminal.draw(|f| render_ui(f, &metrics_snapshot))?;

            // Handle events with timeout
            if event::poll(Duration::from_millis(1000))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                                self.should_quit = true;
                            }
                            KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                self.should_quit = true;
                            }
                            _ => {}
                        }
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(())
    }

}

/// Render the UI (standalone function to avoid borrow conflicts)
fn render_ui(frame: &mut ratatui::Frame, metrics: &crate::modules::monitoring::metrics::BotMetrics) {
    let size = frame.size();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(5),  // Account info
            Constraint::Length(5),  // Market data
            Constraint::Min(6),     // Positions
            Constraint::Length(4),  // Stats
            Constraint::Length(1),  // Footer
        ])
        .split(size);

    // Render each section
    render_header(frame, chunks[0], metrics);
    render_account(frame, chunks[1], metrics);
    render_market(frame, chunks[2], metrics);
    render_positions(frame, chunks[3], metrics);
    render_stats(frame, chunks[4], metrics);
    render_footer(frame, chunks[5]);
}

/// Render header
fn render_header(frame: &mut ratatui::Frame, area: Rect, metrics: &crate::modules::monitoring::metrics::BotMetrics) {
    let status = if metrics.open_positions().is_empty() {
        ("IDLE", Color::Yellow)
    } else {
        ("TRADING", Color::Green)
    };

    let header = Paragraph::new(vec![Line::from(vec![
        Span::styled("🌴 ", Style::default().fg(Color::Green)),
        Span::styled(
            "PALM OIL BOT - LIVE DASHBOARD",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("[{}]", status.0),
            Style::default().fg(status.1).add_modifier(Modifier::BOLD),
        ),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .alignment(Alignment::Center);

    frame.render_widget(header, area);
}

/// Render account information
fn render_account(frame: &mut ratatui::Frame, area: Rect, metrics: &crate::modules::monitoring::metrics::BotMetrics) {
    let daily_pnl = metrics.daily_pnl();
    let daily_pnl_percent = metrics.daily_pnl_percent();
    let pnl_color = if daily_pnl >= 0.0 {
        Color::Green
    } else {
        Color::Red
    };
    let pnl_sign = if daily_pnl >= 0.0 { "+" } else { "" };

    let text = vec![
        Line::from(vec![
            Span::styled("Account ID: ", Style::default().fg(Color::Gray)),
            Span::styled("10092792 (DEMO)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Balance:    ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("${:.2}", metrics.current_balance),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("({}{:.2}% today)", pnl_sign, daily_pnl_percent),
                Style::default().fg(pnl_color),
            ),
        ]),
        Line::from(vec![
            Span::styled("Runtime:    ", Style::default().fg(Color::Gray)),
            Span::styled(
                metrics.runtime_formatted(),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    let account = Paragraph::new(text)
        .block(
            Block::default()
                .title(" ACCOUNT ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(account, area);
}

/// Render market data
fn render_market(frame: &mut ratatui::Frame, area: Rect, metrics: &crate::modules::monitoring::metrics::BotMetrics) {
    let price_str = metrics
        .current_price
        .map(|p| format!("{:.2} MYR", p))
        .unwrap_or_else(|| "N/A".to_string());

    let rsi_str = metrics
        .current_rsi
        .map(|r| format!("{:.1}", r))
        .unwrap_or_else(|| "N/A".to_string());

    let sentiment_str = metrics
        .current_sentiment
        .map(|s| {
            let sign = if s >= 0 { "+" } else { "" };
            format!("{}{}", sign, s)
        })
        .unwrap_or_else(|| "N/A".to_string());

    // Determine RSI color
    let rsi_color = if let Some(rsi) = metrics.current_rsi {
        if rsi < 30.0 {
            Color::Green // Oversold - potential buy
        } else if rsi > 70.0 {
            Color::Red // Overbought - potential sell
        } else {
            Color::Yellow
        }
    } else {
        Color::Gray
    };

    // Determine sentiment color
    let sentiment_color = if let Some(sentiment) = metrics.current_sentiment {
        if sentiment > 30 {
            Color::Green
        } else if sentiment < -30 {
            Color::Red
        } else {
            Color::Yellow
        }
    } else {
        Color::Gray
    };

    let text = vec![
        Line::from(vec![
            Span::styled("FCPO Price:  ", Style::default().fg(Color::Gray)),
            Span::styled(
                price_str,
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("RSI (5m):    ", Style::default().fg(Color::Gray)),
            Span::styled(
                rsi_str,
                Style::default().fg(rsi_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Sentiment:   ", Style::default().fg(Color::Gray)),
            Span::styled(
                sentiment_str,
                Style::default()
                    .fg(sentiment_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (Perplexity)", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let market = Paragraph::new(text)
        .block(
            Block::default()
                .title(" MARKET DATA ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(market, area);
}

/// Render open positions
fn render_positions(frame: &mut ratatui::Frame, area: Rect, metrics: &crate::modules::monitoring::metrics::BotMetrics) {
    let positions = metrics.open_positions();
    let positions_count = positions.len();

    let mut rows = Vec::new();

    if positions.is_empty() {
        rows.push(Row::new(vec![
            "No open positions",
            "",
            "",
            "",
            "",
        ]));
    } else {
        for pos in positions {
            let pnl_str = format!("${:.2}", pos.pnl);
            let pnl_color = if pos.pnl >= 0.0 {
                Color::Green
            } else {
                Color::Red
            };

            let duration = pos.duration_secs();
            let duration_str = if duration >= 3600 {
                format!("{}h {}m", duration / 3600, (duration % 3600) / 60)
            } else if duration >= 60 {
                format!("{}m", duration / 60)
            } else {
                format!("{}s", duration)
            };

            rows.push(Row::new(vec![
                pos.id.clone(),
                format!("{} {}", pos.direction, pos.volume),
                format!("{:.2}", pos.entry_price),
                duration_str,
                pnl_str,
            ]).style(Style::default().fg(pnl_color)));
        }
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Min(10),
        ],
    )
    .header(
        Row::new(vec!["ID", "Type", "Entry", "Duration", "P&L"])
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .title(format!(" OPEN POSITIONS ({}) ", positions_count))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(table, area);
}

/// Render statistics
fn render_stats(frame: &mut ratatui::Frame, area: Rect, metrics: &crate::modules::monitoring::metrics::BotMetrics) {
    let total_pnl = metrics.total_pnl();
    let pnl_color = if total_pnl >= 0.0 {
        Color::Green
    } else {
        Color::Red
    };
    let pnl_sign = if total_pnl >= 0.0 { "+" } else { "" };

    let text = vec![
        Line::from(vec![
            Span::styled("Today: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!(
                    "{} trades | {:.1}% win rate | ",
                    metrics.todays_trades().len(),
                    metrics.win_rate()
                ),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                format!("{}${:.2} P&L", pnl_sign, total_pnl),
                Style::default().fg(pnl_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("All-time: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} trades", metrics.total_trades()),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    let stats = Paragraph::new(text)
        .block(
            Block::default()
                .title(" STATISTICS ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(stats, area);
}

/// Render footer with controls
fn render_footer(frame: &mut ratatui::Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Press ", Style::default().fg(Color::Gray)),
        Span::styled("Q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" or ", Style::default().fg(Color::Gray)),
        Span::styled("Ctrl+C", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(" to quit", Style::default().fg(Color::Gray)),
    ]))
    .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}

impl Drop for Dashboard {
    fn drop(&mut self) {
        // Restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen
        );
        let _ = self.terminal.show_cursor();
    }
}

/// Run dashboard in a separate thread
pub async fn run_dashboard_async(metrics: MetricsHandle) -> io::Result<()> {
    tokio::task::spawn_blocking(move || {
        let mut dashboard = Dashboard::new(metrics)?;
        dashboard.run()
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::monitoring::metrics::Trade;

    #[test]
    fn test_dashboard_creation() {
        let metrics = MetricsHandle::new(10000.0);
        // Can't test full dashboard without TTY, but we can test metrics
        assert_eq!(metrics.snapshot().total_trades(), 0);
    }

    #[test]
    fn test_metrics_with_positions() {
        let metrics = MetricsHandle::new(10000.0);

        metrics.with_metrics_mut(|m| {
            m.add_trade(Trade::new("1".to_string(), "BUY".to_string(), 0.1, 4800.0));
            m.update_market_data(4832.5, 42.3, 28);
        });

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.open_positions().len(), 1);
        assert_eq!(snapshot.current_rsi, Some(42.3));
        assert_eq!(snapshot.current_sentiment, Some(28));
        assert_eq!(snapshot.current_price, Some(4832.5));
    }
}
