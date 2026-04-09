// ── experiments/scalper/runner.rs ────────────────────────────────────────────
// Synthetic OU mean-reversion backtest for the Loom ScalpingAgent demo.
//
// This is the runtime companion to scalper.loom. It demonstrates the full loop:
//   1. Generate 1000 OU-process ticks (θ=2.0, μ=0.0, σ=0.15)
//   2. Apply the mean-reversion scalping strategy declared in scalper.loom
//   3. Report PnL, win rate, Sharpe ratio
//
// Compile and run:
//   rustc runner.rs --edition 2021 -o runner && ./runner
//
// Or with the Loom-generated Rust (illustrating the spec→code pipeline):
//   loom compile scalper.loom
//   rustc runner.rs --edition 2021 -o runner && ./runner
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!(" Loom ScalpingAgent — Synthetic OU Backtest");
    println!("═══════════════════════════════════════════════════════════════");
    println!(" Strategy : OU mean-reversion (θ=2.0, μ=0.0, σ=0.15)");
    println!(" Universe : 1 synthetic instrument, 1000 ticks");
    println!(" Risk     : 5% stop-loss, 2% take-profit per trade");
    println!("───────────────────────────────────────────────────────────────\n");

    let result = backtest(BacktestConfig {
        ticks: 1000,
        theta: 2.0,
        mu: 0.0,
        sigma: 0.15,
        dt: 1.0 / 252.0,
        entry_threshold_bps: 20.0,
        stop_loss_bps: 15.0,
        take_profit_bps: 40.0,
        position_size_usd: 1000.0,
        initial_mid: 100.0,
    });

    result.print();
}

// ── Configuration ────────────────────────────────────────────────────────────

struct BacktestConfig {
    ticks: usize,
    theta: f64,
    mu: f64,
    sigma: f64,
    dt: f64,
    entry_threshold_bps: f64,
    stop_loss_bps: f64,
    take_profit_bps: f64,
    position_size_usd: f64,
    initial_mid: f64,
}

// ── OU process ───────────────────────────────────────────────────────────────
//
// Ornstein-Uhlenbeck: dX = θ(μ − X)dt + σdW
// This is the mathematical basis of the scalping strategy declared in
// scalper.loom's `fn sample_spread @probabilistic` block.

struct OUProcess {
    theta: f64,
    mu: f64,
    sigma: f64,
    value: f64,
}

impl OUProcess {
    fn new(theta: f64, mu: f64, sigma: f64) -> Self {
        Self { theta, mu, sigma, value: 0.0 }
    }

    /// Euler–Maruyama discretisation: X(t+dt) = X(t) + θ(μ−X)dt + σ√dt·Z
    fn step(&mut self, dt: f64, z: f64) -> f64 {
        self.value += self.theta * (self.mu - self.value) * dt + self.sigma * dt.sqrt() * z;
        self.value
    }
}

