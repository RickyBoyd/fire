use std::f64::consts::PI;

use super::types::{
    AgeResult, CashflowYearResult, Inputs, ModelResult, PensionTaxMode, WithdrawalOrder,
    WithdrawalStrategy,
};

#[derive(Debug)]
struct ScenarioResult {
    success: bool,
    reported_retirement_total: f64,
    reported_retirement_isa: f64,
    reported_retirement_taxable: f64,
    reported_retirement_pension: f64,
    reported_retirement_cash: f64,
    reported_retirement_bond_ladder: f64,
    reported_terminal_total: f64,
    reported_terminal_isa: f64,
    reported_terminal_taxable: f64,
    reported_terminal_pension: f64,
    reported_terminal_cash: f64,
    reported_terminal_bond_ladder: f64,
    min_income_ratio: f64,
    avg_income_ratio: f64,
}

#[derive(Debug, Clone, Copy)]
struct ContributionFlow {
    isa: f64,
    taxable: f64,
    pension: f64,
}

impl ContributionFlow {
    fn total(self) -> f64 {
        self.isa + self.taxable + self.pension
    }
}

#[derive(Debug, Clone, Copy)]
struct WithdrawalYearOutcome {
    realized_spending_net: f64,
    portfolio_withdrawn_net: f64,
    non_pension_income_used: f64,
    cgt_tax_paid: f64,
    income_tax_paid: f64,
}

impl WithdrawalYearOutcome {
    fn total_tax_paid(self) -> f64 {
        self.cgt_tax_paid + self.income_tax_paid
    }
}

#[derive(Debug, Clone, Copy)]
struct YearTracePoint {
    contribution_isa_real: f64,
    contribution_taxable_real: f64,
    contribution_pension_real: f64,
    contribution_total_real: f64,
    withdrawal_portfolio_real: f64,
    withdrawal_non_pension_income_real: f64,
    spending_total_real: f64,
    tax_cgt_real: f64,
    tax_income_real: f64,
    tax_total_real: f64,
    end_isa_real: f64,
    end_taxable_real: f64,
    end_pension_real: f64,
    end_cash_real: f64,
    end_bond_ladder_real: f64,
    end_total_real: f64,
}

#[derive(Debug)]
struct Portfolio {
    isa: f64,
    taxable: f64,
    taxable_basis: f64,
    pension: f64,
    cash_buffer: f64,
    bond_ladder: f64,
}

#[derive(Debug)]
struct CgtState {
    allowance_remaining: f64,
    tax_paid: f64,
}

#[derive(Debug, Clone, Copy)]
struct TaxYearState {
    non_pension_taxable_income: f64,
    pension_gross_withdrawn: f64,
    price_index: f64,
}

#[derive(Debug, Clone, Copy)]
struct SpendingState {
    current_real_spending: f64,
    initial_withdrawal_rate: f64,
}

#[derive(Clone, Copy)]
struct MarketSample {
    isa_return: f64,
    taxable_return: f64,
    pension_return: f64,
    inflation: f64,
}

pub fn run_model(inputs: &Inputs) -> ModelResult {
    let mut age_results = Vec::new();
    for retirement_age in inputs.current_age..=inputs.max_retirement_age {
        age_results.push(evaluate_age_candidate(
            inputs,
            retirement_age,
            retirement_age,
            retirement_age,
        ));
    }
    build_model_result(age_results, inputs.success_threshold)
}

pub fn run_coast_model(inputs: &Inputs, retirement_age: u32) -> ModelResult {
    let mut age_results = Vec::new();
    for coast_age in inputs.current_age..=retirement_age {
        age_results.push(evaluate_age_candidate(
            inputs,
            retirement_age,
            coast_age,
            coast_age,
        ));
    }
    build_model_result(age_results, inputs.success_threshold)
}

pub fn run_retirement_age_evaluation(inputs: &Inputs, retirement_age: u32) -> AgeResult {
    evaluate_age_candidate(inputs, retirement_age, retirement_age, retirement_age)
}

struct YearlyAccumulator {
    ages: Vec<u32>,
    contribution_isa: Vec<Vec<f64>>,
    contribution_taxable: Vec<Vec<f64>>,
    contribution_pension: Vec<Vec<f64>>,
    contribution_total: Vec<Vec<f64>>,
    withdrawal_portfolio: Vec<Vec<f64>>,
    withdrawal_non_pension_income: Vec<Vec<f64>>,
    spending_total: Vec<Vec<f64>>,
    tax_cgt: Vec<Vec<f64>>,
    tax_income: Vec<Vec<f64>>,
    tax_total: Vec<Vec<f64>>,
    end_isa: Vec<Vec<f64>>,
    end_taxable: Vec<Vec<f64>>,
    end_pension: Vec<Vec<f64>>,
    end_cash: Vec<Vec<f64>>,
    end_bond_ladder: Vec<Vec<f64>>,
    end_total: Vec<Vec<f64>>,
}

impl YearlyAccumulator {
    fn new(ages: Vec<u32>, expected_samples: usize) -> Self {
        let year_count = ages.len();
        let make = || {
            (0..year_count)
                .map(|_| Vec::with_capacity(expected_samples))
                .collect::<Vec<_>>()
        };

        Self {
            ages,
            contribution_isa: make(),
            contribution_taxable: make(),
            contribution_pension: make(),
            contribution_total: make(),
            withdrawal_portfolio: make(),
            withdrawal_non_pension_income: make(),
            spending_total: make(),
            tax_cgt: make(),
            tax_income: make(),
            tax_total: make(),
            end_isa: make(),
            end_taxable: make(),
            end_pension: make(),
            end_cash: make(),
            end_bond_ladder: make(),
            end_total: make(),
        }
    }

    fn push(&mut self, index: usize, point: YearTracePoint) {
        self.contribution_isa[index].push(point.contribution_isa_real);
        self.contribution_taxable[index].push(point.contribution_taxable_real);
        self.contribution_pension[index].push(point.contribution_pension_real);
        self.contribution_total[index].push(point.contribution_total_real);
        self.withdrawal_portfolio[index].push(point.withdrawal_portfolio_real);
        self.withdrawal_non_pension_income[index].push(point.withdrawal_non_pension_income_real);
        self.spending_total[index].push(point.spending_total_real);
        self.tax_cgt[index].push(point.tax_cgt_real);
        self.tax_income[index].push(point.tax_income_real);
        self.tax_total[index].push(point.tax_total_real);
        self.end_isa[index].push(point.end_isa_real);
        self.end_taxable[index].push(point.end_taxable_real);
        self.end_pension[index].push(point.end_pension_real);
        self.end_cash[index].push(point.end_cash_real);
        self.end_bond_ladder[index].push(point.end_bond_ladder_real);
        self.end_total[index].push(point.end_total_real);
    }

    fn into_results(mut self) -> Vec<CashflowYearResult> {
        let mut results = Vec::with_capacity(self.ages.len());
        for idx in 0..self.ages.len() {
            results.push(CashflowYearResult {
                age: self.ages[idx],
                median_contribution_isa: percentile(&mut self.contribution_isa[idx], 50.0),
                median_contribution_taxable: percentile(&mut self.contribution_taxable[idx], 50.0),
                median_contribution_pension: percentile(&mut self.contribution_pension[idx], 50.0),
                median_contribution_total: percentile(&mut self.contribution_total[idx], 50.0),
                median_withdrawal_portfolio: percentile(&mut self.withdrawal_portfolio[idx], 50.0),
                median_withdrawal_non_pension_income: percentile(
                    &mut self.withdrawal_non_pension_income[idx],
                    50.0,
                ),
                median_spending_total: percentile(&mut self.spending_total[idx], 50.0),
                median_tax_cgt: percentile(&mut self.tax_cgt[idx], 50.0),
                median_tax_income: percentile(&mut self.tax_income[idx], 50.0),
                median_tax_total: percentile(&mut self.tax_total[idx], 50.0),
                median_end_isa: percentile(&mut self.end_isa[idx], 50.0),
                median_end_taxable: percentile(&mut self.end_taxable[idx], 50.0),
                median_end_pension: percentile(&mut self.end_pension[idx], 50.0),
                median_end_cash: percentile(&mut self.end_cash[idx], 50.0),
                median_end_bond_ladder: percentile(&mut self.end_bond_ladder[idx], 50.0),
                median_end_total: percentile(&mut self.end_total[idx], 50.0),
            });
        }
        results
    }
}

pub fn run_yearly_cashflow_trace(
    inputs: &Inputs,
    retirement_age: u32,
    contribution_stop_age: u32,
    reported_age: u32,
) -> Vec<CashflowYearResult> {
    let ages = (inputs.current_age..inputs.horizon_age).collect::<Vec<_>>();
    if ages.is_empty() {
        return Vec::new();
    }

    let mut acc = YearlyAccumulator::new(ages.clone(), inputs.simulations as usize);

    for scenario_id in 0..inputs.simulations {
        let scenario_seed = derive_seed(inputs.seed, reported_age, scenario_id);
        let mut rng = Rng::new(scenario_seed);
        let mut trace = Vec::with_capacity(ages.len());
        let _ = simulate_scenario(
            inputs,
            retirement_age,
            contribution_stop_age,
            &mut rng,
            Some(&mut trace),
        );

        if trace.len() == ages.len() {
            for (idx, point) in trace.into_iter().enumerate() {
                acc.push(idx, point);
            }
            continue;
        }

        for idx in 0..ages.len() {
            let fallback = trace.get(idx).copied().unwrap_or(YearTracePoint {
                contribution_isa_real: 0.0,
                contribution_taxable_real: 0.0,
                contribution_pension_real: 0.0,
                contribution_total_real: 0.0,
                withdrawal_portfolio_real: 0.0,
                withdrawal_non_pension_income_real: 0.0,
                spending_total_real: 0.0,
                tax_cgt_real: 0.0,
                tax_income_real: 0.0,
                tax_total_real: 0.0,
                end_isa_real: 0.0,
                end_taxable_real: 0.0,
                end_pension_real: 0.0,
                end_cash_real: 0.0,
                end_bond_ladder_real: 0.0,
                end_total_real: 0.0,
            });
            acc.push(idx, fallback);
        }
    }

    acc.into_results()
}

fn build_model_result(age_results: Vec<AgeResult>, success_threshold: f64) -> ModelResult {
    let selected_index = age_results
        .iter()
        .position(|r| r.success_rate >= success_threshold);
    let best_index = age_results
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.success_rate.total_cmp(&b.success_rate))
        .map(|(idx, _)| idx)
        .unwrap_or(0);

    ModelResult {
        age_results,
        selected_index,
        best_index,
    }
}

fn evaluate_age_candidate(
    inputs: &Inputs,
    retirement_age: u32,
    contribution_stop_age: u32,
    reported_age: u32,
) -> AgeResult {
    let mut successes = 0_u32;
    let mut retirement = Vec::with_capacity(inputs.simulations as usize);
    let mut retirement_isa = Vec::with_capacity(inputs.simulations as usize);
    let mut retirement_taxable = Vec::with_capacity(inputs.simulations as usize);
    let mut retirement_pension = Vec::with_capacity(inputs.simulations as usize);
    let mut retirement_cash = Vec::with_capacity(inputs.simulations as usize);
    let mut retirement_bond_ladder = Vec::with_capacity(inputs.simulations as usize);
    let mut terminal = Vec::with_capacity(inputs.simulations as usize);
    let mut terminal_isa = Vec::with_capacity(inputs.simulations as usize);
    let mut terminal_taxable = Vec::with_capacity(inputs.simulations as usize);
    let mut terminal_pension = Vec::with_capacity(inputs.simulations as usize);
    let mut terminal_cash = Vec::with_capacity(inputs.simulations as usize);
    let mut terminal_bond_ladder = Vec::with_capacity(inputs.simulations as usize);
    let mut min_income_ratios = Vec::with_capacity(inputs.simulations as usize);
    let mut avg_income_ratios = Vec::with_capacity(inputs.simulations as usize);

    for scenario_id in 0..inputs.simulations {
        let scenario_seed = derive_seed(inputs.seed, reported_age, scenario_id);
        let mut rng = Rng::new(scenario_seed);
        let scenario = simulate_scenario(
            inputs,
            retirement_age,
            contribution_stop_age,
            &mut rng,
            None,
        );
        if scenario.success {
            successes += 1;
        }

        retirement.push(scenario.reported_retirement_total);
        retirement_isa.push(scenario.reported_retirement_isa);
        retirement_taxable.push(scenario.reported_retirement_taxable);
        retirement_pension.push(scenario.reported_retirement_pension);
        retirement_cash.push(scenario.reported_retirement_cash);
        retirement_bond_ladder.push(scenario.reported_retirement_bond_ladder);
        terminal.push(scenario.reported_terminal_total);
        terminal_isa.push(scenario.reported_terminal_isa);
        terminal_taxable.push(scenario.reported_terminal_taxable);
        terminal_pension.push(scenario.reported_terminal_pension);
        terminal_cash.push(scenario.reported_terminal_cash);
        terminal_bond_ladder.push(scenario.reported_terminal_bond_ladder);
        min_income_ratios.push(scenario.min_income_ratio);
        avg_income_ratios.push(scenario.avg_income_ratio);
    }

    AgeResult {
        retirement_age: reported_age,
        success_rate: successes as f64 / inputs.simulations as f64,
        median_retirement_pot: percentile(&mut retirement, 50.0),
        p10_retirement_pot: percentile(&mut retirement, 10.0),
        median_retirement_isa: percentile(&mut retirement_isa, 50.0),
        p10_retirement_isa: percentile(&mut retirement_isa, 10.0),
        median_retirement_taxable: percentile(&mut retirement_taxable, 50.0),
        p10_retirement_taxable: percentile(&mut retirement_taxable, 10.0),
        median_retirement_pension: percentile(&mut retirement_pension, 50.0),
        p10_retirement_pension: percentile(&mut retirement_pension, 10.0),
        median_retirement_cash: percentile(&mut retirement_cash, 50.0),
        p10_retirement_cash: percentile(&mut retirement_cash, 10.0),
        median_retirement_bond_ladder: percentile(&mut retirement_bond_ladder, 50.0),
        p10_retirement_bond_ladder: percentile(&mut retirement_bond_ladder, 10.0),
        median_terminal_pot: percentile(&mut terminal, 50.0),
        p10_terminal_pot: percentile(&mut terminal, 10.0),
        median_terminal_isa: percentile(&mut terminal_isa, 50.0),
        p10_terminal_isa: percentile(&mut terminal_isa, 10.0),
        median_terminal_taxable: percentile(&mut terminal_taxable, 50.0),
        p10_terminal_taxable: percentile(&mut terminal_taxable, 10.0),
        median_terminal_pension: percentile(&mut terminal_pension, 50.0),
        p10_terminal_pension: percentile(&mut terminal_pension, 10.0),
        median_terminal_cash: percentile(&mut terminal_cash, 50.0),
        p10_terminal_cash: percentile(&mut terminal_cash, 10.0),
        median_terminal_bond_ladder: percentile(&mut terminal_bond_ladder, 50.0),
        p10_terminal_bond_ladder: percentile(&mut terminal_bond_ladder, 10.0),
        p10_min_income_ratio: percentile(&mut min_income_ratios, 10.0),
        median_avg_income_ratio: percentile(&mut avg_income_ratios, 50.0),
    }
}

