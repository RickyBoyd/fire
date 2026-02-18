use serde::Serialize;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WithdrawalOrder {
    ProRata,
    IsaFirst,
    TaxableFirst,
    PensionFirst,
    BondLadderFirst,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WithdrawalStrategy {
    Guardrails,
    GuytonKlinger,
    Vpw,
    FloorUpside,
    Bucket,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PensionTaxMode {
    UkBands,
    FlatRate,
}

#[derive(Debug, Clone)]
pub struct Inputs {
    pub current_age: u32,
    pub pension_access_age: u32,
    pub isa_start: f64,
    pub taxable_start: f64,
    pub taxable_cost_basis_start: f64,
    pub pension_start: f64,
    pub cash_start: f64,
    pub bond_ladder_start: f64,
    pub isa_annual_contribution: f64,
    pub isa_annual_contribution_limit: f64,
    pub taxable_annual_contribution: f64,
    pub pension_annual_contribution: f64,
    pub contribution_growth_rate: f64,
    pub isa_return_mean: f64,
    pub isa_return_vol: f64,
    pub taxable_return_mean: f64,
    pub taxable_return_vol: f64,
    pub pension_return_mean: f64,
    pub pension_return_vol: f64,
    pub return_correlation: f64,
    pub capital_gains_tax_rate: f64,
    pub capital_gains_allowance: f64,
    pub taxable_return_tax_drag: f64,
    pub pension_tax_mode: PensionTaxMode,
    pub pension_flat_tax_rate: f64,
    pub uk_personal_allowance: f64,
    pub uk_basic_rate_limit: f64,
    pub uk_higher_rate_limit: f64,
    pub uk_basic_rate: f64,
    pub uk_higher_rate: f64,
    pub uk_additional_rate: f64,
    pub uk_allowance_taper_start: f64,
    pub uk_allowance_taper_end: f64,
    pub state_pension_start_age: u32,
    pub state_pension_annual_income: f64,
    pub inflation_mean: f64,
    pub inflation_vol: f64,
    pub target_annual_income: f64,
    pub mortgage_annual_payment: f64,
    pub mortgage_end_age: Option<u32>,
    pub max_retirement_age: u32,
    pub horizon_age: u32,
    pub simulations: u32,
    pub success_threshold: f64,
    pub seed: u64,
    pub bad_year_threshold: f64,
    pub good_year_threshold: f64,
    pub bad_year_cut: f64,
    pub good_year_raise: f64,
    pub min_income_floor: f64,
    pub max_income_ceiling: f64,
    pub withdrawal_strategy: WithdrawalStrategy,
    pub gk_lower_guardrail: f64,
    pub gk_upper_guardrail: f64,
    pub vpw_expected_real_return: f64,
    pub floor_upside_capture: f64,
    pub bucket_target_years: f64,
    pub good_year_extra_buffer_withdrawal: f64,
    pub cash_growth_rate: f64,
    pub bond_ladder_yield: f64,
    pub bond_ladder_years: u32,
    pub post_access_withdrawal_order: WithdrawalOrder,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgeResult {
    pub retirement_age: u32,
    pub success_rate: f64,
    pub median_retirement_pot: f64,
    pub p10_retirement_pot: f64,
    pub median_retirement_isa: f64,
    pub p10_retirement_isa: f64,
    pub median_retirement_taxable: f64,
    pub p10_retirement_taxable: f64,
    pub median_retirement_pension: f64,
    pub p10_retirement_pension: f64,
    pub median_retirement_cash: f64,
    pub p10_retirement_cash: f64,
    pub median_retirement_bond_ladder: f64,
    pub p10_retirement_bond_ladder: f64,
    pub median_terminal_pot: f64,
    pub p10_terminal_pot: f64,
    pub median_terminal_isa: f64,
    pub p10_terminal_isa: f64,
    pub median_terminal_taxable: f64,
    pub p10_terminal_taxable: f64,
    pub median_terminal_pension: f64,
    pub p10_terminal_pension: f64,
    pub median_terminal_cash: f64,
    pub p10_terminal_cash: f64,
    pub median_terminal_bond_ladder: f64,
    pub p10_terminal_bond_ladder: f64,
    pub p10_min_income_ratio: f64,
    pub median_avg_income_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct ModelResult {
    pub age_results: Vec<AgeResult>,
    pub selected_index: Option<usize>,
    pub best_index: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashflowYearResult {
    pub age: u32,
    pub median_contribution_isa: f64,
    pub median_contribution_taxable: f64,
    pub median_contribution_pension: f64,
    pub median_contribution_total: f64,
    pub median_withdrawal_portfolio: f64,
    pub median_withdrawal_non_pension_income: f64,
    pub median_spending_total: f64,
    pub median_tax_cgt: f64,
    pub median_tax_income: f64,
    pub median_tax_total: f64,
    pub median_end_isa: f64,
    pub median_end_taxable: f64,
    pub median_end_pension: f64,
    pub median_end_cash: f64,
    pub median_end_bond_ladder: f64,
    pub median_end_total: f64,
}