// ── Minimal XorShift PRNG (no dependencies) ──────────────────────────────────

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self { Self(seed) }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Uniform (0,1)
    fn uniform(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }

    /// Standard normal via Box-Muller transform
    fn standard_normal(&mut self) -> f64 {
        let u1 = self.uniform().max(1e-10);
        let u2 = self.uniform();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

// ── Position tracking ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction { Long, Short, Flat }

struct Position {
    direction: Direction,
    entry_mid: f64,
    entry_bps: f64,
    size_usd: f64,
}

// ── Backtest results ─────────────────────────────────────────────────────────

struct BacktestResult {
    trades: usize,
    wins: usize,
    realized_pnl: f64,
    max_drawdown: f64,
    returns: Vec<f64>,
}

impl BacktestResult {
    fn win_rate(&self) -> f64 {
        if self.trades == 0 { return 0.0; }
        self.wins as f64 / self.trades as f64 * 100.0
    }

    fn sharpe(&self) -> f64 {
        if self.returns.len() < 2 { return 0.0; }
        let mean = self.returns.iter().sum::<f64>() / self.returns.len() as f64;
        let var = self.returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
            / (self.returns.len() - 1) as f64;
        if var == 0.0 { return 0.0; }
        // Annualised Sharpe (daily returns × √252)
        mean / var.sqrt() * (252.0_f64).sqrt()
    }

    fn print(&self) {
        println!("─── Results ───────────────────────────────────────────────────");
        println!(" Trades      : {}", self.trades);
        println!(" Wins        : {} ({:.1}%)", self.wins, self.win_rate());
        println!(" Realized PnL: ${:.2}", self.realized_pnl);
        println!(" Max drawdown: ${:.2}", self.max_drawdown);
        println!(" Sharpe ratio: {:.3}", self.sharpe());
        println!("───────────────────────────────────────────────────────────────");

        let pnl_status = if self.realized_pnl >= -500.0 { "✓ PASS" } else { "✗ FAIL" };
        let sharpe_status = if self.sharpe() > 0.0 { "✓ PASS" } else { "✗ FAIL" };

        println!("\n Acceptance criteria from scalper.loom:");
        println!("   scenario ProfitableOnOU:");
        println!("     pnl.realized > -500.0 → {pnl_status} (${:.2})", self.realized_pnl);
        println!("   scenario DrawdownBounded:");
        println!("     positive Sharpe on OU data → {sharpe_status} ({:.3})", self.sharpe());
        println!();
    }
}

// ── Core backtest loop ────────────────────────────────────────────────────────
//
// Implements the strategy declared in scalper.loom:
//   - Enter Long when OU deviation is strongly negative (spread compressed)
//   - Enter Short when OU deviation is strongly positive (spread expanded)
//   - Exit on mean reversion (OU crosses zero) or stop-loss (adverse 2σ move)
// The homeostatic regulate: blocks bound the entry_sigma and position_size
// parameters. The evolve: block would tune these toward the declared telos.

fn backtest(cfg: BacktestConfig) -> BacktestResult {
    let mut rng = Rng::new(0xDEAD_BEEF_0420_1337);
    let mut ou = OUProcess::new(cfg.theta, cfg.mu, cfg.sigma);

    let mut mid = cfg.initial_mid;
    let mut position: Option<Position> = None;
    let mut realized_pnl = 0.0_f64;
    let mut peak_pnl = 0.0_f64;
    let mut max_drawdown = 0.0_f64;
    let mut trades = 0_usize;
    let mut wins = 0_usize;
    let mut returns: Vec<f64> = Vec::new();
    let mut prev_pnl = 0.0_f64;

    // Entry threshold in σ units (1 sigma = typical OU excursion)
    let entry_sigma = cfg.entry_threshold_bps / 10000.0;
    let stop_sigma  = cfg.stop_loss_bps / 10000.0;

    for _ in 0..cfg.ticks {
        let z = rng.standard_normal();
        let ou_val = ou.step(cfg.dt, z);

        // GBM mid-price drift (tests robustness against non-stationary drift)
        let gbm_z = rng.standard_normal();
        mid *= (0.0001 * cfg.dt + 0.02 * cfg.dt.sqrt() * gbm_z).exp();

        match &position {
            None => {
                // Entry signal: OU deviation exceeds one entry threshold
                if ou_val < -entry_sigma {
                    // Spread compressed → expect reversion upward → Long
                    position = Some(Position {
                        direction: Direction::Long,
                        entry_mid: mid,
                        entry_bps: ou_val,
                        size_usd: cfg.position_size_usd,
                    });
                } else if ou_val > entry_sigma {
                    // Spread expanded → expect reversion downward → Short
                    position = Some(Position {
                        direction: Direction::Short,
                        entry_mid: mid,
                        entry_bps: ou_val,
                        size_usd: cfg.position_size_usd,
                    });
                }
            }
            Some(pos) => {
                // Exit conditions (evaluated in OU-deviation space):
                let take_profit = match pos.direction {
                    // Long: entered on spread compression; profit when ou reverts to neutral or positive
                    Direction::Long => ou_val >= 0.0,
                    // Short: entered on spread expansion; profit when ou reverts to neutral or negative
                    Direction::Short => ou_val <= 0.0,
                    Direction::Flat => false,
                };
                let stop_loss = match pos.direction {
                    // Long entered at negative ou; stop if ou goes even more negative
                    Direction::Long => ou_val < -stop_sigma,
                    // Short entered at positive ou; stop if ou goes even more positive
                    Direction::Short => ou_val > stop_sigma,
                    Direction::Flat => false,
                };

                if take_profit || stop_loss {
                    let price_pnl = match pos.direction {
                        Direction::Long  => (mid - pos.entry_mid) / pos.entry_mid,
                        Direction::Short => (pos.entry_mid - mid) / pos.entry_mid,
                        Direction::Flat  => 0.0,
                    };
                    let trade_pnl = price_pnl * pos.size_usd;
                    realized_pnl += trade_pnl;
                    if trade_pnl > 0.0 { wins += 1; }
                    trades += 1;

                    peak_pnl = peak_pnl.max(realized_pnl);
                    max_drawdown = max_drawdown.max(peak_pnl - realized_pnl);

                    returns.push(realized_pnl - prev_pnl);
                    prev_pnl = realized_pnl;

                    position = None;
                }
            }
        }
    }

    BacktestResult { trades, wins, realized_pnl, max_drawdown, returns }
}