fn simulate_scenario(
    inputs: &Inputs,
    retirement_age: u32,
    contribution_stop_age: u32,
    rng: &mut Rng,
    mut trace: Option<&mut Vec<YearTracePoint>>,
) -> ScenarioResult {
    let mut portfolio = Portfolio {
        isa: inputs.isa_start,
        taxable: inputs.taxable_start,
        taxable_basis: inputs.taxable_cost_basis_start.min(inputs.taxable_start),
        pension: inputs.pension_start,
        cash_buffer: inputs.cash_start,
        bond_ladder: inputs.bond_ladder_start,
    };

    let mut price_index = 1.0;

    for (years_since_start, age) in (inputs.current_age..retirement_age).enumerate() {
        let sampled = sample_market(inputs, rng);
        apply_pre_retirement_growth(inputs, &mut portfolio, &sampled);
        let contributions = if age < contribution_stop_age {
            apply_pre_retirement_contributions(inputs, &mut portfolio, years_since_start as u32)
        } else {
            ContributionFlow {
                isa: 0.0,
                taxable: 0.0,
                pension: 0.0,
            }
        };
        price_index *= 1.0 + sampled.inflation;

        if let Some(trace_rows) = trace.as_deref_mut() {
            let deflator = price_index.max(1e-9);
            trace_rows.push(YearTracePoint {
                contribution_isa_real: contributions.isa / deflator,
                contribution_taxable_real: contributions.taxable / deflator,
                contribution_pension_real: contributions.pension / deflator,
                contribution_total_real: contributions.total() / deflator,
                withdrawal_portfolio_real: 0.0,
                withdrawal_non_pension_income_real: 0.0,
                spending_total_real: 0.0,
                tax_cgt_real: 0.0,
                tax_income_real: 0.0,
                tax_total_real: 0.0,
                end_isa_real: portfolio.isa / deflator,
                end_taxable_real: portfolio.taxable / deflator,
                end_pension_real: portfolio.pension / deflator,
                end_cash_real: portfolio.cash_buffer / deflator,
                end_bond_ladder_real: portfolio.bond_ladder / deflator,
                end_total_real: (portfolio.isa
                    + portfolio.taxable
                    + portfolio.pension
                    + portfolio.cash_buffer
                    + portfolio.bond_ladder)
                    / deflator,
            });
        }
    }

    let retirement_deflator = price_index.max(1e-9);
    let retirement_nominal_total = portfolio.isa
        + portfolio.taxable
        + portfolio.pension
        + portfolio.cash_buffer
        + portfolio.bond_ladder;
    let retirement_total_real = retirement_nominal_total / retirement_deflator;
    let retirement_isa_real = portfolio.isa / retirement_deflator;
    let retirement_taxable_real = portfolio.taxable / retirement_deflator;
    let retirement_pension_real = portfolio.pension / retirement_deflator;
    let retirement_cash_real = portfolio.cash_buffer / retirement_deflator;
    let retirement_bond_ladder_real = portfolio.bond_ladder / retirement_deflator;

    let initial_withdrawal_rate = inputs.target_annual_income / retirement_total_real.max(1e-9);
    let mut spending_state = SpendingState {
        current_real_spending: inputs.target_annual_income,
        initial_withdrawal_rate,
    };
    let mut prev_real_return = 0.0;
    let mut min_income_ratio = f64::INFINITY;
    let mut income_ratio_sum = 0.0;
    let mut years = 0_u32;

    for age in retirement_age..inputs.horizon_age {
        let mortgage_real_spending = mortgage_payment_real(inputs, age);
        let available_real = available_spendable_real(inputs, age, &portfolio, price_index);
        let available_core_real = (available_real - mortgage_real_spending).max(0.0);
        let planned_core_real_spending = plan_real_spending(
            inputs,
            age,
            prev_real_return,
            available_core_real,
            &mut spending_state,
        );
        let planned_real_spending = planned_core_real_spending + mortgage_real_spending;

        let sampled = sample_market(inputs, rng);
        price_index *= 1.0 + sampled.inflation;

        let planned_nominal_spending = planned_real_spending * price_index;
        let mut cgt_state = CgtState {
            allowance_remaining: inputs.capital_gains_allowance,
            tax_paid: 0.0,
        };

        let state_pension_gross = state_pension_gross_income(inputs, age, price_index);
        let state_pension_net = net_income_after_tax(state_pension_gross, inputs, price_index);
        let mut tax_state = TaxYearState {
            non_pension_taxable_income: state_pension_gross,
            pension_gross_withdrawn: 0.0,
            price_index,
        };

        let year_outcome = run_withdrawal_year(
            inputs,
            age,
            age.saturating_sub(retirement_age),
            planned_nominal_spending,
            prev_real_return,
            planned_real_spending,
            &mut portfolio,
            &mut cgt_state,
            &mut tax_state,
            state_pension_net,
        );

        let required_real_spending = required_real_spending(inputs, age).max(1e-9);
        let income_ratio =
            (year_outcome.realized_spending_net / price_index) / required_real_spending;
        min_income_ratio = min_income_ratio.min(income_ratio);
        income_ratio_sum += income_ratio;
        years += 1;

        let failed = year_outcome.realized_spending_net + 1e-9 < planned_nominal_spending;
        if failed {
            if let Some(trace_rows) = trace.as_deref_mut() {
                let deflator = price_index.max(1e-9);
                trace_rows.push(YearTracePoint {
                    contribution_isa_real: 0.0,
                    contribution_taxable_real: 0.0,
                    contribution_pension_real: 0.0,
                    contribution_total_real: 0.0,
                    withdrawal_portfolio_real: year_outcome.portfolio_withdrawn_net / deflator,
                    withdrawal_non_pension_income_real: year_outcome.non_pension_income_used
                        / deflator,
                    spending_total_real: year_outcome.realized_spending_net / deflator,
                    tax_cgt_real: year_outcome.cgt_tax_paid / deflator,
                    tax_income_real: year_outcome.income_tax_paid / deflator,
                    tax_total_real: year_outcome.total_tax_paid() / deflator,
                    end_isa_real: 0.0,
                    end_taxable_real: 0.0,
                    end_pension_real: 0.0,
                    end_cash_real: 0.0,
                    end_bond_ladder_real: 0.0,
                    end_total_real: 0.0,
                });
                push_zero_trace_tail(trace_rows, age + 1, inputs.horizon_age);
            }

            return ScenarioResult {
                success: false,
                reported_retirement_total: retirement_total_real,
                reported_retirement_isa: retirement_isa_real,
                reported_retirement_taxable: retirement_taxable_real,
                reported_retirement_pension: retirement_pension_real,
                reported_retirement_cash: retirement_cash_real,
                reported_retirement_bond_ladder: retirement_bond_ladder_real,
                reported_terminal_total: 0.0,
                reported_terminal_isa: 0.0,
                reported_terminal_taxable: 0.0,
                reported_terminal_pension: 0.0,
                reported_terminal_cash: 0.0,
                reported_terminal_bond_ladder: 0.0,
                min_income_ratio,
                avg_income_ratio: income_ratio_sum / years as f64,
            };
        }

        let start_invested =
            portfolio.isa + portfolio.taxable + portfolio.pension + portfolio.bond_ladder;
        apply_post_retirement_growth(inputs, &mut portfolio, &sampled);
        let end_invested =
            portfolio.isa + portfolio.taxable + portfolio.pension + portfolio.bond_ladder;
        prev_real_return = realized_real_return(start_invested, end_invested, sampled.inflation);

        if let Some(trace_rows) = trace.as_deref_mut() {
            let deflator = price_index.max(1e-9);
            trace_rows.push(YearTracePoint {
                contribution_isa_real: 0.0,
                contribution_taxable_real: 0.0,
                contribution_pension_real: 0.0,
                contribution_total_real: 0.0,
                withdrawal_portfolio_real: year_outcome.portfolio_withdrawn_net / deflator,
                withdrawal_non_pension_income_real: year_outcome.non_pension_income_used / deflator,
                spending_total_real: year_outcome.realized_spending_net / deflator,
                tax_cgt_real: year_outcome.cgt_tax_paid / deflator,
                tax_income_real: year_outcome.income_tax_paid / deflator,
                tax_total_real: year_outcome.total_tax_paid() / deflator,
                end_isa_real: portfolio.isa / deflator,
                end_taxable_real: portfolio.taxable / deflator,
                end_pension_real: portfolio.pension / deflator,
                end_cash_real: portfolio.cash_buffer / deflator,
                end_bond_ladder_real: portfolio.bond_ladder / deflator,
                end_total_real: (portfolio.isa
                    + portfolio.taxable
                    + portfolio.pension
                    + portfolio.cash_buffer
                    + portfolio.bond_ladder)
                    / deflator,
            });
        }
    }

    let inflation_deflator = price_index.max(1e-9);
    let nominal_total = portfolio.isa
        + portfolio.taxable
        + portfolio.pension
        + portfolio.cash_buffer
        + portfolio.bond_ladder;

    ScenarioResult {
        success: true,
        reported_retirement_total: retirement_total_real,
        reported_retirement_isa: retirement_isa_real,
        reported_retirement_taxable: retirement_taxable_real,
        reported_retirement_pension: retirement_pension_real,
        reported_retirement_cash: retirement_cash_real,
        reported_retirement_bond_ladder: retirement_bond_ladder_real,
        reported_terminal_total: nominal_total / inflation_deflator,
        reported_terminal_isa: portfolio.isa / inflation_deflator,
        reported_terminal_taxable: portfolio.taxable / inflation_deflator,
        reported_terminal_pension: portfolio.pension / inflation_deflator,
        reported_terminal_cash: portfolio.cash_buffer / inflation_deflator,
        reported_terminal_bond_ladder: portfolio.bond_ladder / inflation_deflator,
        min_income_ratio,
        avg_income_ratio: income_ratio_sum / years as f64,
    }
}

fn push_zero_trace_tail(trace: &mut Vec<YearTracePoint>, start_age: u32, horizon_age: u32) {
    for _ in start_age..horizon_age {
        trace.push(YearTracePoint {
            contribution_isa_real: 0.0,
            contribution_taxable_real: 0.0,
            contribution_pension_real: 0.0,
            contribution_total_real: 0.0,
            withdrawal_portfolio_real: 0.0,
            withdrawal_non_pension_income_real: 0.0,
            spending_total_real: 0.0,
            tax_cgt_real: 0.0,
            tax_income_real: 0.0,
            tax_total_real: 0.0,
            end_isa_real: 0.0,
            end_taxable_real: 0.0,
            end_pension_real: 0.0,
            end_cash_real: 0.0,
            end_bond_ladder_real: 0.0,
            end_total_real: 0.0,
        });
    }
}

fn apply_pre_retirement_growth(inputs: &Inputs, portfolio: &mut Portfolio, sampled: &MarketSample) {
    portfolio.isa = (portfolio.isa * (1.0 + sampled.isa_return)).max(0.0);
    portfolio.taxable = (portfolio.taxable * (1.0 + sampled.taxable_return)).max(0.0);
    portfolio.taxable *= 1.0 - inputs.taxable_return_tax_drag;
    portfolio.taxable = portfolio.taxable.max(0.0);
    portfolio.pension = (portfolio.pension * (1.0 + sampled.pension_return)).max(0.0);
    portfolio.bond_ladder = (portfolio.bond_ladder * (1.0 + inputs.bond_ladder_yield)).max(0.0);
    portfolio.taxable_basis = portfolio.taxable_basis.min(portfolio.taxable);
}

fn apply_pre_retirement_contributions(
    inputs: &Inputs,
    portfolio: &mut Portfolio,
    years_since_start: u32,
) -> ContributionFlow {
    let contribution_multiplier =
        (1.0 + inputs.contribution_growth_rate).powi(years_since_start as i32);
    let requested_isa_contribution = inputs.isa_annual_contribution * contribution_multiplier;
    let requested_taxable_contribution =
        inputs.taxable_annual_contribution * contribution_multiplier;
    let requested_pension_contribution =
        inputs.pension_annual_contribution * contribution_multiplier;

    let isa_contribution = requested_isa_contribution
        .max(0.0)
        .min(inputs.isa_annual_contribution_limit);
    let overflow_to_taxable = (requested_isa_contribution - isa_contribution).max(0.0);
    let taxable_contribution = requested_taxable_contribution.max(0.0) + overflow_to_taxable;

    portfolio.isa += isa_contribution;
    portfolio.taxable += taxable_contribution;
    portfolio.taxable_basis += taxable_contribution;
    let pension_contribution = requested_pension_contribution.max(0.0);
    portfolio.pension += pension_contribution;

    ContributionFlow {
        isa: isa_contribution,
        taxable: taxable_contribution,
        pension: pension_contribution,
    }
}

fn apply_post_retirement_growth(
    inputs: &Inputs,
    portfolio: &mut Portfolio,
    sampled: &MarketSample,
) {
    portfolio.isa = (portfolio.isa * (1.0 + sampled.isa_return)).max(0.0);
    portfolio.taxable = (portfolio.taxable * (1.0 + sampled.taxable_return)).max(0.0);
    portfolio.taxable *= 1.0 - inputs.taxable_return_tax_drag;
    portfolio.taxable = portfolio.taxable.max(0.0);
    portfolio.pension = (portfolio.pension * (1.0 + sampled.pension_return)).max(0.0);
    portfolio.cash_buffer = (portfolio.cash_buffer * (1.0 + inputs.cash_growth_rate)).max(0.0);
    portfolio.bond_ladder = (portfolio.bond_ladder * (1.0 + inputs.bond_ladder_yield)).max(0.0);
    portfolio.taxable_basis = portfolio.taxable_basis.min(portfolio.taxable);
}

fn spending_bounds(inputs: &Inputs) -> (f64, f64) {
    let min_real_spending = inputs.target_annual_income * inputs.min_income_floor;
    let max_real_spending = inputs.target_annual_income * inputs.max_income_ceiling;
    (min_real_spending, max_real_spending.max(min_real_spending))
}

fn mortgage_payment_real(inputs: &Inputs, age: u32) -> f64 {
    if inputs.mortgage_annual_payment <= 0.0 {
        return 0.0;
    }
    let Some(end_age) = inputs.mortgage_end_age else {
        return 0.0;
    };
    if age < end_age {
        inputs.mortgage_annual_payment.max(0.0)
    } else {
        0.0
    }
}

fn required_real_spending(inputs: &Inputs, age: u32) -> f64 {
    inputs.target_annual_income + mortgage_payment_real(inputs, age)
}

fn available_spendable_real(
    inputs: &Inputs,
    age: u32,
    portfolio: &Portfolio,
    price_index: f64,
) -> f64 {
    let mut total =
        portfolio.cash_buffer + portfolio.isa + portfolio.taxable + portfolio.bond_ladder;
    if age >= inputs.pension_access_age {
        total += portfolio.pension;
    }
    total / price_index.max(1e-9)
}

fn annuity_withdrawal_rate(real_return: f64, years_remaining: u32) -> f64 {
    let years = years_remaining.max(1) as f64;
    if real_return.abs() < 1e-9 {
        return (1.0 / years).clamp(0.0, 1.0);
    }

    if real_return <= -0.99 {
        return 1.0;
    }

    let denom = 1.0 - (1.0 + real_return).powf(-years);
    if denom <= 1e-9 {
        1.0
    } else {
        (real_return / denom).clamp(0.0, 1.0)
    }
}

fn plan_real_spending(
    inputs: &Inputs,
    age: u32,
    prev_real_return: f64,
    available_real: f64,
    spending_state: &mut SpendingState,
) -> f64 {
    let (min_real_spending, max_real_spending) = spending_bounds(inputs);
    let mut spending_real = match inputs.withdrawal_strategy {
        WithdrawalStrategy::Guardrails => {
            let mut spending = spending_state.current_real_spending;
            if prev_real_return < inputs.bad_year_threshold {
                spending *= 1.0 - inputs.bad_year_cut;
            } else if prev_real_return > inputs.good_year_threshold {
                spending *= 1.0 + inputs.good_year_raise;
            }
            spending
        }
        WithdrawalStrategy::GuytonKlinger => {
            let mut spending = spending_state.current_real_spending;
            let current_wr = spending / available_real.max(1e-9);
            let lower_guardrail =
                spending_state.initial_withdrawal_rate * inputs.gk_lower_guardrail;
            let upper_guardrail =
                spending_state.initial_withdrawal_rate * inputs.gk_upper_guardrail;

            if prev_real_return < inputs.bad_year_threshold && current_wr > upper_guardrail {
                spending *= 1.0 - inputs.bad_year_cut;
            } else if prev_real_return > inputs.good_year_threshold && current_wr < lower_guardrail
            {
                spending *= 1.0 + inputs.good_year_raise;
            }
            spending
        }
        WithdrawalStrategy::Vpw => {
            let years_remaining = inputs.horizon_age.saturating_sub(age).max(1);
            let withdraw_rate =
                annuity_withdrawal_rate(inputs.vpw_expected_real_return, years_remaining);
            available_real.max(0.0) * withdraw_rate
        }
        WithdrawalStrategy::FloorUpside => {
            let mut spending = spending_state.current_real_spending.max(min_real_spending);
            if prev_real_return < inputs.bad_year_threshold {
                spending *= 1.0 - inputs.bad_year_cut;
            }
            if prev_real_return > 0.0 {
                spending *= 1.0 + prev_real_return * inputs.floor_upside_capture.max(0.0);
            }
            spending
        }
        WithdrawalStrategy::Bucket => {
            let mut spending = spending_state.current_real_spending;
            if prev_real_return < inputs.bad_year_threshold {
                spending *= 1.0 - inputs.bad_year_cut;
            } else if prev_real_return > inputs.good_year_threshold {
                // Keep spending changes more muted; bucket logic stores excess in cash.
                spending *= 1.0 + (inputs.good_year_raise * 0.5);
            }
            spending
        }
    };

    spending_real = spending_real.clamp(min_real_spending, max_real_spending);
    spending_state.current_real_spending = spending_real;
    spending_real
}

