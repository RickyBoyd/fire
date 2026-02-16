mod engine;
mod types;

pub use engine::{run_coast_model, run_model, run_yearly_cashflow_trace};
pub use types::{
    AgeResult, CashflowYearResult, Inputs, ModelResult, PensionTaxMode, WithdrawalOrder,
    WithdrawalStrategy,
};
