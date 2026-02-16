mod engine;
mod solver;
mod types;

pub use engine::{
    run_coast_model, run_model, run_retirement_age_evaluation, run_yearly_cashflow_trace,
};
pub use solver::{
    ContributionAllocation, GoalSolveConfig, GoalSolveIteration, GoalSolveResult, GoalType,
    solve_goal,
};
pub use types::{
    AgeResult, CashflowYearResult, Inputs, ModelResult, PensionTaxMode, WithdrawalOrder,
    WithdrawalStrategy,
};