#[allow(clippy::too_many_arguments)]
fn run_withdrawal_year(
    inputs: &Inputs,
    age: u32,
    retirement_year_index: u32,
    planned_nominal_spending: f64,
    prev_real_return: f64,
    planned_real_spending: f64,
    portfolio: &mut Portfolio,
    cgt_state: &mut CgtState,
    tax_state: &mut TaxYearState,
    net_non_pension_income: f64,
) -> WithdrawalYearOutcome {
    let mut realized = 0.0;
    let starting_cgt_tax_paid = cgt_state.tax_paid;
    let mut portfolio_withdrawn_total = 0.0;

    let non_pension_used = net_non_pension_income.min(planned_nominal_spending);
    realized += non_pension_used;
    let non_pension_surplus = (net_non_pension_income - non_pension_used).max(0.0);
    portfolio.cash_buffer += non_pension_surplus;

    let from_cash = portfolio
        .cash_buffer
        .min((planned_nominal_spending - realized).max(0.0));
    portfolio.cash_buffer -= from_cash;
    realized += from_cash;

    let ladder_scheduled = withdraw_from_bond_ladder_for_net(
        inputs,
        retirement_year_index,
        (planned_nominal_spending - realized).max(0.0),
        &mut portfolio.bond_ladder,
        true,
    );
    realized += ladder_scheduled;
    portfolio_withdrawn_total += ladder_scheduled;

    let needed = (planned_nominal_spending - realized).max(0.0);
    let main_withdrawn = withdraw_from_portfolio(
        inputs,
        age,
        needed,
        portfolio,
        cgt_state,
        tax_state,
        inputs.post_access_withdrawal_order,
    );
    realized += main_withdrawn;
    portfolio_withdrawn_total += main_withdrawn;

    // If scheduled ladder withdrawals plus the normal order still cannot fund spending,
    // allow remaining ladder balance to be tapped as an emergency backstop.
    let ladder_backstop = withdraw_from_bond_ladder_for_net(
        inputs,
        retirement_year_index,
        (planned_nominal_spending - realized).max(0.0),
        &mut portfolio.bond_ladder,
        false,
    );
    realized += ladder_backstop;
    portfolio_withdrawn_total += ladder_backstop;

    if prev_real_return > inputs.good_year_threshold {
        let extra = match inputs.withdrawal_strategy {
            WithdrawalStrategy::Bucket => {
                let spending_for_bucket = planned_nominal_spending.max(planned_real_spending);
                let target_cash = spending_for_bucket * inputs.bucket_target_years.max(0.0);
                let shortfall = (target_cash - portfolio.cash_buffer).max(0.0);
                let refill_cap =
                    spending_for_bucket * inputs.good_year_extra_buffer_withdrawal.max(0.0);
                if refill_cap > 0.0 {
                    shortfall.min(refill_cap)
                } else {
                    shortfall
                }
            }
            _ => planned_nominal_spending * inputs.good_year_extra_buffer_withdrawal.max(0.0),
        };

        if extra > 0.0 {
            let extra_withdrawn = withdraw_from_portfolio(
                inputs,
                age,
                extra,
                portfolio,
                cgt_state,
                tax_state,
                inputs.post_access_withdrawal_order,
            );
            portfolio.cash_buffer += extra_withdrawn;
            portfolio_withdrawn_total += extra_withdrawn;
        }
    }

    let total_gross_income =
        tax_state.non_pension_taxable_income + tax_state.pension_gross_withdrawn;
    let income_tax_paid =
        income_tax_for_total_income(total_gross_income, inputs, tax_state.price_index);
    let cgt_tax_paid = (cgt_state.tax_paid - starting_cgt_tax_paid).max(0.0);

    WithdrawalYearOutcome {
        realized_spending_net: realized,
        portfolio_withdrawn_net: portfolio_withdrawn_total,
        non_pension_income_used: non_pension_used,
        cgt_tax_paid,
        income_tax_paid,
    }
}

fn withdraw_from_bond_ladder_for_net(
    inputs: &Inputs,
    retirement_year_index: u32,
    target_net: f64,
    bond_ladder: &mut f64,
    scheduled: bool,
) -> f64 {
    if target_net <= 0.0 || *bond_ladder <= 0.0 {
        return 0.0;
    }

    let max_available = if scheduled && inputs.bond_ladder_years > 0 {
        if retirement_year_index >= inputs.bond_ladder_years {
            *bond_ladder
        } else {
            let years_left = (inputs.bond_ladder_years - retirement_year_index).max(1) as f64;
            (*bond_ladder / years_left).max(0.0).min(*bond_ladder)
        }
    } else {
        *bond_ladder
    };

    let withdrawn = target_net.min(max_available);
    *bond_ladder -= withdrawn;
    withdrawn
}

fn withdraw_from_portfolio(
    inputs: &Inputs,
    age: u32,
    target_net: f64,
    portfolio: &mut Portfolio,
    cgt_state: &mut CgtState,
    tax_state: &mut TaxYearState,
    order: WithdrawalOrder,
) -> f64 {
    if target_net <= 0.0 {
        return 0.0;
    }

    let pension_access = age >= inputs.pension_access_age;

    if order == WithdrawalOrder::ProRata {
        return withdraw_pro_rata(
            inputs,
            pension_access,
            target_net,
            portfolio,
            cgt_state,
            tax_state,
        );
    }

    let sequence: &[PotKind] = if !pension_access {
        match order {
            WithdrawalOrder::BondLadderFirst => {
                &[PotKind::BondLadder, PotKind::Isa, PotKind::Taxable]
            }
            _ => &[PotKind::Isa, PotKind::Taxable],
        }
    } else {
        match order {
            WithdrawalOrder::IsaFirst => &[PotKind::Isa, PotKind::Taxable, PotKind::Pension],
            WithdrawalOrder::TaxableFirst => &[PotKind::Taxable, PotKind::Isa, PotKind::Pension],
            WithdrawalOrder::PensionFirst => &[PotKind::Pension, PotKind::Taxable, PotKind::Isa],
            WithdrawalOrder::BondLadderFirst => &[
                PotKind::BondLadder,
                PotKind::Isa,
                PotKind::Taxable,
                PotKind::Pension,
            ],
            WithdrawalOrder::ProRata => unreachable!(),
        }
    };

    let mut realized = 0.0;
    let mut remaining = target_net;

    for pot in sequence {
        if remaining <= 0.0 {
            break;
        }

        let withdrawn = withdraw_from_single_pot(
            inputs,
            *pot,
            remaining,
            pension_access,
            portfolio,
            cgt_state,
            tax_state,
        );

        realized += withdrawn;
        remaining -= withdrawn;
    }

    realized
}

#[derive(Copy, Clone)]
enum PotKind {
    BondLadder,
    Isa,
    Taxable,
    Pension,
}

fn withdraw_from_single_pot(
    inputs: &Inputs,
    pot: PotKind,
    target_net: f64,
    pension_access: bool,
    portfolio: &mut Portfolio,
    cgt_state: &mut CgtState,
    tax_state: &mut TaxYearState,
) -> f64 {
    match pot {
        PotKind::BondLadder => {
            let x = portfolio.bond_ladder.min(target_net);
            portfolio.bond_ladder -= x;
            x
        }
        PotKind::Isa => {
            let x = portfolio.isa.min(target_net);
            portfolio.isa -= x;
            x
        }
        PotKind::Pension => {
            if !pension_access {
                return 0.0;
            }
            withdraw_from_pension_for_net(target_net, &mut portfolio.pension, inputs, tax_state)
        }
        PotKind::Taxable => withdraw_from_taxable_for_net(
            target_net,
            &mut portfolio.taxable,
            &mut portfolio.taxable_basis,
            cgt_state,
            inputs.capital_gains_tax_rate,
        ),
    }
}

fn withdraw_pro_rata(
    inputs: &Inputs,
    pension_access: bool,
    target_net: f64,
    portfolio: &mut Portfolio,
    cgt_state: &mut CgtState,
    tax_state: &mut TaxYearState,
) -> f64 {
    let mut realized = 0.0;
    let mut remaining = target_net;

    for _ in 0..4 {
        if remaining <= 1e-9 {
            break;
        }

        let isa_balance = portfolio.isa.max(0.0);
        let ladder_balance = portfolio.bond_ladder.max(0.0);
        let taxable_balance = net_from_taxable_gross(
            portfolio.taxable,
            portfolio.taxable,
            portfolio.taxable_basis,
            cgt_state.allowance_remaining,
            inputs.capital_gains_tax_rate,
        )
        .max(0.0);

        let pension_balance = if pension_access {
            net_from_additional_pension_gross(portfolio.pension, tax_state, inputs).max(0.0)
        } else {
            0.0
        };

        let total_capacity = isa_balance + taxable_balance + pension_balance + ladder_balance;
        if total_capacity <= 1e-9 {
            break;
        }

        let isa_target = remaining * (isa_balance / total_capacity);
        let ladder_target = remaining * (ladder_balance / total_capacity);
        let pension_target = remaining * (pension_balance / total_capacity);
        let taxable_target = remaining * (taxable_balance / total_capacity);

        let mut round_realized = 0.0;
        round_realized += withdraw_from_single_pot(
            inputs,
            PotKind::BondLadder,
            ladder_target,
            pension_access,
            portfolio,
            cgt_state,
            tax_state,
        );

        round_realized += withdraw_from_single_pot(
            inputs,
            PotKind::Isa,
            isa_target,
            pension_access,
            portfolio,
            cgt_state,
            tax_state,
        );

        if pension_access {
            round_realized += withdraw_from_single_pot(
                inputs,
                PotKind::Pension,
                pension_target,
                pension_access,
                portfolio,
                cgt_state,
                tax_state,
            );
        }

        round_realized += withdraw_from_single_pot(
            inputs,
            PotKind::Taxable,
            taxable_target,
            pension_access,
            portfolio,
            cgt_state,
            tax_state,
        );

        realized += round_realized;
        remaining = target_net - realized;

        if round_realized <= 1e-9 {
            break;
        }
    }

    let fallback: &[PotKind] = if pension_access {
        &[
            PotKind::Isa,
            PotKind::Pension,
            PotKind::Taxable,
            PotKind::BondLadder,
        ]
    } else {
        &[PotKind::Isa, PotKind::Taxable, PotKind::BondLadder]
    };

    for pot in fallback {
        if remaining <= 1e-9 {
            break;
        }

        let withdrawn = withdraw_from_single_pot(
            inputs,
            *pot,
            remaining,
            pension_access,
            portfolio,
            cgt_state,
            tax_state,
        );
        realized += withdrawn;
        remaining -= withdrawn;
    }

    realized
}

