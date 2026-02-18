use super::{Inputs, run_retirement_age_evaluation};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GoalType {
    RequiredContribution,
    MaxIncome,
}

#[derive(Debug, Clone, Copy)]
pub struct GoalSolveConfig {
    pub goal_type: GoalType,
    pub target_retirement_age: u32,
    pub target_success_threshold: f64,
    pub search_min: f64,
    pub search_max: f64,
    pub tolerance: f64,
    pub max_iterations: u32,
    pub simulations_per_iteration: u32,
    pub final_simulations: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct GoalSolveIteration {
    pub iteration: u32,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub candidate_value: f64,
    pub success_rate: f64,
    pub success_ci_half_width: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct ContributionAllocation {
    pub isa: f64,
    pub taxable: f64,
    pub pension: f64,
}

#[derive(Debug, Clone)]
pub struct GoalSolveResult {
    pub goal_type: GoalType,
    pub target_retirement_age: u32,
    pub target_success_threshold: f64,
    pub search_min: f64,
    pub search_max: f64,
    pub tolerance: f64,
    pub max_iterations: u32,
    pub simulations_per_iteration: u32,
    pub final_simulations: u32,
    pub solved_value: Option<f64>,
    pub solved_contributions: Option<ContributionAllocation>,
    pub achieved_success_rate: Option<f64>,
    pub achieved_success_ci_half_width: Option<f64>,
    pub iterations: Vec<GoalSolveIteration>,
    pub converged: bool,
    pub feasible: bool,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
struct ContributionMix {
    isa: f64,
    taxable: f64,
    pension: f64,
    total: f64,
}

impl ContributionMix {
    fn from_inputs(inputs: &Inputs) -> Self {
        let isa = inputs.isa_annual_contribution.max(0.0);
        let taxable = inputs.taxable_annual_contribution.max(0.0);
        let pension = inputs.pension_annual_contribution.max(0.0);
        let total = isa + taxable + pension;
        Self {
            isa,
            taxable,
            pension,
            total,
        }
    }

    fn allocation_for_total(self, total: f64) -> ContributionAllocation {
        let total = total.max(0.0);
        if self.total <= 1e-12 {
            return ContributionAllocation {
                isa: total,
                taxable: 0.0,
                pension: 0.0,
            };
        }

        let scale = total / self.total;
        ContributionAllocation {
            isa: self.isa * scale,
            taxable: self.taxable * scale,
            pension: self.pension * scale,
        }
    }
}

pub fn solve_goal(inputs: &Inputs, config: GoalSolveConfig) -> Result<GoalSolveResult, String> {
    validate_config(inputs, config)?;

    let mix = ContributionMix::from_inputs(inputs);

    let mut iterations = Vec::with_capacity(config.max_iterations as usize);
    let low_eval = evaluate_candidate(inputs, config, config.search_min, mix);
    let high_eval = evaluate_candidate(inputs, config, config.search_max, mix);

    let mut solved_value = None;
    let mut converged = false;
    let feasible;
    let message;

    match config.goal_type {
        GoalType::RequiredContribution => {
            if low_eval.success_rate + 1e-12 >= config.target_success_threshold {
                solved_value = Some(config.search_min);
                converged = true;
                feasible = true;
                message = "Already meets target at lower contribution bound.".to_string();
            } else if high_eval.success_rate + 1e-12 < config.target_success_threshold {
                feasible = false;
                message = "No feasible contribution found within the search bounds.".to_string();
            } else {
                let mut lo = config.search_min;
                let mut hi = config.search_max;
                let mut it = 0;
                while it < config.max_iterations {
                    it += 1;
                    let mid = (lo + hi) * 0.5;
                    let eval = evaluate_candidate(inputs, config, mid, mix);
                    iterations.push(GoalSolveIteration {
                        iteration: it,
                        lower_bound: lo,
                        upper_bound: hi,
                        candidate_value: mid,
                        success_rate: eval.success_rate,
                        success_ci_half_width: eval.success_ci_half_width,
                    });

                    if eval.success_rate + 1e-12 >= config.target_success_threshold {
                        hi = mid;
                    } else {
                        lo = mid;
                    }

                    if (hi - lo).abs() <= config.tolerance {
                        converged = true;
                        solved_value = Some(hi);
                        break;
                    }
                }
                if solved_value.is_none() {
                    solved_value = Some(hi);
                }
                feasible = true;
                message = if converged {
                    "Solved required contribution.".to_string()
                } else {
                    "Reached max iterations before tolerance was met; returning best estimate."
                        .to_string()
                };
            }
        }
        GoalType::MaxIncome => {
            if low_eval.success_rate + 1e-12 < config.target_success_threshold {
                feasible = false;
                message = "No feasible income found within the search bounds.".to_string();
            } else if high_eval.success_rate + 1e-12 >= config.target_success_threshold {
                solved_value = Some(config.search_max);
                converged = true;
                feasible = true;
                message =
                    "Upper income bound is still feasible; increase search max for higher target."
                        .to_string();
            } else {
                let mut lo = config.search_min;
                let mut hi = config.search_max;
                let mut it = 0;
                while it < config.max_iterations {
                    it += 1;
                    let mid = (lo + hi) * 0.5;
                    let eval = evaluate_candidate(inputs, config, mid, mix);
                    iterations.push(GoalSolveIteration {
                        iteration: it,
                        lower_bound: lo,
                        upper_bound: hi,
                        candidate_value: mid,
                        success_rate: eval.success_rate,
                        success_ci_half_width: eval.success_ci_half_width,
                    });

                    if eval.success_rate + 1e-12 >= config.target_success_threshold {
                        lo = mid;
                    } else {
                        hi = mid;
                    }

                    if (hi - lo).abs() <= config.tolerance {
                        converged = true;
                        solved_value = Some(lo);
                        break;
                    }
                }
                if solved_value.is_none() {
                    solved_value = Some(lo);
                }
                feasible = true;
                message = if converged {
                    "Solved maximum sustainable income.".to_string()
                } else {
                    "Reached max iterations before tolerance was met; returning best estimate."
                        .to_string()
                };
            }
        }
    }

    let mut achieved_success_rate = None;
    let mut achieved_success_ci_half_width = None;
    let mut solved_contributions = None;
    if let Some(value) = solved_value {
        let final_eval_with_samples = evaluate_candidate(
            inputs,
            GoalSolveConfig {
                simulations_per_iteration: config.final_simulations,
                ..config
            },
            value,
            mix,
        );
        achieved_success_rate = Some(final_eval_with_samples.success_rate);
        achieved_success_ci_half_width = Some(final_eval_with_samples.success_ci_half_width);
        if config.goal_type == GoalType::RequiredContribution {
            solved_contributions = Some(mix.allocation_for_total(value));
        }
    }

    Ok(GoalSolveResult {
        goal_type: config.goal_type,
        target_retirement_age: config.target_retirement_age,
        target_success_threshold: config.target_success_threshold,
        search_min: config.search_min,
        search_max: config.search_max,
        tolerance: config.tolerance,
        max_iterations: config.max_iterations,
        simulations_per_iteration: config.simulations_per_iteration,
        final_simulations: config.final_simulations,
        solved_value,
        solved_contributions,
        achieved_success_rate,
        achieved_success_ci_half_width,
        iterations,
        converged,
        feasible,
        message,
    })
}

#[derive(Debug, Clone, Copy)]
struct CandidateEval {
    success_rate: f64,
    success_ci_half_width: f64,
}

fn evaluate_candidate(
    base_inputs: &Inputs,
    config: GoalSolveConfig,
    candidate_value: f64,
    mix: ContributionMix,
) -> CandidateEval {
    let mut inputs = base_inputs.clone();
    inputs.simulations = config.simulations_per_iteration.max(1);

    match config.goal_type {
        GoalType::RequiredContribution => {
            let allocation = mix.allocation_for_total(candidate_value);
            inputs.isa_annual_contribution = allocation.isa;
            inputs.taxable_annual_contribution = allocation.taxable;
            inputs.pension_annual_contribution = allocation.pension;
        }
        GoalType::MaxIncome => {
            inputs.target_annual_income = candidate_value.max(0.0);
        }
    }

    let age = run_retirement_age_evaluation(&inputs, config.target_retirement_age);
    CandidateEval {
        success_rate: age.success_rate,
        success_ci_half_width: binomial_ci_half_width(age.success_rate, inputs.simulations),
    }
}

fn binomial_ci_half_width(p: f64, n: u32) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let p = p.clamp(0.0, 1.0);
    1.96 * (p * (1.0 - p) / n as f64).sqrt()
}

fn validate_config(inputs: &Inputs, config: GoalSolveConfig) -> Result<(), String> {
    if config.target_retirement_age < inputs.current_age {
        return Err("target_retirement_age must be >= current_age".to_string());
    }
    if config.target_retirement_age >= inputs.horizon_age {
        return Err("target_retirement_age must be < horizon_age".to_string());
    }
    if !(0.0..=1.0).contains(&config.target_success_threshold) {
        return Err("target_success_threshold must be between 0 and 1".to_string());
    }
    if !config.search_min.is_finite() || !config.search_max.is_finite() {
        return Err("search bounds must be finite".to_string());
    }
    if config.search_max <= config.search_min {
        return Err("search_max must be greater than search_min".to_string());
    }
    if !config.tolerance.is_finite() || config.tolerance <= 0.0 {
        return Err("tolerance must be > 0".to_string());
    }
    if config.max_iterations == 0 {
        return Err("max_iterations must be > 0".to_string());
    }
    if config.simulations_per_iteration == 0 {
        return Err("simulations_per_iteration must be > 0".to_string());
    }
    if config.final_simulations == 0 {
        return Err("final_simulations must be > 0".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{PensionTaxMode, WithdrawalOrder, WithdrawalStrategy};

    fn assert_close(actual: f64, expected: f64, tol: f64) {
        assert!(
            (actual - expected).abs() <= tol,
            "expected {expected}, got {actual}, tolerance {tol}"
        );
    }

    fn deterministic_inputs() -> Inputs {
        Inputs {
            current_age: 30,
            pension_access_age: 30,
            isa_start: 0.0,
            taxable_start: 0.0,
            taxable_cost_basis_start: 0.0,
            pension_start: 0.0,
            cash_start: 0.0,
            bond_ladder_start: 0.0,
            isa_annual_contribution: 1.0,
            isa_annual_contribution_limit: 20_000.0,
            taxable_annual_contribution: 0.0,
            pension_annual_contribution: 0.0,
            contribution_growth_rate: 0.0,
            isa_return_mean: 0.0,
            isa_return_vol: 0.0,
            taxable_return_mean: 0.0,
            taxable_return_vol: 0.0,
            pension_return_mean: 0.0,
            pension_return_vol: 0.0,
            return_correlation: 0.0,
            capital_gains_tax_rate: 0.0,
            capital_gains_allowance: 0.0,
            taxable_return_tax_drag: 0.0,
            pension_tax_mode: PensionTaxMode::FlatRate,
            pension_flat_tax_rate: 0.0,
            uk_personal_allowance: 12_570.0,
            uk_basic_rate_limit: 50_270.0,
            uk_higher_rate_limit: 125_140.0,
            uk_basic_rate: 0.20,
            uk_higher_rate: 0.40,
            uk_additional_rate: 0.45,
            uk_allowance_taper_start: 100_000.0,
            uk_allowance_taper_end: 125_140.0,
            state_pension_start_age: 200,
            state_pension_annual_income: 0.0,
            inflation_mean: 0.0,
            inflation_vol: 0.0,
            target_annual_income: 100.0,
            mortgage_annual_payment: 0.0,
            mortgage_end_age: None,
            max_retirement_age: 31,
            horizon_age: 32,
            simulations: 1,
            success_threshold: 1.0,
            seed: 7,
            bad_year_threshold: -1.0,
            good_year_threshold: 1.0,
            bad_year_cut: 0.0,
            good_year_raise: 0.0,
            min_income_floor: 1.0,
            max_income_ceiling: 1.0,
            withdrawal_strategy: WithdrawalStrategy::Guardrails,
            gk_lower_guardrail: 0.8,
            gk_upper_guardrail: 1.2,
            vpw_expected_real_return: 0.03,
            floor_upside_capture: 0.5,
            bucket_target_years: 2.0,
            good_year_extra_buffer_withdrawal: 0.0,
            cash_growth_rate: 0.0,
            bond_ladder_yield: 0.0,
            bond_ladder_years: 0,
            post_access_withdrawal_order: WithdrawalOrder::IsaFirst,
        }
    }

    #[test]
    fn required_contribution_solver_finds_deterministic_solution() {
        let inputs = deterministic_inputs();
        let config = GoalSolveConfig {
            goal_type: GoalType::RequiredContribution,
            target_retirement_age: 31,
            target_success_threshold: 1.0,
            search_min: 0.0,
            search_max: 200.0,
            tolerance: 0.5,
            max_iterations: 24,
            simulations_per_iteration: 1,
            final_simulations: 1,
        };

        let result = solve_goal(&inputs, config).expect("must solve");
        assert!(result.feasible);
        assert!(result.solved_value.is_some());
        assert_close(
            result.solved_value.expect("value expected"),
            100.0,
            config.tolerance + 0.5,
        );
        assert_close(
            result.achieved_success_rate.expect("rate expected"),
            1.0,
            1e-9,
        );
    }

    #[test]
    fn max_income_solver_finds_deterministic_solution() {
        let mut inputs = deterministic_inputs();
        inputs.max_retirement_age = 30;
        inputs.horizon_age = 31;
        inputs.isa_start = 500.0;
        inputs.target_annual_income = 100.0;

        let config = GoalSolveConfig {
            goal_type: GoalType::MaxIncome,
            target_retirement_age: 30,
            target_success_threshold: 1.0,
            search_min: 0.0,
            search_max: 600.0,
            tolerance: 0.5,
            max_iterations: 24,
            simulations_per_iteration: 1,
            final_simulations: 1,
        };

        let result = solve_goal(&inputs, config).expect("must solve");
        assert!(result.feasible);
        assert!(result.solved_value.is_some());
        assert_close(
            result.solved_value.expect("value expected"),
            500.0,
            config.tolerance + 0.5,
        );
    }

    #[test]
    fn required_contribution_solver_reports_infeasible_when_bounds_too_low() {
        let inputs = deterministic_inputs();
        let config = GoalSolveConfig {
            goal_type: GoalType::RequiredContribution,
            target_retirement_age: 31,
            target_success_threshold: 1.0,
            search_min: 0.0,
            search_max: 50.0,
            tolerance: 0.5,
            max_iterations: 16,
            simulations_per_iteration: 1,
            final_simulations: 1,
        };

        let result = solve_goal(&inputs, config).expect("must return result");
        assert!(!result.feasible);
        assert!(result.solved_value.is_none());
    }
}