fn withdraw_from_pension_for_net(
    target_net: f64,
    pension_gross: &mut f64,
    inputs: &Inputs,
    tax_state: &mut TaxYearState,
) -> f64 {
    if target_net <= 0.0 || *pension_gross <= 0.0 {
        return 0.0;
    }

    let max_net = net_from_additional_pension_gross(*pension_gross, tax_state, inputs);
    let desired_net = target_net.min(max_net);
    if desired_net <= 0.0 {
        return 0.0;
    }

    let mut lo = 0.0;
    let mut hi = *pension_gross;

    for _ in 0..40 {
        let mid = (lo + hi) * 0.5;
        let net_mid = net_from_additional_pension_gross(mid, tax_state, inputs);
        if net_mid < desired_net {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    let gross_withdrawn = hi.min(*pension_gross);
    let net = net_from_additional_pension_gross(gross_withdrawn, tax_state, inputs);
    *pension_gross -= gross_withdrawn;
    tax_state.pension_gross_withdrawn += gross_withdrawn;
    net
}

fn net_from_additional_pension_gross(
    additional_gross: f64,
    tax_state: &TaxYearState,
    inputs: &Inputs,
) -> f64 {
    if additional_gross <= 0.0 {
        return 0.0;
    }

    let before_income = tax_state.non_pension_taxable_income + tax_state.pension_gross_withdrawn;
    let after_income = before_income + additional_gross;

    let before_tax = income_tax_for_total_income(before_income, inputs, tax_state.price_index);
    let after_tax = income_tax_for_total_income(after_income, inputs, tax_state.price_index);
    let incremental_tax = (after_tax - before_tax).max(0.0);

    (additional_gross - incremental_tax).max(0.0)
}

fn income_tax_for_total_income(total_income: f64, inputs: &Inputs, price_index: f64) -> f64 {
    let gross = total_income.max(0.0);
    match inputs.pension_tax_mode {
        PensionTaxMode::FlatRate => gross * inputs.pension_flat_tax_rate.clamp(0.0, 1.0),
        PensionTaxMode::UkBands => uk_income_tax(gross, inputs, price_index),
    }
}

fn uk_income_tax(gross_income: f64, inputs: &Inputs, price_index: f64) -> f64 {
    let gross = gross_income.max(0.0);

    let taper_start = (inputs.uk_allowance_taper_start * price_index).max(0.0);
    let taper_end = (inputs.uk_allowance_taper_end * price_index).max(taper_start);

    let mut allowance = (inputs.uk_personal_allowance * price_index).max(0.0);
    if gross > taper_start {
        let reduction = (gross - taper_start) / 2.0;
        allowance = (allowance - reduction).max(0.0);
    }
    if gross >= taper_end {
        allowance = 0.0;
    }

    let taxable_income = (gross - allowance).max(0.0);

    let basic_limit = (inputs.uk_basic_rate_limit * price_index).max(0.0);
    let higher_limit = (inputs.uk_higher_rate_limit * price_index).max(basic_limit);

    let basic_band_width = (basic_limit - allowance).max(0.0);
    let higher_band_width = (higher_limit - basic_limit).max(0.0);

    let basic_taxable = taxable_income.min(basic_band_width);
    let higher_taxable = (taxable_income - basic_taxable)
        .min(higher_band_width)
        .max(0.0);
    let additional_taxable = (taxable_income - basic_taxable - higher_taxable).max(0.0);

    basic_taxable * inputs.uk_basic_rate.clamp(0.0, 1.0)
        + higher_taxable * inputs.uk_higher_rate.clamp(0.0, 1.0)
        + additional_taxable * inputs.uk_additional_rate.clamp(0.0, 1.0)
}

fn state_pension_gross_income(inputs: &Inputs, age: u32, price_index: f64) -> f64 {
    if age < inputs.state_pension_start_age {
        0.0
    } else {
        (inputs.state_pension_annual_income * price_index).max(0.0)
    }
}

fn net_income_after_tax(gross_income: f64, inputs: &Inputs, price_index: f64) -> f64 {
    let gross = gross_income.max(0.0);
    let tax = income_tax_for_total_income(gross, inputs, price_index);
    (gross - tax).max(0.0)
}

fn withdraw_from_taxable_for_net(
    target_net: f64,
    taxable_value: &mut f64,
    taxable_basis: &mut f64,
    cgt_state: &mut CgtState,
    cgt_rate: f64,
) -> f64 {
    if target_net <= 0.0 || *taxable_value <= 0.0 {
        return 0.0;
    }

    let max_net = net_from_taxable_gross(
        *taxable_value,
        *taxable_value,
        *taxable_basis,
        cgt_state.allowance_remaining,
        cgt_rate,
    );

    let desired_net = target_net.min(max_net);
    if desired_net <= 0.0 {
        return 0.0;
    }

    let mut lo = 0.0;
    let mut hi = *taxable_value;

    for _ in 0..40 {
        let mid = (lo + hi) * 0.5;
        let net_mid = net_from_taxable_gross(
            mid,
            *taxable_value,
            *taxable_basis,
            cgt_state.allowance_remaining,
            cgt_rate,
        );

        if net_mid < desired_net {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    let gross = hi.min(*taxable_value);
    execute_taxable_sale(gross, taxable_value, taxable_basis, cgt_state, cgt_rate)
}

fn net_from_taxable_gross(
    gross_sale: f64,
    value_before: f64,
    basis_before: f64,
    allowance_remaining: f64,
    cgt_rate: f64,
) -> f64 {
    if gross_sale <= 0.0 || value_before <= 0.0 {
        return 0.0;
    }

    let gross = gross_sale.min(value_before);
    let basis_portion = (basis_before * (gross / value_before)).min(basis_before);
    let realized_gain = gross - basis_portion;
    if realized_gain <= 0.0 {
        return gross;
    }

    let allowance_used = allowance_remaining.max(0.0).min(realized_gain);
    let taxable_gain = (realized_gain - allowance_used).max(0.0);
    let tax = taxable_gain * cgt_rate.max(0.0);
    (gross - tax).max(0.0)
}

fn execute_taxable_sale(
    gross_sale: f64,
    taxable_value: &mut f64,
    taxable_basis: &mut f64,
    cgt_state: &mut CgtState,
    cgt_rate: f64,
) -> f64 {
    if gross_sale <= 0.0 || *taxable_value <= 0.0 {
        return 0.0;
    }

    let gross = gross_sale.min(*taxable_value);
    let value_before = *taxable_value;
    let basis_before = *taxable_basis;

    let basis_portion = (basis_before * (gross / value_before)).min(basis_before);
    let realized_gain = gross - basis_portion;

    *taxable_value -= gross;
    *taxable_basis = (basis_before - basis_portion).max(0.0).min(*taxable_value);

    if realized_gain <= 0.0 {
        return gross;
    }

    let allowance_used = cgt_state.allowance_remaining.min(realized_gain).max(0.0);
    cgt_state.allowance_remaining = (cgt_state.allowance_remaining - allowance_used).max(0.0);

    let taxable_gain = (realized_gain - allowance_used).max(0.0);
    let tax = taxable_gain * cgt_rate.max(0.0);
    cgt_state.tax_paid += tax;
    (gross - tax).max(0.0)
}

fn realized_real_return(start_invested: f64, end_invested: f64, inflation: f64) -> f64 {
    if start_invested <= 0.0 {
        return 0.0;
    }

    let nominal_return = (end_invested / start_invested).max(0.0) - 1.0;
    ((1.0 + nominal_return) / (1.0 + inflation)) - 1.0
}

fn sample_market(inputs: &Inputs, rng: &mut Rng) -> MarketSample {
    let z1 = rng.standard_normal();
    let z2 = rng.standard_normal();
    let z3 = rng.standard_normal();

    let corr = inputs.return_correlation;
    let orth = (1.0 - corr * corr).sqrt();

    let isa_return = (inputs.isa_return_mean + inputs.isa_return_vol * z1).clamp(-0.95, 2.5);
    let taxable_return =
        (inputs.taxable_return_mean + inputs.taxable_return_vol * z1).clamp(-0.95, 2.5);
    let pension_return = (inputs.pension_return_mean
        + inputs.pension_return_vol * (corr * z1 + orth * z2))
        .clamp(-0.95, 2.5);
    let inflation = (inputs.inflation_mean + inputs.inflation_vol * z3).clamp(-0.03, 0.20);

    MarketSample {
        isa_return,
        taxable_return,
        pension_return,
        inflation,
    }
}

fn derive_seed(base_seed: u64, age: u32, scenario_id: u32) -> u64 {
    let mixed = base_seed ^ ((age as u64) << 32) ^ scenario_id as u64;
    splitmix64(mixed)
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

struct Rng {
    state: u64,
    cached_normal: Option<f64>,
}

impl Rng {
    fn new(seed: u64) -> Self {
        let state = if seed == 0 {
            0xA5A5_A5A5_A5A5_A5A5
        } else {
            seed
        };
        Self {
            state,
            cached_normal: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn next_f64(&mut self) -> f64 {
        const DENOM: f64 = (1_u64 << 53) as f64;
        let v = self.next_u64() >> 11;
        ((v as f64) + 0.5) / DENOM
    }

    fn standard_normal(&mut self) -> f64 {
        if let Some(z) = self.cached_normal.take() {
            return z;
        }

        let u1 = self.next_f64().max(1e-12);
        let u2 = self.next_f64();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * PI * u2;

        let z0 = r * theta.cos();
        let z1 = r * theta.sin();
        self.cached_normal = Some(z1);
        z0
    }
}

fn percentile(values: &mut [f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    values.sort_by(|a, b| a.total_cmp(b));

    let n = values.len();
    if n == 1 {
        return values[0];
    }

    let rank = (p / 100.0) * (n as f64 - 1.0);
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;

    if lower == upper {
        values[lower]
    } else {
        let w = rank - lower as f64;
        values[lower] * (1.0 - w) + values[upper] * w
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::{any, prop_assert, prop_assume, proptest};

    const EPS: f64 = 1e-6;

    fn assert_approx(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPS,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_approx_tol(actual: f64, expected: f64, tol: f64) {
        assert!(
            (actual - expected).abs() <= tol,
            "expected {expected}, got {actual}, tolerance {tol}"
        );
    }

    fn sample_inputs() -> Inputs {
        Inputs {
            current_age: 30,
            pension_access_age: 57,
            isa_start: 100_000.0,
            taxable_start: 15_000.0,
            taxable_cost_basis_start: 12_000.0,
            pension_start: 200_000.0,
            cash_start: 0.0,
            bond_ladder_start: 0.0,
            isa_annual_contribution: 30_000.0,
            isa_annual_contribution_limit: 20_000.0,
            taxable_annual_contribution: 5_000.0,
            pension_annual_contribution: 0.0,
            contribution_growth_rate: 0.0,
            isa_return_mean: 0.08,
            isa_return_vol: 0.12,
            taxable_return_mean: 0.07,
            taxable_return_vol: 0.10,
            pension_return_mean: 0.08,
            pension_return_vol: 0.12,
            return_correlation: 0.8,
            capital_gains_tax_rate: 0.20,
            capital_gains_allowance: 3_000.0,
            taxable_return_tax_drag: 0.01,
            pension_tax_mode: PensionTaxMode::FlatRate,
            pension_flat_tax_rate: 0.20,
            uk_personal_allowance: 12_570.0,
            uk_basic_rate_limit: 50_270.0,
            uk_higher_rate_limit: 125_140.0,
            uk_basic_rate: 0.20,
            uk_higher_rate: 0.40,
            uk_additional_rate: 0.45,
            uk_allowance_taper_start: 100_000.0,
            uk_allowance_taper_end: 125_140.0,
            state_pension_start_age: 67,
            state_pension_annual_income: 0.0,
            inflation_mean: 0.025,
            inflation_vol: 0.01,
            target_annual_income: 50_000.0,
            mortgage_annual_payment: 0.0,
            mortgage_end_age: None,
            max_retirement_age: 70,
            horizon_age: 90,
            simulations: 500,
            success_threshold: 0.90,
            seed: 42,
            bad_year_threshold: -0.05,
            good_year_threshold: 0.10,
            bad_year_cut: 0.10,
            good_year_raise: 0.05,
            min_income_floor: 0.80,
            max_income_ceiling: 2.0,
            withdrawal_strategy: WithdrawalStrategy::Guardrails,
            gk_lower_guardrail: 0.8,
            gk_upper_guardrail: 1.2,
            vpw_expected_real_return: 0.035,
            floor_upside_capture: 0.5,
            bucket_target_years: 2.0,
            good_year_extra_buffer_withdrawal: 0.10,
            cash_growth_rate: 0.01,
            bond_ladder_yield: 0.03,
            bond_ladder_years: 10,
            post_access_withdrawal_order: WithdrawalOrder::ProRata,
        }
    }

    fn deterministic_oracle_inputs() -> Inputs {
        let mut inputs = sample_inputs();
        inputs.current_age = 30;
        inputs.max_retirement_age = 30;
        inputs.horizon_age = 31;
        inputs.pension_access_age = 30;
        inputs.simulations = 1;
        inputs.seed = 7;

        inputs.isa_annual_contribution = 0.0;
        inputs.taxable_annual_contribution = 0.0;
        inputs.pension_annual_contribution = 0.0;
        inputs.contribution_growth_rate = 0.0;

        inputs.isa_return_mean = 0.0;
        inputs.taxable_return_mean = 0.0;
        inputs.pension_return_mean = 0.0;
        inputs.isa_return_vol = 0.0;
        inputs.taxable_return_vol = 0.0;
        inputs.pension_return_vol = 0.0;
        inputs.inflation_mean = 0.0;
        inputs.inflation_vol = 0.0;
        inputs.cash_growth_rate = 0.0;
        inputs.taxable_return_tax_drag = 0.0;

        inputs.target_annual_income = 0.0;
        inputs.mortgage_annual_payment = 0.0;
        inputs.mortgage_end_age = None;

        inputs.capital_gains_tax_rate = 0.0;
        inputs.capital_gains_allowance = 0.0;
        inputs.pension_tax_mode = PensionTaxMode::FlatRate;
        inputs.pension_flat_tax_rate = 0.0;
        inputs.state_pension_start_age = 200;
        inputs.state_pension_annual_income = 0.0;

        inputs.withdrawal_strategy = WithdrawalStrategy::Guardrails;
        inputs.bad_year_threshold = -1.0;
        inputs.good_year_threshold = 1.0;
        inputs.bad_year_cut = 0.0;
        inputs.good_year_raise = 0.0;
        inputs.min_income_floor = 1.0;
        inputs.max_income_ceiling = 1.0;
        inputs.good_year_extra_buffer_withdrawal = 0.0;
        inputs.post_access_withdrawal_order = WithdrawalOrder::IsaFirst;
        inputs.bond_ladder_start = 0.0;
        inputs.bond_ladder_yield = 0.0;
        inputs.bond_ladder_years = 0;
        inputs
    }

    fn configure_metamorphic_inputs(inputs: &mut Inputs) {
        inputs.withdrawal_strategy = WithdrawalStrategy::Guardrails;
        inputs.bad_year_threshold = -2.0;
        inputs.good_year_threshold = 2.0;
        inputs.bad_year_cut = 0.0;
        inputs.good_year_raise = 0.0;
        inputs.min_income_floor = 1.0;
        inputs.max_income_ceiling = 1.0;
        inputs.good_year_extra_buffer_withdrawal = 0.0;

        inputs.capital_gains_tax_rate = 0.0;
        inputs.capital_gains_allowance = 0.0;
        inputs.taxable_return_tax_drag = 0.0;
        inputs.pension_tax_mode = PensionTaxMode::FlatRate;
        inputs.pension_flat_tax_rate = 0.0;
        inputs.state_pension_start_age = 200;
        inputs.state_pension_annual_income = 0.0;
        inputs.mortgage_annual_payment = 0.0;
        inputs.mortgage_end_age = None;
        inputs.cash_growth_rate = 0.0;
        inputs.bond_ladder_start = 0.0;
        inputs.bond_ladder_yield = 0.0;
        inputs.bond_ladder_years = 0;
        inputs.post_access_withdrawal_order = WithdrawalOrder::ProRata;
    }

    fn trace_row_is_all_zero(row: &YearTracePoint) -> bool {
        [
            row.contribution_isa_real,
            row.contribution_taxable_real,
            row.contribution_pension_real,
            row.contribution_total_real,
            row.withdrawal_portfolio_real,
            row.withdrawal_non_pension_income_real,
            row.spending_total_real,
            row.tax_cgt_real,
            row.tax_income_real,
            row.tax_total_real,
            row.end_isa_real,
            row.end_taxable_real,
            row.end_pension_real,
            row.end_cash_real,
            row.end_bond_ladder_real,
            row.end_total_real,
        ]
        .iter()
        .all(|v| v.abs() <= 1e-9)
    }

    fn assert_models_approx_equal(left: &ModelResult, right: &ModelResult) {
        assert_eq!(left.selected_index, right.selected_index);
        assert_eq!(left.best_index, right.best_index);
        assert_eq!(left.age_results.len(), right.age_results.len());

        for (a, b) in left.age_results.iter().zip(right.age_results.iter()) {
            assert_eq!(a.retirement_age, b.retirement_age);
            for (label, l, r) in [
                ("success_rate", a.success_rate, b.success_rate),
                (
                    "median_retirement_pot",
                    a.median_retirement_pot,
                    b.median_retirement_pot,
                ),
                (
                    "p10_retirement_pot",
                    a.p10_retirement_pot,
                    b.p10_retirement_pot,
                ),
                (
                    "median_retirement_isa",
                    a.median_retirement_isa,
                    b.median_retirement_isa,
                ),
                (
                    "p10_retirement_isa",
                    a.p10_retirement_isa,
                    b.p10_retirement_isa,
                ),
                (
                    "median_retirement_taxable",
                    a.median_retirement_taxable,
                    b.median_retirement_taxable,
                ),
                (
                    "p10_retirement_taxable",
                    a.p10_retirement_taxable,
                    b.p10_retirement_taxable,
                ),
                (
                    "median_retirement_pension",
                    a.median_retirement_pension,
                    b.median_retirement_pension,
                ),
                (
                    "p10_retirement_pension",
                    a.p10_retirement_pension,
                    b.p10_retirement_pension,
                ),
                (
                    "median_retirement_cash",
                    a.median_retirement_cash,
                    b.median_retirement_cash,
                ),
                (
                    "p10_retirement_cash",
                    a.p10_retirement_cash,
                    b.p10_retirement_cash,
                ),
                (
                    "median_retirement_bond_ladder",
                    a.median_retirement_bond_ladder,
                    b.median_retirement_bond_ladder,
                ),
                (
                    "p10_retirement_bond_ladder",
                    a.p10_retirement_bond_ladder,
                    b.p10_retirement_bond_ladder,
                ),
                (
                    "median_terminal_pot",
                    a.median_terminal_pot,
                    b.median_terminal_pot,
                ),
                ("p10_terminal_pot", a.p10_terminal_pot, b.p10_terminal_pot),
                (
                    "median_terminal_isa",
                    a.median_terminal_isa,
                    b.median_terminal_isa,
                ),
                ("p10_terminal_isa", a.p10_terminal_isa, b.p10_terminal_isa),
                (
                    "median_terminal_taxable",
                    a.median_terminal_taxable,
                    b.median_terminal_taxable,
                ),
                (
                    "p10_terminal_taxable",
                    a.p10_terminal_taxable,
                    b.p10_terminal_taxable,
                ),
                (
                    "median_terminal_pension",
                    a.median_terminal_pension,
                    b.median_terminal_pension,
                ),
                (
                    "p10_terminal_pension",
                    a.p10_terminal_pension,
                    b.p10_terminal_pension,
                ),
                (
                    "median_terminal_cash",
                    a.median_terminal_cash,
                    b.median_terminal_cash,
                ),
                (
                    "p10_terminal_cash",
                    a.p10_terminal_cash,
                    b.p10_terminal_cash,
                ),
                (
                    "median_terminal_bond_ladder",
                    a.median_terminal_bond_ladder,
                    b.median_terminal_bond_ladder,
                ),
                (
                    "p10_terminal_bond_ladder",
                    a.p10_terminal_bond_ladder,
                    b.p10_terminal_bond_ladder,
                ),
                (
                    "p10_min_income_ratio",
                    a.p10_min_income_ratio,
                    b.p10_min_income_ratio,
                ),
                (
                    "median_avg_income_ratio",
                    a.median_avg_income_ratio,
                    b.median_avg_income_ratio,
                ),
            ] {
                assert!(
                    (l - r).abs() <= 1e-9,
                    "field {label}: expected {l}, got {r}"
                );
            }
        }
    }

    fn assert_finite_non_negative(value: f64, label: &str) {
        assert!(value.is_finite(), "{label} must be finite");
        assert!(value >= -1e-6, "{label} must be non-negative");
    }

    fn assert_age_result_invariants(age: &AgeResult) {
        assert!((0.0..=1.0).contains(&age.success_rate));

        for (label, value) in [
            ("median_retirement_pot", age.median_retirement_pot),
            ("p10_retirement_pot", age.p10_retirement_pot),
            ("median_retirement_isa", age.median_retirement_isa),
            ("p10_retirement_isa", age.p10_retirement_isa),
            ("median_retirement_taxable", age.median_retirement_taxable),
            ("p10_retirement_taxable", age.p10_retirement_taxable),
            ("median_retirement_pension", age.median_retirement_pension),
            ("p10_retirement_pension", age.p10_retirement_pension),
            ("median_retirement_cash", age.median_retirement_cash),
            ("p10_retirement_cash", age.p10_retirement_cash),
            (
                "median_retirement_bond_ladder",
                age.median_retirement_bond_ladder,
            ),
            ("p10_retirement_bond_ladder", age.p10_retirement_bond_ladder),
            ("median_terminal_pot", age.median_terminal_pot),
            ("p10_terminal_pot", age.p10_terminal_pot),
            ("median_terminal_isa", age.median_terminal_isa),
            ("p10_terminal_isa", age.p10_terminal_isa),
            ("median_terminal_taxable", age.median_terminal_taxable),
            ("p10_terminal_taxable", age.p10_terminal_taxable),
            ("median_terminal_pension", age.median_terminal_pension),
            ("p10_terminal_pension", age.p10_terminal_pension),
            ("median_terminal_cash", age.median_terminal_cash),
            ("p10_terminal_cash", age.p10_terminal_cash),
            (
                "median_terminal_bond_ladder",
                age.median_terminal_bond_ladder,
            ),
            ("p10_terminal_bond_ladder", age.p10_terminal_bond_ladder),
            ("p10_min_income_ratio", age.p10_min_income_ratio),
            ("median_avg_income_ratio", age.median_avg_income_ratio),
        ] {
            assert_finite_non_negative(value, label);
        }

        assert!(age.p10_retirement_pot <= age.median_retirement_pot + 1e-6);
        assert!(age.p10_retirement_isa <= age.median_retirement_isa + 1e-6);
        assert!(age.p10_retirement_taxable <= age.median_retirement_taxable + 1e-6);
        assert!(age.p10_retirement_pension <= age.median_retirement_pension + 1e-6);
        assert!(age.p10_retirement_cash <= age.median_retirement_cash + 1e-6);
        assert!(age.p10_retirement_bond_ladder <= age.median_retirement_bond_ladder + 1e-6);
        assert!(age.p10_terminal_pot <= age.median_terminal_pot + 1e-6);
        assert!(age.p10_terminal_isa <= age.median_terminal_isa + 1e-6);
        assert!(age.p10_terminal_taxable <= age.median_terminal_taxable + 1e-6);
        assert!(age.p10_terminal_pension <= age.median_terminal_pension + 1e-6);
        assert!(age.p10_terminal_cash <= age.median_terminal_cash + 1e-6);
        assert!(age.p10_terminal_bond_ladder <= age.median_terminal_bond_ladder + 1e-6);
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(24))]

        #[test]
        fn prop_run_model_outputs_are_finite_and_non_negative(
            seed in any::<u64>(),
            current_age in 25u32..56,
            retirement_span in 0u32..6,
            horizon_extra in 1u32..12,
            simulations in 5u32..24,
            pension_access_offset in 0u32..26,
            isa_start in 0u32..700_000,
            taxable_start in 0u32..500_000,
            pension_start in 0u32..900_000,
            cash_start in 0u32..120_000,
            target_income in 10_000u32..90_000,
            isa_mean_bp in -200i32..1400,
            taxable_mean_bp in -200i32..1400,
            pension_mean_bp in -200i32..1400,
            isa_vol_bp in 0u32..3500,
            taxable_vol_bp in 0u32..3500,
            pension_vol_bp in 0u32..3500,
            inflation_mean_bp in 0u32..800,
            inflation_vol_bp in 0u32..400,
            correlation_bp in -100i32..101
        ) {
            let mut inputs = sample_inputs();
            inputs.seed = seed;
            inputs.current_age = current_age;
            inputs.max_retirement_age = current_age + retirement_span;
            inputs.horizon_age = inputs.max_retirement_age + horizon_extra + 1;
            inputs.simulations = simulations;
            inputs.pension_access_age = (current_age + pension_access_offset).min(inputs.horizon_age - 1);

            inputs.isa_start = isa_start as f64;
            inputs.taxable_start = taxable_start as f64;
            inputs.taxable_cost_basis_start = inputs.taxable_start;
            inputs.pension_start = pension_start as f64;
            inputs.cash_start = cash_start as f64;
            inputs.target_annual_income = target_income as f64;

            inputs.isa_return_mean = isa_mean_bp as f64 / 10_000.0;
            inputs.taxable_return_mean = taxable_mean_bp as f64 / 10_000.0;
            inputs.pension_return_mean = pension_mean_bp as f64 / 10_000.0;
            inputs.isa_return_vol = isa_vol_bp as f64 / 10_000.0;
            inputs.taxable_return_vol = taxable_vol_bp as f64 / 10_000.0;
            inputs.pension_return_vol = pension_vol_bp as f64 / 10_000.0;
            inputs.inflation_mean = inflation_mean_bp as f64 / 10_000.0;
            inputs.inflation_vol = inflation_vol_bp as f64 / 10_000.0;
            inputs.return_correlation = correlation_bp as f64 / 100.0;

            inputs.isa_annual_contribution = 0.0;
            inputs.taxable_annual_contribution = 0.0;
            inputs.pension_annual_contribution = 0.0;
            inputs.good_year_extra_buffer_withdrawal = 0.0;

            let model = run_model(&inputs);
            prop_assert!(!model.age_results.is_empty());
            prop_assert!(model.best_index < model.age_results.len());
            if let Some(selected) = model.selected_index {
                prop_assert!(selected < model.age_results.len());
            }

            for age in &model.age_results {
                assert_age_result_invariants(age);
            }
        }
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(40))]

        #[test]
        fn prop_one_year_zero_growth_conserves_total_when_income_is_affordable(
            isa_start in 0u32..300_000,
            taxable_start in 0u32..300_000,
            pension_start in 0u32..300_000,
            cash_start in 0u32..150_000,
            income_ratio_pct in 1u32..95
        ) {
            let mut inputs = sample_inputs();
            inputs.current_age = 30;
            inputs.max_retirement_age = 30;
            inputs.horizon_age = 31;
            inputs.pension_access_age = 30;
            inputs.simulations = 1;
            inputs.seed = 99;

            inputs.isa_start = isa_start as f64;
            inputs.taxable_start = taxable_start as f64;
            inputs.taxable_cost_basis_start = inputs.taxable_start;
            inputs.pension_start = pension_start as f64;
            inputs.cash_start = cash_start as f64;

            let start_total = inputs.isa_start + inputs.taxable_start + inputs.pension_start + inputs.cash_start;
            prop_assume!(start_total > 100.0);

            inputs.target_annual_income = start_total * income_ratio_pct as f64 / 100.0;

            inputs.isa_annual_contribution = 0.0;
            inputs.taxable_annual_contribution = 0.0;
            inputs.pension_annual_contribution = 0.0;
            inputs.contribution_growth_rate = 0.0;

            inputs.isa_return_mean = 0.0;
            inputs.taxable_return_mean = 0.0;
            inputs.pension_return_mean = 0.0;
            inputs.isa_return_vol = 0.0;
            inputs.taxable_return_vol = 0.0;
            inputs.pension_return_vol = 0.0;
            inputs.inflation_mean = 0.0;
            inputs.inflation_vol = 0.0;
            inputs.cash_growth_rate = 0.0;
            inputs.taxable_return_tax_drag = 0.0;

            inputs.capital_gains_tax_rate = 0.0;
            inputs.capital_gains_allowance = 0.0;
            inputs.pension_tax_mode = PensionTaxMode::FlatRate;
            inputs.pension_flat_tax_rate = 0.0;
            inputs.state_pension_start_age = 200;
            inputs.state_pension_annual_income = 0.0;

            inputs.withdrawal_strategy = WithdrawalStrategy::Guardrails;
            inputs.bad_year_threshold = -1.0;
            inputs.good_year_threshold = 1.0;
            inputs.bad_year_cut = 0.0;
            inputs.good_year_raise = 0.0;
            inputs.min_income_floor = 1.0;
            inputs.max_income_ceiling = 1.0;
            inputs.good_year_extra_buffer_withdrawal = 0.0;

            let model = run_model(&inputs);
            let age = &model.age_results[0];
            prop_assert!((age.success_rate - 1.0).abs() < 1e-9);

            let expected_terminal = start_total - inputs.target_annual_income;
            prop_assert!((age.median_terminal_pot - expected_terminal).abs() < 1e-2);
            prop_assert!((age.p10_terminal_pot - expected_terminal).abs() < 1e-2);

            let median_terminal_sum = age.median_terminal_isa
                + age.median_terminal_taxable
                + age.median_terminal_pension
                + age.median_terminal_cash
                + age.median_terminal_bond_ladder;
            prop_assert!((median_terminal_sum - age.median_terminal_pot).abs() < 1e-3);
        }
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(24))]

        #[test]
        fn prop_trace_rows_have_finite_non_negative_balances_and_consistent_zero_suffix(
            seed in any::<u64>(),
            current_age in 25u32..56,
            retirement_span in 0u32..6,
            horizon_extra in 1u32..10,
            pension_access_offset in 0u32..26,
            isa_start in 0u32..500_000,
            taxable_start in 0u32..400_000,
            pension_start in 0u32..700_000,
            cash_start in 0u32..120_000,
            target_income in 10_000u32..100_000,
            isa_mean_bp in -500i32..1600,
            taxable_mean_bp in -500i32..1600,
            pension_mean_bp in -500i32..1600,
            isa_vol_bp in 0u32..2500,
            taxable_vol_bp in 0u32..2500,
            pension_vol_bp in 0u32..2500,
            inflation_mean_bp in 0u32..700,
            inflation_vol_bp in 0u32..300
        ) {
            let mut inputs = sample_inputs();
            inputs.seed = seed;
            inputs.simulations = 1;
            inputs.current_age = current_age;
            let retirement_age = current_age + retirement_span;
            inputs.max_retirement_age = retirement_age;
            inputs.horizon_age = retirement_age + horizon_extra + 1;
            inputs.pension_access_age = (current_age + pension_access_offset).min(inputs.horizon_age - 1);

            inputs.isa_start = isa_start as f64;
            inputs.taxable_start = taxable_start as f64;
            inputs.taxable_cost_basis_start = inputs.taxable_start;
            inputs.pension_start = pension_start as f64;
            inputs.cash_start = cash_start as f64;
            inputs.target_annual_income = target_income as f64;

            inputs.isa_return_mean = isa_mean_bp as f64 / 10_000.0;
            inputs.taxable_return_mean = taxable_mean_bp as f64 / 10_000.0;
            inputs.pension_return_mean = pension_mean_bp as f64 / 10_000.0;
            inputs.isa_return_vol = isa_vol_bp as f64 / 10_000.0;
            inputs.taxable_return_vol = taxable_vol_bp as f64 / 10_000.0;
            inputs.pension_return_vol = pension_vol_bp as f64 / 10_000.0;
            inputs.inflation_mean = inflation_mean_bp as f64 / 10_000.0;
            inputs.inflation_vol = inflation_vol_bp as f64 / 10_000.0;

            let mut trace = Vec::new();
            let mut rng = Rng::new(derive_seed(inputs.seed, retirement_age, 0));
            let scenario = simulate_scenario(
                &inputs,
                retirement_age,
                retirement_age,
                &mut rng,
                Some(&mut trace),
            );

            prop_assert!(trace.len() == (inputs.horizon_age - inputs.current_age) as usize);
            prop_assert!(scenario.success as u8 <= 1);
            for (label, value) in [
                ("reported_retirement_total", scenario.reported_retirement_total),
                ("reported_retirement_isa", scenario.reported_retirement_isa),
                ("reported_retirement_taxable", scenario.reported_retirement_taxable),
                ("reported_retirement_pension", scenario.reported_retirement_pension),
                ("reported_retirement_cash", scenario.reported_retirement_cash),
                (
                    "reported_retirement_bond_ladder",
                    scenario.reported_retirement_bond_ladder,
                ),
                ("reported_terminal_total", scenario.reported_terminal_total),
                ("reported_terminal_isa", scenario.reported_terminal_isa),
                ("reported_terminal_taxable", scenario.reported_terminal_taxable),
                ("reported_terminal_pension", scenario.reported_terminal_pension),
                ("reported_terminal_cash", scenario.reported_terminal_cash),
                (
                    "reported_terminal_bond_ladder",
                    scenario.reported_terminal_bond_ladder,
                ),
                ("min_income_ratio", scenario.min_income_ratio),
                ("avg_income_ratio", scenario.avg_income_ratio),
            ] {
                prop_assert!(value.is_finite(), "{label} must be finite");
                prop_assert!(value >= -1e-6, "{label} must be non-negative");
            }

            let mut saw_zero_row = false;
            for row in &trace {
                for value in [
                    row.contribution_isa_real,
                    row.contribution_taxable_real,
                    row.contribution_pension_real,
                    row.contribution_total_real,
                    row.withdrawal_portfolio_real,
                    row.withdrawal_non_pension_income_real,
                    row.spending_total_real,
                    row.tax_cgt_real,
                    row.tax_income_real,
                    row.tax_total_real,
                    row.end_isa_real,
                    row.end_taxable_real,
                    row.end_pension_real,
                    row.end_cash_real,
                    row.end_bond_ladder_real,
                    row.end_total_real,
                ] {
                    prop_assert!(value.is_finite());
                }

                prop_assert!(row.end_isa_real >= -1e-6);
                prop_assert!(row.end_taxable_real >= -1e-6);
                prop_assert!(row.end_pension_real >= -1e-6);
                prop_assert!(row.end_cash_real >= -1e-6);
                prop_assert!(row.end_bond_ladder_real >= -1e-6);
                prop_assert!(row.end_total_real >= -1e-6);

                let reconstructed_end = row.end_isa_real
                    + row.end_taxable_real
                    + row.end_pension_real
                    + row.end_cash_real
                    + row.end_bond_ladder_real;
                prop_assert!((row.end_total_real - reconstructed_end).abs() <= 1e-4);

                if row.end_total_real <= 1e-9 {
                    prop_assert!(row.end_isa_real.abs() <= 1e-6);
                    prop_assert!(row.end_taxable_real.abs() <= 1e-6);
                    prop_assert!(row.end_pension_real.abs() <= 1e-6);
                    prop_assert!(row.end_cash_real.abs() <= 1e-6);
                    prop_assert!(row.end_bond_ladder_real.abs() <= 1e-6);
                }

                let all_zero = trace_row_is_all_zero(row);
                if saw_zero_row {
                    prop_assert!(all_zero);
                } else if all_zero {
                    saw_zero_row = true;
                }
            }
        }
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(28))]

        #[test]
        fn prop_pre_retirement_yearly_accounting_identity_per_pot(
            years in 2u32..7,
            isa_start in 0u32..200_000,
            taxable_start in 0u32..200_000,
            pension_start in 0u32..300_000,
            taxable_basis_pct in 0u32..101,
            isa_contrib in 0u32..50_000,
            taxable_contrib in 0u32..30_000,
            pension_contrib in 0u32..20_000,
            isa_limit in 5_000u32..40_001,
            contribution_growth_bp in 0u32..1_001,
            isa_return_bp in -500i32..1_501,
            taxable_return_bp in -500i32..1_501,
            pension_return_bp in -500i32..1_501,
            tax_drag_bp in 0u32..501
        ) {
            let mut inputs = deterministic_oracle_inputs();
            inputs.current_age = 30;
            inputs.max_retirement_age = 30 + years;
            inputs.horizon_age = 30 + years;
            inputs.pension_access_age = 80;
            inputs.seed = 11;
            inputs.simulations = 1;

            inputs.isa_start = isa_start as f64;
            inputs.taxable_start = taxable_start as f64;
            inputs.taxable_cost_basis_start =
                inputs.taxable_start * (taxable_basis_pct as f64 / 100.0);
            inputs.pension_start = pension_start as f64;
            inputs.cash_start = 0.0;

            inputs.isa_annual_contribution = isa_contrib as f64;
            inputs.taxable_annual_contribution = taxable_contrib as f64;
            inputs.pension_annual_contribution = pension_contrib as f64;
            inputs.isa_annual_contribution_limit = isa_limit as f64;
            inputs.contribution_growth_rate = contribution_growth_bp as f64 / 10_000.0;

            inputs.isa_return_mean = isa_return_bp as f64 / 10_000.0;
            inputs.taxable_return_mean = taxable_return_bp as f64 / 10_000.0;
            inputs.pension_return_mean = pension_return_bp as f64 / 10_000.0;
            inputs.isa_return_vol = 0.0;
            inputs.taxable_return_vol = 0.0;
            inputs.pension_return_vol = 0.0;
            inputs.inflation_mean = 0.0;
            inputs.inflation_vol = 0.0;
            inputs.taxable_return_tax_drag = tax_drag_bp as f64 / 10_000.0;
            inputs.target_annual_income = 0.0;

            let rows = run_yearly_cashflow_trace(
                &inputs,
                inputs.max_retirement_age,
                inputs.max_retirement_age,
                inputs.max_retirement_age,
            );
            prop_assert!(rows.len() == years as usize);

            let mut expected_isa = inputs.isa_start;
            let mut expected_taxable = inputs.taxable_start;
            let mut expected_pension = inputs.pension_start;
            let mut expected_bond_ladder = inputs.bond_ladder_start;

            for (year, row) in rows.iter().enumerate() {
                let y = year as u32;

                let isa_after_growth = (expected_isa * (1.0 + inputs.isa_return_mean)).max(0.0);
                let taxable_after_growth =
                    (expected_taxable * (1.0 + inputs.taxable_return_mean)).max(0.0);
                let taxable_after_growth =
                    (taxable_after_growth * (1.0 - inputs.taxable_return_tax_drag)).max(0.0);
                let pension_after_growth =
                    (expected_pension * (1.0 + inputs.pension_return_mean)).max(0.0);
                let bond_ladder_after_growth =
                    (expected_bond_ladder * (1.0 + inputs.bond_ladder_yield)).max(0.0);

                let multiplier = (1.0 + inputs.contribution_growth_rate).powi(y as i32);
                let requested_isa = inputs.isa_annual_contribution * multiplier;
                let requested_taxable = inputs.taxable_annual_contribution * multiplier;
                let requested_pension = inputs.pension_annual_contribution * multiplier;

                let isa_add = requested_isa.max(0.0).min(inputs.isa_annual_contribution_limit);
                let overflow = (requested_isa - isa_add).max(0.0);
                let taxable_add = requested_taxable.max(0.0) + overflow;
                let pension_add = requested_pension.max(0.0);

                let expected_isa_end = isa_after_growth + isa_add;
                let expected_taxable_end = taxable_after_growth + taxable_add;
                let expected_pension_end = pension_after_growth + pension_add;

                prop_assert!((row.median_contribution_isa - isa_add).abs() <= 1e-6);
                prop_assert!((row.median_contribution_taxable - taxable_add).abs() <= 1e-6);
                prop_assert!((row.median_contribution_pension - pension_add).abs() <= 1e-6);
                prop_assert!(
                    (row.median_contribution_total - (isa_add + taxable_add + pension_add)).abs()
                        <= 1e-6
                );
                prop_assert!((row.median_end_isa - expected_isa_end).abs() <= 1e-6);
                prop_assert!((row.median_end_taxable - expected_taxable_end).abs() <= 1e-6);
                prop_assert!((row.median_end_pension - expected_pension_end).abs() <= 1e-6);
                prop_assert!((row.median_end_bond_ladder - bond_ladder_after_growth).abs() <= 1e-6);

                let expected_total = expected_isa_end
                    + expected_taxable_end
                    + expected_pension_end
                    + bond_ladder_after_growth;
                prop_assert!((row.median_end_total - expected_total).abs() <= 1e-6);

                expected_isa = expected_isa_end;
                expected_taxable = expected_taxable_end;
                expected_pension = expected_pension_end;
                expected_bond_ladder = bond_ladder_after_growth;
            }
        }
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(24))]

        #[test]
        fn prop_single_retirement_year_identity_holds_with_withdrawals_taxes_and_growth(
            isa_start in 0u32..300_000,
            taxable_start in 0u32..300_000,
            taxable_basis_pct in 0u32..101,
            pension_start in 0u32..300_000,
            cash_start in 0u32..100_000,
            isa_return_bp in -300i32..1_201,
            taxable_return_bp in -300i32..1_201,
            pension_return_bp in -300i32..1_201,
            cash_growth_bp in 0u32..501,
            cgt_rate_bp in 0u32..3_501,
            cgt_allowance in 0u32..20_000,
            pension_tax_bp in 0u32..4_501,
            spend_ratio_pct in 5u32..71
        ) {
            let mut inputs = deterministic_oracle_inputs();
            inputs.current_age = 30;
            inputs.max_retirement_age = 30;
            inputs.horizon_age = 31;
            inputs.pension_access_age = 30;
            inputs.seed = 19;
            inputs.simulations = 1;

            inputs.isa_start = isa_start as f64;
            inputs.taxable_start = taxable_start as f64;
            inputs.taxable_cost_basis_start =
                inputs.taxable_start * (taxable_basis_pct as f64 / 100.0);
            inputs.pension_start = pension_start as f64;
            inputs.cash_start = cash_start as f64;

            inputs.isa_return_mean = isa_return_bp as f64 / 10_000.0;
            inputs.taxable_return_mean = taxable_return_bp as f64 / 10_000.0;
            inputs.pension_return_mean = pension_return_bp as f64 / 10_000.0;
            inputs.isa_return_vol = 0.0;
            inputs.taxable_return_vol = 0.0;
            inputs.pension_return_vol = 0.0;
            inputs.inflation_mean = 0.0;
            inputs.inflation_vol = 0.0;
            inputs.cash_growth_rate = cash_growth_bp as f64 / 10_000.0;

            inputs.capital_gains_tax_rate = cgt_rate_bp as f64 / 10_000.0;
            inputs.capital_gains_allowance = cgt_allowance as f64;
            inputs.pension_tax_mode = PensionTaxMode::FlatRate;
            inputs.pension_flat_tax_rate = pension_tax_bp as f64 / 10_000.0;
            inputs.state_pension_start_age = 200;
            inputs.state_pension_annual_income = 0.0;

            inputs.withdrawal_strategy = WithdrawalStrategy::Guardrails;
            inputs.bad_year_threshold = -2.0;
            inputs.good_year_threshold = 2.0;
            inputs.bad_year_cut = 0.0;
            inputs.good_year_raise = 0.0;
            inputs.min_income_floor = 1.0;
            inputs.max_income_ceiling = 1.0;
            inputs.good_year_extra_buffer_withdrawal = 0.0;
            inputs.post_access_withdrawal_order = WithdrawalOrder::ProRata;

            let tax_state0 = TaxYearState {
                non_pension_taxable_income: 0.0,
                pension_gross_withdrawn: 0.0,
                price_index: 1.0,
            };
            let taxable_net_capacity = net_from_taxable_gross(
                inputs.taxable_start,
                inputs.taxable_start,
                inputs.taxable_cost_basis_start,
                inputs.capital_gains_allowance,
                inputs.capital_gains_tax_rate,
            );
            let pension_net_capacity =
                net_from_additional_pension_gross(inputs.pension_start, &tax_state0, &inputs);
            let net_capacity =
                inputs.cash_start
                    + inputs.isa_start
                    + inputs.bond_ladder_start
                    + taxable_net_capacity
                    + pension_net_capacity;
            prop_assume!(net_capacity > 10.0);

            inputs.target_annual_income = net_capacity * spend_ratio_pct as f64 / 100.0;

            let mut trace = Vec::new();
            let mut rng = Rng::new(derive_seed(inputs.seed, 30, 0));
            let scenario = simulate_scenario(&inputs, 30, 30, &mut rng, Some(&mut trace));
            prop_assume!(scenario.success);
            prop_assert!(trace.len() == 1);

            let sampled = MarketSample {
                isa_return: inputs.isa_return_mean,
                taxable_return: inputs.taxable_return_mean,
                pension_return: inputs.pension_return_mean,
                inflation: inputs.inflation_mean,
            };
            let mut portfolio = Portfolio {
                isa: inputs.isa_start,
                taxable: inputs.taxable_start,
                taxable_basis: inputs.taxable_cost_basis_start,
                pension: inputs.pension_start,
                cash_buffer: inputs.cash_start,
                bond_ladder: inputs.bond_ladder_start,
            };

            let total_start = portfolio.isa
                + portfolio.taxable
                + portfolio.pension
                + portfolio.cash_buffer
                + portfolio.bond_ladder;
            let mut spending_state = SpendingState {
                current_real_spending: inputs.target_annual_income,
                initial_withdrawal_rate: inputs.target_annual_income / total_start.max(1e-9),
            };
            let planned_real_spending = plan_real_spending(
                &inputs,
                30,
                0.0,
                available_spendable_real(&inputs, 30, &portfolio, 1.0),
                &mut spending_state,
            );
            let planned_nominal_spending = planned_real_spending;

            let mut cgt_state = CgtState {
                allowance_remaining: inputs.capital_gains_allowance,
                tax_paid: 0.0,
            };
            let mut tax_state = TaxYearState {
                non_pension_taxable_income: 0.0,
                pension_gross_withdrawn: 0.0,
                price_index: 1.0,
            };

            let start_isa = portfolio.isa;
            let start_taxable = portfolio.taxable;
            let start_pension = portfolio.pension;
            let start_cash = portfolio.cash_buffer;

            let outcome = run_withdrawal_year(
                &inputs,
                30,
                0,
                planned_nominal_spending,
                0.0,
                planned_real_spending,
                &mut portfolio,
                &mut cgt_state,
                &mut tax_state,
                0.0,
            );
            prop_assert!(outcome.realized_spending_net + 1e-6 >= planned_nominal_spending);

            let after_withdraw_isa = portfolio.isa;
            let after_withdraw_taxable = portfolio.taxable;
            let after_withdraw_pension = portfolio.pension;
            let after_withdraw_cash = portfolio.cash_buffer;
            let after_withdraw_bond_ladder = portfolio.bond_ladder;

            let isa_withdrawn_gross = (start_isa - after_withdraw_isa).max(0.0);
            let taxable_withdrawn_gross = (start_taxable - after_withdraw_taxable).max(0.0);
            let pension_withdrawn_gross = (start_pension - after_withdraw_pension).max(0.0);
            let cash_used = (start_cash - after_withdraw_cash).max(0.0);
            let start_bond_ladder = inputs.bond_ladder_start;
            let bond_ladder_withdrawn_gross = (start_bond_ladder - after_withdraw_bond_ladder).max(0.0);

            let portfolio_withdrawn_gross =
                isa_withdrawn_gross
                    + taxable_withdrawn_gross
                    + pension_withdrawn_gross
                    + bond_ladder_withdrawn_gross;
            prop_assert!(
                (portfolio_withdrawn_gross
                    - (outcome.portfolio_withdrawn_net + outcome.total_tax_paid()))
                .abs()
                    <= 1e-3
            );

            apply_post_retirement_growth(&inputs, &mut portfolio, &sampled);

            let expected_isa_end = (after_withdraw_isa * (1.0 + inputs.isa_return_mean)).max(0.0);
            let expected_taxable_end = ((after_withdraw_taxable
                * (1.0 + inputs.taxable_return_mean))
            .max(0.0)
                * (1.0 - inputs.taxable_return_tax_drag))
                .max(0.0);
            let expected_pension_end =
                (after_withdraw_pension * (1.0 + inputs.pension_return_mean)).max(0.0);
            let expected_cash_end = (after_withdraw_cash * (1.0 + inputs.cash_growth_rate)).max(0.0);
            let expected_bond_ladder_end =
                (after_withdraw_bond_ladder * (1.0 + inputs.bond_ladder_yield)).max(0.0);

            prop_assert!((portfolio.isa - expected_isa_end).abs() <= 1e-6);
            prop_assert!((portfolio.taxable - expected_taxable_end).abs() <= 1e-6);
            prop_assert!((portfolio.pension - expected_pension_end).abs() <= 1e-6);
            prop_assert!((portfolio.cash_buffer - expected_cash_end).abs() <= 1e-6);
            prop_assert!((portfolio.bond_ladder - expected_bond_ladder_end).abs() <= 1e-6);

            // Pot-level accounting identity:
            // end = start + contributions(0) - gross_withdrawals + market_move
            let isa_market_move = expected_isa_end - after_withdraw_isa;
            let taxable_market_move = expected_taxable_end - after_withdraw_taxable;
            let pension_market_move = expected_pension_end - after_withdraw_pension;
            let cash_market_move = expected_cash_end - after_withdraw_cash;
            let bond_ladder_market_move = expected_bond_ladder_end - after_withdraw_bond_ladder;

            let isa_identity_end = start_isa - isa_withdrawn_gross + isa_market_move;
            let taxable_identity_end = start_taxable - taxable_withdrawn_gross + taxable_market_move;
            let pension_identity_end = start_pension - pension_withdrawn_gross + pension_market_move;
            let cash_identity_end = start_cash - cash_used + cash_market_move;
            let bond_ladder_identity_end =
                start_bond_ladder - bond_ladder_withdrawn_gross + bond_ladder_market_move;

            prop_assert!((isa_identity_end - expected_isa_end).abs() <= 1e-6);
            prop_assert!((taxable_identity_end - expected_taxable_end).abs() <= 1e-6);
            prop_assert!((pension_identity_end - expected_pension_end).abs() <= 1e-6);
            prop_assert!((cash_identity_end - expected_cash_end).abs() <= 1e-6);
            prop_assert!((bond_ladder_identity_end - expected_bond_ladder_end).abs() <= 1e-6);

            let row = &trace[0];
            prop_assert!((row.withdrawal_portfolio_real - outcome.portfolio_withdrawn_net).abs() <= 1e-4);
            prop_assert!((row.tax_total_real - outcome.total_tax_paid()).abs() <= 1e-4);
            prop_assert!((row.spending_total_real - outcome.realized_spending_net).abs() <= 1e-4);
            prop_assert!((row.end_isa_real - expected_isa_end).abs() <= 1e-6);
            prop_assert!((row.end_taxable_real - expected_taxable_end).abs() <= 1e-6);
            prop_assert!((row.end_pension_real - expected_pension_end).abs() <= 1e-6);
            prop_assert!((row.end_cash_real - expected_cash_end).abs() <= 1e-6);
            prop_assert!((row.end_bond_ladder_real - expected_bond_ladder_end).abs() <= 1e-6);
        }
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(24))]

        #[test]
        fn prop_terminal_best_decile_bounds_median_and_worst_decile(
            seed in any::<u64>(),
            current_age in 25u32..56,
            retirement_span in 0u32..6,
            horizon_extra in 1u32..10,
            simulations in 10u32..64,
            target_income in 10_000u32..90_000
        ) {
            let mut inputs = sample_inputs();
            inputs.seed = seed;
            inputs.current_age = current_age;
            let retirement_age = current_age + retirement_span;
            inputs.max_retirement_age = retirement_age;
            inputs.horizon_age = retirement_age + horizon_extra + 1;
            inputs.pension_access_age = (current_age + 20).min(inputs.horizon_age - 1);
            inputs.simulations = simulations;
            inputs.target_annual_income = target_income as f64;

            let age_result =
                evaluate_age_candidate(&inputs, retirement_age, retirement_age, retirement_age);

            let mut terminal_totals = Vec::with_capacity(inputs.simulations as usize);
            for scenario_id in 0..inputs.simulations {
                let scenario_seed = derive_seed(inputs.seed, retirement_age, scenario_id);
                let mut rng = Rng::new(scenario_seed);
                let scenario =
                    simulate_scenario(&inputs, retirement_age, retirement_age, &mut rng, None);
                terminal_totals.push(scenario.reported_terminal_total);
            }

            let p90_terminal = percentile(&mut terminal_totals, 90.0);
            prop_assert!(age_result.p10_terminal_pot <= age_result.median_terminal_pot + 1e-9);
            prop_assert!(age_result.median_terminal_pot <= p90_terminal + 1e-9);
            prop_assert!(age_result.p10_terminal_pot <= p90_terminal + 1e-9);
        }
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(16))]

        #[test]
        fn prop_higher_contributions_do_not_reduce_success_rate(
            seed in any::<u64>(),
            current_age in 30u32..51,
            retirement_span in 0u32..7,
            horizon_extra in 1u32..12,
            simulations in 24u32..72,
            isa_start in 0u32..300_000,
            taxable_start in 0u32..200_000,
            pension_start in 0u32..400_000,
            target_income in 10_000u32..80_000,
            return_mean_bp in -200i32..1001,
            return_vol_bp in 500u32..2_001,
            base_isa_contrib in 0u32..20_000,
            base_taxable_contrib in 0u32..20_000,
            base_pension_contrib in 0u32..20_000,
            contribution_delta in 1u32..10_001
        ) {
            let mut low = sample_inputs();
            configure_metamorphic_inputs(&mut low);
            low.seed = seed;
            low.current_age = current_age;
            low.max_retirement_age = current_age + retirement_span;
            low.horizon_age = low.max_retirement_age + horizon_extra + 1;
            low.pension_access_age = (current_age + 20).min(low.horizon_age - 1);
            low.simulations = simulations;

            low.isa_start = isa_start as f64;
            low.taxable_start = taxable_start as f64;
            low.taxable_cost_basis_start = low.taxable_start;
            low.pension_start = pension_start as f64;
            low.target_annual_income = target_income as f64;

            let mean = return_mean_bp as f64 / 10_000.0;
            let vol = return_vol_bp as f64 / 10_000.0;
            low.isa_return_mean = mean;
            low.taxable_return_mean = mean;
            low.pension_return_mean = mean;
            low.isa_return_vol = vol;
            low.taxable_return_vol = vol;
            low.pension_return_vol = vol;

            low.isa_annual_contribution = base_isa_contrib as f64;
            low.taxable_annual_contribution = base_taxable_contrib as f64;
            low.pension_annual_contribution = base_pension_contrib as f64;

            let mut high = low.clone();
            let delta = contribution_delta as f64;
            high.isa_annual_contribution += delta;
            high.taxable_annual_contribution += delta;
            high.pension_annual_contribution += delta;

            let low_model = run_model(&low);
            let high_model = run_model(&high);

            for (low_age, high_age) in low_model.age_results.iter().zip(high_model.age_results.iter()) {
                prop_assert!(high_age.success_rate + 1e-9 >= low_age.success_rate);
            }
        }
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(16))]

        #[test]
        fn prop_higher_expected_returns_do_not_reduce_success_rate(
            seed in any::<u64>(),
            current_age in 30u32..51,
            retirement_span in 0u32..7,
            horizon_extra in 1u32..12,
            simulations in 24u32..72,
            isa_start in 0u32..300_000,
            taxable_start in 0u32..200_000,
            pension_start in 0u32..400_000,
            target_income in 10_000u32..80_000,
            return_mean_bp in -200i32..901,
            return_vol_bp in 500u32..2_001,
            mean_delta_bp in 1u32..401
        ) {
            let mut lower = sample_inputs();
            configure_metamorphic_inputs(&mut lower);
            lower.seed = seed;
            lower.current_age = current_age;
            lower.max_retirement_age = current_age + retirement_span;
            lower.horizon_age = lower.max_retirement_age + horizon_extra + 1;
            lower.pension_access_age = (current_age + 20).min(lower.horizon_age - 1);
            lower.simulations = simulations;

            lower.isa_start = isa_start as f64;
            lower.taxable_start = taxable_start as f64;
            lower.taxable_cost_basis_start = lower.taxable_start;
            lower.pension_start = pension_start as f64;
            lower.target_annual_income = target_income as f64;

            let base_mean = return_mean_bp as f64 / 10_000.0;
            let vol = return_vol_bp as f64 / 10_000.0;
            lower.isa_return_mean = base_mean;
            lower.taxable_return_mean = base_mean;
            lower.pension_return_mean = base_mean;
            lower.isa_return_vol = vol;
            lower.taxable_return_vol = vol;
            lower.pension_return_vol = vol;

            let mut higher = lower.clone();
            let delta = mean_delta_bp as f64 / 10_000.0;
            higher.isa_return_mean += delta;
            higher.taxable_return_mean += delta;
            higher.pension_return_mean += delta;

            let lower_model = run_model(&lower);
            let higher_model = run_model(&higher);

            for (lo, hi) in lower_model.age_results.iter().zip(higher_model.age_results.iter()) {
                prop_assert!(hi.success_rate + 1e-9 >= lo.success_rate);
            }
        }
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(16))]

        #[test]
        fn prop_higher_target_income_does_not_improve_success_rate(
            seed in any::<u64>(),
            current_age in 30u32..51,
            retirement_span in 0u32..7,
            horizon_extra in 1u32..12,
            simulations in 24u32..72,
            isa_start in 0u32..300_000,
            taxable_start in 0u32..200_000,
            pension_start in 0u32..400_000,
            return_mean_bp in -200i32..1001,
            return_vol_bp in 500u32..2_001,
            base_income in 8_000u32..70_000,
            income_multiplier_pct in 105u32..181
        ) {
            let mut lower_income = sample_inputs();
            configure_metamorphic_inputs(&mut lower_income);
            lower_income.seed = seed;
            lower_income.current_age = current_age;
            lower_income.max_retirement_age = current_age + retirement_span;
            lower_income.horizon_age = lower_income.max_retirement_age + horizon_extra + 1;
            lower_income.pension_access_age =
                (current_age + 20).min(lower_income.horizon_age - 1);
            lower_income.simulations = simulations;

            lower_income.isa_start = isa_start as f64;
            lower_income.taxable_start = taxable_start as f64;
            lower_income.taxable_cost_basis_start = lower_income.taxable_start;
            lower_income.pension_start = pension_start as f64;
            lower_income.target_annual_income = base_income as f64;

            let mean = return_mean_bp as f64 / 10_000.0;
            let vol = return_vol_bp as f64 / 10_000.0;
            lower_income.isa_return_mean = mean;
            lower_income.taxable_return_mean = mean;
            lower_income.pension_return_mean = mean;
            lower_income.isa_return_vol = vol;
            lower_income.taxable_return_vol = vol;
            lower_income.pension_return_vol = vol;

            let mut higher_income = lower_income.clone();
            higher_income.target_annual_income =
                lower_income.target_annual_income * income_multiplier_pct as f64 / 100.0;

            let lower_model = run_model(&lower_income);
            let higher_model = run_model(&higher_income);

            for (lo, hi) in lower_model.age_results.iter().zip(higher_model.age_results.iter()) {
                prop_assert!(hi.success_rate <= lo.success_rate + 1e-9);
            }
        }
    }

    #[test]
    fn zero_volatility_fixed_seed_reruns_are_identical() {
        let mut inputs = sample_inputs();
        inputs.seed = 123;
        inputs.simulations = 40;
        inputs.current_age = 30;
        inputs.max_retirement_age = 36;
        inputs.horizon_age = 45;
        inputs.pension_access_age = 57;

        inputs.isa_return_vol = 0.0;
        inputs.taxable_return_vol = 0.0;
        inputs.pension_return_vol = 0.0;
        inputs.inflation_vol = 0.0;

        let model_a = run_model(&inputs);
        let model_b = run_model(&inputs);
        assert_models_approx_equal(&model_a, &model_b);

        let rows_a = run_yearly_cashflow_trace(&inputs, 36, 36, 36);
        let rows_b = run_yearly_cashflow_trace(&inputs, 36, 36, 36);
        assert_eq!(rows_a.len(), rows_b.len());
        for (a, b) in rows_a.iter().zip(rows_b.iter()) {
            assert_eq!(a.age, b.age);
            for (label, left, right) in [
                (
                    "median_contribution_isa",
                    a.median_contribution_isa,
                    b.median_contribution_isa,
                ),
                (
                    "median_contribution_taxable",
                    a.median_contribution_taxable,
                    b.median_contribution_taxable,
                ),
                (
                    "median_contribution_pension",
                    a.median_contribution_pension,
                    b.median_contribution_pension,
                ),
                (
                    "median_contribution_total",
                    a.median_contribution_total,
                    b.median_contribution_total,
                ),
                (
                    "median_withdrawal_portfolio",
                    a.median_withdrawal_portfolio,
                    b.median_withdrawal_portfolio,
                ),
                (
                    "median_withdrawal_non_pension_income",
                    a.median_withdrawal_non_pension_income,
                    b.median_withdrawal_non_pension_income,
                ),
                (
                    "median_spending_total",
                    a.median_spending_total,
                    b.median_spending_total,
                ),
                ("median_tax_cgt", a.median_tax_cgt, b.median_tax_cgt),
                (
                    "median_tax_income",
                    a.median_tax_income,
                    b.median_tax_income,
                ),
                ("median_tax_total", a.median_tax_total, b.median_tax_total),
                ("median_end_isa", a.median_end_isa, b.median_end_isa),
                (
                    "median_end_taxable",
                    a.median_end_taxable,
                    b.median_end_taxable,
                ),
                (
                    "median_end_pension",
                    a.median_end_pension,
                    b.median_end_pension,
                ),
                ("median_end_cash", a.median_end_cash, b.median_end_cash),
                (
                    "median_end_bond_ladder",
                    a.median_end_bond_ladder,
                    b.median_end_bond_ladder,
                ),
                ("median_end_total", a.median_end_total, b.median_end_total),
            ] {
                assert!(
                    (left - right).abs() <= 1e-9,
                    "field {label}: expected {left}, got {right}"
                );
            }
        }
    }

    #[test]
    fn oracle_compound_pre_retirement_path_matches_hand_calculation() {
        let mut inputs = deterministic_oracle_inputs();
        inputs.current_age = 30;
        inputs.max_retirement_age = 33;
        inputs.horizon_age = 34;
        inputs.pension_access_age = 57;

        inputs.isa_start = 100.0;
        inputs.taxable_start = 50.0;
        inputs.taxable_cost_basis_start = 50.0;
        inputs.pension_start = 200.0;
        inputs.cash_start = 0.0;

        inputs.isa_annual_contribution = 10.0;
        inputs.taxable_annual_contribution = 5.0;
        inputs.pension_annual_contribution = 2.0;
        inputs.contribution_growth_rate = 0.0;

        inputs.isa_return_mean = 0.10;
        inputs.taxable_return_mean = 0.10;
        inputs.pension_return_mean = 0.10;
        inputs.target_annual_income = 0.0;

        // Hand calculation:
        // ISA: ((100*1.1+10)*1.1+10)*1.1+10 = 166.2
        // Taxable: ((50*1.1+5)*1.1+5)*1.1+5 = 83.1
        // Pension: ((200*1.1+2)*1.1+2)*1.1+2 = 272.82
        // Retirement total = 522.12; then one retirement year of 10% growth -> 574.332
        let mut rng = Rng::new(derive_seed(inputs.seed, 33, 0));
        let scenario = simulate_scenario(&inputs, 33, 33, &mut rng, None);

        assert!(scenario.success);
        assert_approx(scenario.reported_retirement_isa, 166.2);
        assert_approx(scenario.reported_retirement_taxable, 83.1);
        assert_approx(scenario.reported_retirement_pension, 272.82);
        assert_approx(scenario.reported_retirement_total, 522.12);

        assert_approx(scenario.reported_terminal_isa, 182.82);
        assert_approx(scenario.reported_terminal_taxable, 91.41);
        assert_approx(scenario.reported_terminal_pension, 300.102);
        assert_approx(scenario.reported_terminal_total, 574.332);
    }

    #[test]
    fn oracle_isa_cap_overflow_and_contribution_growth_match_hand_calculation() {
        let mut inputs = deterministic_oracle_inputs();
        inputs.current_age = 30;
        inputs.max_retirement_age = 33;
        inputs.horizon_age = 34;
        inputs.pension_access_age = 57;

        inputs.isa_start = 0.0;
        inputs.taxable_start = 0.0;
        inputs.taxable_cost_basis_start = 0.0;
        inputs.pension_start = 0.0;
        inputs.cash_start = 0.0;

        inputs.isa_annual_contribution = 30_000.0;
        inputs.isa_annual_contribution_limit = 20_000.0;
        inputs.taxable_annual_contribution = 5_000.0;
        inputs.pension_annual_contribution = 0.0;
        inputs.contribution_growth_rate = 0.10;

        // Hand calculation:
        // Year 0: ISA 20k, taxable 5k + (30k-20k) = 15k
        // Year 1: ISA 20k, taxable 5.5k + (33k-20k) = 18.5k
        // Year 2: ISA 20k, taxable 6.05k + (36.3k-20k) = 22.35k
        // Retirement balances: ISA 60k, taxable 55.85k
        let mut rng = Rng::new(derive_seed(inputs.seed, 33, 0));
        let scenario = simulate_scenario(&inputs, 33, 33, &mut rng, None);

        assert!(scenario.success);
        assert_approx(scenario.reported_retirement_isa, 60_000.0);
        assert_approx(scenario.reported_retirement_taxable, 55_850.0);
        assert_approx(scenario.reported_retirement_total, 115_850.0);
        assert_approx(scenario.reported_terminal_total, 115_850.0);

        let rows = run_yearly_cashflow_trace(&inputs, 33, 33, 33);
        assert_eq!(rows.len(), 4);

        assert_approx(rows[0].median_contribution_isa, 20_000.0);
        assert_approx(rows[0].median_contribution_taxable, 15_000.0);
        assert_approx(rows[0].median_contribution_total, 35_000.0);
        assert_approx(rows[0].median_end_isa, 20_000.0);
        assert_approx(rows[0].median_end_taxable, 15_000.0);

        assert_approx(rows[1].median_contribution_isa, 20_000.0);
        assert_approx(rows[1].median_contribution_taxable, 18_500.0);
        assert_approx(rows[1].median_contribution_total, 38_500.0);
        assert_approx(rows[1].median_end_isa, 40_000.0);
        assert_approx(rows[1].median_end_taxable, 33_500.0);

        assert_approx(rows[2].median_contribution_isa, 20_000.0);
        assert_approx(rows[2].median_contribution_taxable, 22_350.0);
        assert_approx(rows[2].median_contribution_total, 42_350.0);
        assert_approx(rows[2].median_end_isa, 60_000.0);
        assert_approx(rows[2].median_end_taxable, 55_850.0);

        assert_approx(rows[3].median_contribution_total, 0.0);
        assert_approx(rows[3].median_end_total, 115_850.0);
    }

    #[test]
    fn oracle_taxable_first_withdrawal_applies_cgt_and_preserves_pension() {
        let mut inputs = deterministic_oracle_inputs();
        inputs.current_age = 30;
        inputs.max_retirement_age = 30;
        inputs.horizon_age = 31;
        inputs.pension_access_age = 30;

        inputs.isa_start = 100.0;
        inputs.taxable_start = 100.0;
        inputs.taxable_cost_basis_start = 50.0;
        inputs.pension_start = 100.0;
        inputs.cash_start = 0.0;
        inputs.target_annual_income = 180.0;

        inputs.capital_gains_tax_rate = 0.20;
        inputs.capital_gains_allowance = 0.0;
        inputs.post_access_withdrawal_order = WithdrawalOrder::TaxableFirst;

        // Hand calculation:
        // Taxable full sale: gross 100, gain 50, CGT 10, net 90.
        // Remaining 90 from ISA. Pension untouched.
        // Terminal: ISA 10, taxable 0, pension 100, total 110.
        let mut rng = Rng::new(derive_seed(inputs.seed, 30, 0));
        let scenario = simulate_scenario(&inputs, 30, 30, &mut rng, None);

        assert!(scenario.success);
        assert_approx(scenario.reported_retirement_total, 300.0);
        assert_approx(scenario.reported_terminal_isa, 10.0);
        assert_approx(scenario.reported_terminal_taxable, 0.0);
        assert_approx(scenario.reported_terminal_pension, 100.0);
        assert_approx(scenario.reported_terminal_total, 110.0);
        assert_approx(scenario.min_income_ratio, 1.0);

        let rows = run_yearly_cashflow_trace(&inputs, 30, 30, 30);
        assert_eq!(rows.len(), 1);
        assert_approx(rows[0].median_withdrawal_portfolio, 180.0);
        assert_approx(rows[0].median_tax_cgt, 10.0);
        assert_approx(rows[0].median_tax_income, 0.0);
        assert_approx(rows[0].median_end_isa, 10.0);
        assert_approx(rows[0].median_end_taxable, 0.0);
        assert_approx(rows[0].median_end_pension, 100.0);
    }

    #[test]
    fn oracle_pension_withdrawal_uses_gross_up_for_flat_income_tax() {
        let mut inputs = deterministic_oracle_inputs();
        inputs.current_age = 30;
        inputs.max_retirement_age = 30;
        inputs.horizon_age = 31;
        inputs.pension_access_age = 30;

        inputs.isa_start = 0.0;
        inputs.taxable_start = 0.0;
        inputs.taxable_cost_basis_start = 0.0;
        inputs.pension_start = 100.0;
        inputs.cash_start = 0.0;
        inputs.target_annual_income = 80.0;

        inputs.pension_tax_mode = PensionTaxMode::FlatRate;
        inputs.pension_flat_tax_rate = 0.20;
        inputs.post_access_withdrawal_order = WithdrawalOrder::PensionFirst;

        let mut rng = Rng::new(derive_seed(inputs.seed, 30, 0));
        let scenario = simulate_scenario(&inputs, 30, 30, &mut rng, None);
        assert!(scenario.success);
        assert_approx_tol(scenario.reported_terminal_pension, 0.0, 1e-6);
        assert_approx_tol(scenario.reported_terminal_total, 0.0, 1e-6);
        assert_approx(scenario.min_income_ratio, 1.0);

        let rows = run_yearly_cashflow_trace(&inputs, 30, 30, 30);
        assert_eq!(rows.len(), 1);
        assert_approx_tol(rows[0].median_withdrawal_portfolio, 80.0, 1e-5);
        assert_approx_tol(rows[0].median_tax_income, 20.0, 1e-5);
        assert_approx_tol(rows[0].median_end_pension, 0.0, 1e-6);
        assert_approx_tol(rows[0].median_end_total, 0.0, 1e-6);
    }

    #[test]
    fn oracle_bond_ladder_draws_evenly_before_other_pots() {
        let mut inputs = deterministic_oracle_inputs();
        inputs.current_age = 30;
        inputs.max_retirement_age = 30;
        inputs.horizon_age = 33;
        inputs.pension_access_age = 57;

        inputs.isa_start = 0.0;
        inputs.taxable_start = 0.0;
        inputs.taxable_cost_basis_start = 0.0;
        inputs.pension_start = 0.0;
        inputs.cash_start = 0.0;
        inputs.bond_ladder_start = 90.0;
        inputs.bond_ladder_yield = 0.0;
        inputs.bond_ladder_years = 3;
        inputs.target_annual_income = 30.0;

        let mut rng = Rng::new(derive_seed(inputs.seed, 30, 0));
        let scenario = simulate_scenario(&inputs, 30, 30, &mut rng, None);
        assert!(scenario.success);
        assert_approx(scenario.reported_terminal_bond_ladder, 0.0);
        assert_approx(scenario.reported_terminal_total, 0.0);

        let rows = run_yearly_cashflow_trace(&inputs, 30, 30, 30);
        assert_eq!(rows.len(), 3);
        assert_approx(rows[0].median_end_bond_ladder, 60.0);
        assert_approx(rows[1].median_end_bond_ladder, 30.0);
        assert_approx(rows[2].median_end_bond_ladder, 0.0);
    }

    #[test]
    fn pre_retirement_contributions_apply_isa_cap_and_overflow() {
        let inputs = sample_inputs();
        let mut portfolio = Portfolio {
            isa: 0.0,
            taxable: 0.0,
            taxable_basis: 0.0,
            pension: 0.0,
            cash_buffer: 0.0,
            bond_ladder: 0.0,
        };

        apply_pre_retirement_contributions(&inputs, &mut portfolio, 0);
        assert_approx(portfolio.isa, 20_000.0);
        assert_approx(portfolio.taxable, 15_000.0);
        assert_approx(portfolio.taxable_basis, 15_000.0);
    }

    #[test]
    fn pre_retirement_contributions_clamp_negative_values() {
        let mut inputs = sample_inputs();
        inputs.isa_annual_contribution = -100.0;
        inputs.taxable_annual_contribution = -50.0;
        inputs.pension_annual_contribution = -1.0;

        let mut portfolio = Portfolio {
            isa: 1_000.0,
            taxable: 2_000.0,
            taxable_basis: 2_000.0,
            pension: 3_000.0,
            cash_buffer: 0.0,
            bond_ladder: 0.0,
        };

        apply_pre_retirement_contributions(&inputs, &mut portfolio, 0);
        assert_approx(portfolio.isa, 1_000.0);
        assert_approx(portfolio.taxable, 2_000.0);
        assert_approx(portfolio.pension, 3_000.0);
    }

    #[test]
    fn pre_retirement_contributions_grow_annually_with_rate() {
        let mut inputs = sample_inputs();
        inputs.contribution_growth_rate = 0.10;
        let mut portfolio = Portfolio {
            isa: 0.0,
            taxable: 0.0,
            taxable_basis: 0.0,
            pension: 0.0,
            cash_buffer: 0.0,
            bond_ladder: 0.0,
        };

        apply_pre_retirement_contributions(&inputs, &mut portfolio, 1);
        assert_approx(portfolio.isa, 20_000.0);
        assert_approx(portfolio.taxable, 18_500.0);
        assert_approx(portfolio.taxable_basis, 18_500.0);
    }

    #[test]
    fn uk_tax_bands_apply_progressive_rates() {
        let mut inputs = sample_inputs();
        inputs.pension_tax_mode = PensionTaxMode::UkBands;
        let tax = income_tax_for_total_income(60_000.0, &inputs, 1.0);
        assert!((tax - 11_432.0).abs() < 1e-3);
    }

    #[test]
    fn state_pension_can_cover_spending_without_assets() {
        let mut inputs = sample_inputs();
        inputs.current_age = 30;
        inputs.horizon_age = 31;
        inputs.max_retirement_age = 30;
        inputs.isa_start = 0.0;
        inputs.taxable_start = 0.0;
        inputs.taxable_cost_basis_start = 0.0;
        inputs.pension_start = 0.0;
        inputs.isa_annual_contribution = 0.0;
        inputs.taxable_annual_contribution = 0.0;
        inputs.pension_annual_contribution = 0.0;
        inputs.target_annual_income = 10_000.0;
        inputs.isa_return_mean = 0.0;
        inputs.taxable_return_mean = 0.0;
        inputs.pension_return_mean = 0.0;
        inputs.isa_return_vol = 0.0;
        inputs.taxable_return_vol = 0.0;
        inputs.pension_return_vol = 0.0;
        inputs.inflation_mean = 0.0;
        inputs.inflation_vol = 0.0;
        inputs.pension_tax_mode = PensionTaxMode::UkBands;
        inputs.state_pension_start_age = 30;
        inputs.state_pension_annual_income = 10_000.0;

        let mut rng = Rng::new(1);
        let s = simulate_scenario(&inputs, 30, 30, &mut rng, None);
        assert!(s.success);
    }

    #[test]
    fn required_spending_drops_after_mortgage_end_age() {
        let mut inputs = sample_inputs();
        inputs.target_annual_income = 30_000.0;
        inputs.mortgage_annual_payment = 12_000.0;
        inputs.mortgage_end_age = Some(40);

        assert_approx(required_real_spending(&inputs, 39), 42_000.0);
        assert_approx(required_real_spending(&inputs, 40), 30_000.0);
        assert_approx(required_real_spending(&inputs, 41), 30_000.0);
    }

    #[test]
    fn mortgage_end_age_reduces_required_spending_in_retirement() {
        let mut inputs = sample_inputs();
        inputs.current_age = 30;
        inputs.max_retirement_age = 30;
        inputs.horizon_age = 32;
        inputs.pension_access_age = 30;
        inputs.isa_start = 25_000.0;
        inputs.taxable_start = 0.0;
        inputs.taxable_cost_basis_start = 0.0;
        inputs.pension_start = 0.0;
        inputs.cash_start = 0.0;
        inputs.isa_annual_contribution = 0.0;
        inputs.taxable_annual_contribution = 0.0;
        inputs.pension_annual_contribution = 0.0;
        inputs.contribution_growth_rate = 0.0;
        inputs.target_annual_income = 10_000.0;
        inputs.mortgage_annual_payment = 5_000.0;
        inputs.mortgage_end_age = Some(31);
        inputs.isa_return_mean = 0.0;
        inputs.taxable_return_mean = 0.0;
        inputs.pension_return_mean = 0.0;
        inputs.isa_return_vol = 0.0;
        inputs.taxable_return_vol = 0.0;
        inputs.pension_return_vol = 0.0;
        inputs.inflation_mean = 0.0;
        inputs.inflation_vol = 0.0;
        inputs.cash_growth_rate = 0.0;
        inputs.taxable_return_tax_drag = 0.0;
        inputs.capital_gains_tax_rate = 0.0;
        inputs.capital_gains_allowance = 0.0;
        inputs.pension_tax_mode = PensionTaxMode::FlatRate;
        inputs.pension_flat_tax_rate = 0.0;
        inputs.state_pension_start_age = 200;
        inputs.state_pension_annual_income = 0.0;
        inputs.withdrawal_strategy = WithdrawalStrategy::Guardrails;
        inputs.bad_year_threshold = -1.0;
        inputs.good_year_threshold = 1.0;
        inputs.bad_year_cut = 0.0;
        inputs.good_year_raise = 0.0;
        inputs.min_income_floor = 1.0;
        inputs.max_income_ceiling = 1.0;
        inputs.good_year_extra_buffer_withdrawal = 0.0;
        inputs.post_access_withdrawal_order = WithdrawalOrder::IsaFirst;

        let mut rng = Rng::new(123);
        let ends_early = simulate_scenario(&inputs, 30, 30, &mut rng, None);
        assert!(ends_early.success);
        assert_approx(ends_early.reported_terminal_total, 0.0);
        assert_approx(ends_early.min_income_ratio, 1.0);

        inputs.mortgage_end_age = Some(35);
        let mut rng2 = Rng::new(123);
        let ends_late = simulate_scenario(&inputs, 30, 30, &mut rng2, None);
        assert!(!ends_late.success);
        assert!(ends_late.min_income_ratio < 1.0);
    }

    #[test]
    fn net_from_taxable_gross_with_no_gain_has_no_tax() {
        let net = net_from_taxable_gross(100.0, 200.0, 200.0, 3_000.0, 0.20);
        assert_approx(net, 100.0);
    }

    #[test]
    fn net_from_taxable_gross_applies_allowance_then_tax() {
        let net = net_from_taxable_gross(50.0, 100.0, 40.0, 10.0, 0.20);
        assert_approx(net, 46.0);
    }

    #[test]
    fn execute_taxable_sale_updates_value_basis_and_allowance() {
        let mut taxable = 100.0;
        let mut basis = 40.0;
        let mut cgt = CgtState {
            allowance_remaining: 10.0,
            tax_paid: 0.0,
        };

        let net = execute_taxable_sale(50.0, &mut taxable, &mut basis, &mut cgt, 0.20);
        assert_approx(net, 46.0);
        assert_approx(taxable, 50.0);
        assert_approx(basis, 20.0);
        assert_approx(cgt.allowance_remaining, 0.0);
    }

    #[test]
    fn withdraw_from_taxable_for_net_targets_net_amount() {
        let mut taxable = 100.0;
        let mut basis = 40.0;
        let mut cgt = CgtState {
            allowance_remaining: 10.0,
            tax_paid: 0.0,
        };

        let withdrawn =
            withdraw_from_taxable_for_net(46.0, &mut taxable, &mut basis, &mut cgt, 0.20);
        assert!((withdrawn - 46.0).abs() < 1e-3);
        assert!(taxable < 100.0);
        assert!(basis < 40.0);
    }

    #[test]
    fn withdraw_from_portfolio_before_pension_access_ignores_pension() {
        let mut inputs = sample_inputs();
        inputs.pension_access_age = 60;
        let mut portfolio = Portfolio {
            isa: 100.0,
            taxable: 100.0,
            taxable_basis: 100.0,
            pension: 100.0,
            cash_buffer: 0.0,
            bond_ladder: 0.0,
        };
        let mut cgt = CgtState {
            allowance_remaining: 3_000.0,
            tax_paid: 0.0,
        };
        let mut tax_state = TaxYearState {
            non_pension_taxable_income: 0.0,
            pension_gross_withdrawn: 0.0,
            price_index: 1.0,
        };

        let withdrawn = withdraw_from_portfolio(
            &inputs,
            50,
            150.0,
            &mut portfolio,
            &mut cgt,
            &mut tax_state,
            WithdrawalOrder::PensionFirst,
        );

        assert_approx(withdrawn, 150.0);
        assert_approx(portfolio.pension, 100.0);
    }

    #[test]
    fn run_withdrawal_year_adds_extra_to_cash_in_good_years() {
        let mut inputs = sample_inputs();
        inputs.good_year_threshold = 0.0;
        inputs.good_year_extra_buffer_withdrawal = 0.10;
        inputs.post_access_withdrawal_order = WithdrawalOrder::IsaFirst;

        let mut portfolio = Portfolio {
            isa: 200.0,
            taxable: 0.0,
            taxable_basis: 0.0,
            pension: 0.0,
            cash_buffer: 0.0,
            bond_ladder: 0.0,
        };
        let mut cgt = CgtState {
            allowance_remaining: 3_000.0,
            tax_paid: 0.0,
        };
        let mut tax_state = TaxYearState {
            non_pension_taxable_income: 0.0,
            pension_gross_withdrawn: 0.0,
            price_index: 1.0,
        };

        let outcome = run_withdrawal_year(
            &inputs,
            60,
            0,
            100.0,
            0.10,
            100.0,
            &mut portfolio,
            &mut cgt,
            &mut tax_state,
            0.0,
        );
        assert_approx(outcome.realized_spending_net, 100.0);
        assert_approx(portfolio.cash_buffer, 10.0);
        assert_approx(portfolio.isa, 90.0);
    }

    #[test]
    fn plan_real_spending_guyton_klinger_cuts_after_bad_year_above_guardrail() {
        let mut inputs = sample_inputs();
        inputs.withdrawal_strategy = WithdrawalStrategy::GuytonKlinger;
        inputs.bad_year_threshold = -0.05;
        inputs.bad_year_cut = 0.10;
        inputs.gk_upper_guardrail = 1.20;

        let mut spending_state = SpendingState {
            current_real_spending: 50_000.0,
            initial_withdrawal_rate: 0.04,
        };

        let planned = plan_real_spending(&inputs, 60, -0.10, 1_000_000.0, &mut spending_state);
        assert_approx(planned, 45_000.0);
    }

    #[test]
    fn plan_real_spending_vpw_spends_more_later_with_same_balance() {
        let mut inputs = sample_inputs();
        inputs.withdrawal_strategy = WithdrawalStrategy::Vpw;
        inputs.vpw_expected_real_return = 0.03;

        let mut early_state = SpendingState {
            current_real_spending: 50_000.0,
            initial_withdrawal_rate: 0.04,
        };
        let early = plan_real_spending(&inputs, 60, 0.0, 1_000_000.0, &mut early_state);

        let mut late_state = SpendingState {
            current_real_spending: 50_000.0,
            initial_withdrawal_rate: 0.04,
        };
        let late = plan_real_spending(&inputs, 80, 0.0, 1_000_000.0, &mut late_state);

        assert!(late > early);
    }

    #[test]
    fn plan_real_spending_floor_upside_increases_after_positive_returns() {
        let mut inputs = sample_inputs();
        inputs.withdrawal_strategy = WithdrawalStrategy::FloorUpside;
        inputs.floor_upside_capture = 0.50;

        let mut spending_state = SpendingState {
            current_real_spending: 50_000.0,
            initial_withdrawal_rate: 0.04,
        };

        let planned = plan_real_spending(&inputs, 60, 0.20, 1_000_000.0, &mut spending_state);
        assert_approx(planned, 55_000.0);
    }

    #[test]
    fn run_withdrawal_year_bucket_refills_cash_toward_target_after_good_year() {
        let mut inputs = sample_inputs();
        inputs.withdrawal_strategy = WithdrawalStrategy::Bucket;
        inputs.good_year_threshold = 0.0;
        inputs.bucket_target_years = 2.0;
        inputs.good_year_extra_buffer_withdrawal = 2.0;
        inputs.post_access_withdrawal_order = WithdrawalOrder::IsaFirst;

        let mut portfolio = Portfolio {
            isa: 500.0,
            taxable: 0.0,
            taxable_basis: 0.0,
            pension: 0.0,
            cash_buffer: 0.0,
            bond_ladder: 0.0,
        };
        let mut cgt = CgtState {
            allowance_remaining: 3_000.0,
            tax_paid: 0.0,
        };
        let mut tax_state = TaxYearState {
            non_pension_taxable_income: 0.0,
            pension_gross_withdrawn: 0.0,
            price_index: 1.0,
        };

        let outcome = run_withdrawal_year(
            &inputs,
            60,
            0,
            100.0,
            0.10,
            100.0,
            &mut portfolio,
            &mut cgt,
            &mut tax_state,
            0.0,
        );

        assert_approx(outcome.realized_spending_net, 100.0);
        assert_approx(portfolio.cash_buffer, 200.0);
    }

    #[test]
    fn sample_market_zero_volatility_returns_means() {
        let mut inputs = sample_inputs();
        inputs.isa_return_vol = 0.0;
        inputs.taxable_return_vol = 0.0;
        inputs.pension_return_vol = 0.0;
        inputs.inflation_vol = 0.0;

        let mut rng = Rng::new(123);
        let s = sample_market(&inputs, &mut rng);
        assert_approx(s.isa_return, inputs.isa_return_mean);
        assert_approx(s.taxable_return, inputs.taxable_return_mean);
        assert_approx(s.pension_return, inputs.pension_return_mean);
        assert_approx(s.inflation, inputs.inflation_mean);
    }

    #[test]
    fn sample_market_clamps_extreme_values() {
        let mut inputs = sample_inputs();
        inputs.isa_return_mean = -2.0;
        inputs.taxable_return_mean = -2.0;
        inputs.pension_return_mean = 3.0;
        inputs.inflation_mean = 0.5;
        inputs.isa_return_vol = 0.0;
        inputs.taxable_return_vol = 0.0;
        inputs.pension_return_vol = 0.0;
        inputs.inflation_vol = 0.0;

        let mut rng = Rng::new(1);
        let s = sample_market(&inputs, &mut rng);
        assert_approx(s.isa_return, -0.95);
        assert_approx(s.taxable_return, -0.95);
        assert_approx(s.pension_return, 2.5);
        assert_approx(s.inflation, 0.20);
    }

    #[test]
    fn percentile_interpolates_between_points() {
        let mut values = vec![1.0, 2.0, 3.0, 4.0];
        let p25 = percentile(&mut values, 25.0);
        assert_approx(p25, 1.75);
    }

    #[test]
    fn derive_seed_changes_per_age_and_scenario() {
        let a = derive_seed(42, 30, 0);
        let b = derive_seed(42, 31, 0);
        let c = derive_seed(42, 30, 1);
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn simulate_scenario_respects_contribution_stop_age() {
        let mut inputs = sample_inputs();
        inputs.current_age = 30;
        inputs.horizon_age = 33;
        inputs.max_retirement_age = 32;
        inputs.isa_start = 0.0;
        inputs.taxable_start = 0.0;
        inputs.taxable_cost_basis_start = 0.0;
        inputs.pension_start = 0.0;
        inputs.isa_annual_contribution = 1_000.0;
        inputs.isa_annual_contribution_limit = 20_000.0;
        inputs.taxable_annual_contribution = 0.0;
        inputs.pension_annual_contribution = 0.0;
        inputs.contribution_growth_rate = 0.0;
        inputs.target_annual_income = 1e-9;
        inputs.isa_return_mean = 0.0;
        inputs.taxable_return_mean = 0.0;
        inputs.pension_return_mean = 0.0;
        inputs.isa_return_vol = 0.0;
        inputs.taxable_return_vol = 0.0;
        inputs.pension_return_vol = 0.0;
        inputs.inflation_mean = 0.0;
        inputs.inflation_vol = 0.0;
        inputs.taxable_return_tax_drag = 0.0;
        inputs.good_year_extra_buffer_withdrawal = 0.0;

        let mut rng_a = Rng::new(7);
        let coast_from_31 = simulate_scenario(&inputs, 32, 31, &mut rng_a, None);
        assert!(coast_from_31.success);
        assert_approx(coast_from_31.reported_retirement_total, 1_000.0);

        let mut rng_b = Rng::new(7);
        let coast_from_32 = simulate_scenario(&inputs, 32, 32, &mut rng_b, None);
        assert!(coast_from_32.success);
        assert_approx(coast_from_32.reported_retirement_total, 2_000.0);
    }

    #[test]
    fn yearly_cashflow_trace_includes_contributions_spending_taxes_and_balances() {
        let mut inputs = sample_inputs();
        inputs.current_age = 30;
        inputs.max_retirement_age = 31;
        inputs.horizon_age = 34;
        inputs.simulations = 5;
        inputs.seed = 99;
        inputs.isa_start = 50_000.0;
        inputs.taxable_start = 0.0;
        inputs.taxable_cost_basis_start = 0.0;
        inputs.pension_start = 0.0;
        inputs.cash_start = 0.0;
        inputs.isa_annual_contribution = 12_000.0;
        inputs.isa_annual_contribution_limit = 10_000.0;
        inputs.taxable_annual_contribution = 2_000.0;
        inputs.pension_annual_contribution = 1_000.0;
        inputs.contribution_growth_rate = 0.0;
        inputs.target_annual_income = 10_000.0;
        inputs.isa_return_mean = 0.0;
        inputs.taxable_return_mean = 0.0;
        inputs.pension_return_mean = 0.0;
        inputs.isa_return_vol = 0.0;
        inputs.taxable_return_vol = 0.0;
        inputs.pension_return_vol = 0.0;
        inputs.inflation_mean = 0.0;
        inputs.inflation_vol = 0.0;
        inputs.taxable_return_tax_drag = 0.0;
        inputs.capital_gains_tax_rate = 0.0;
        inputs.capital_gains_allowance = 0.0;
        inputs.pension_tax_mode = PensionTaxMode::FlatRate;
        inputs.pension_flat_tax_rate = 0.0;
        inputs.state_pension_start_age = 200;
        inputs.state_pension_annual_income = 0.0;
        inputs.good_year_extra_buffer_withdrawal = 0.0;
        inputs.cash_growth_rate = 0.0;
        inputs.post_access_withdrawal_order = WithdrawalOrder::IsaFirst;

        let rows = run_yearly_cashflow_trace(&inputs, 31, 31, 31);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].age, 30);
        assert_eq!(rows[1].age, 31);
        assert_approx(rows[0].median_contribution_isa, 10_000.0);
        assert_approx(rows[0].median_contribution_taxable, 4_000.0);
        assert_approx(rows[0].median_contribution_pension, 1_000.0);
        assert_approx(rows[0].median_contribution_total, 15_000.0);
        assert_approx(rows[1].median_contribution_total, 0.0);
        assert_approx(rows[1].median_spending_total, 10_000.0);
        assert_approx(rows[1].median_tax_total, 0.0);
        assert!(rows[1].median_end_total >= 0.0);
    }

    #[test]
    fn run_model_populates_per_pot_stats() {
        let mut inputs = sample_inputs();
        inputs.current_age = 30;
        inputs.max_retirement_age = 30;
        inputs.horizon_age = 31;
        inputs.simulations = 5;
        inputs.isa_return_vol = 0.0;
        inputs.taxable_return_vol = 0.0;
        inputs.pension_return_vol = 0.0;
        inputs.inflation_vol = 0.0;
        inputs.target_annual_income = 0.01;

        let model = run_model(&inputs);
        let age = &model.age_results[0];
        assert!(age.median_retirement_isa >= 0.0);
        assert!(age.median_terminal_pot >= age.p10_terminal_pot);
    }
}
