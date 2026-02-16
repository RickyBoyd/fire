use axum::{
    Router,
    extract::{Json, Query},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::core::{
    AgeResult, CashflowYearResult, Inputs, ModelResult, PensionTaxMode, WithdrawalOrder,
    WithdrawalStrategy, run_coast_model, run_model, run_yearly_cashflow_trace,
};

const INDEX_HTML: &str = include_str!("../../web/index.html");
const STYLES_CSS: &str = include_str!("../../web/styles.css");
const APP_JS: &str = include_str!("../../web/app.js");

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum CliWithdrawalOrder {
    ProRata,
    IsaFirst,
    TaxableFirst,
    PensionFirst,
}

impl From<CliWithdrawalOrder> for WithdrawalOrder {
    fn from(value: CliWithdrawalOrder) -> Self {
        match value {
            CliWithdrawalOrder::ProRata => WithdrawalOrder::ProRata,
            CliWithdrawalOrder::IsaFirst => WithdrawalOrder::IsaFirst,
            CliWithdrawalOrder::TaxableFirst => WithdrawalOrder::TaxableFirst,
            CliWithdrawalOrder::PensionFirst => WithdrawalOrder::PensionFirst,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum CliPensionTaxMode {
    UkBands,
    FlatRate,
}

impl From<CliPensionTaxMode> for PensionTaxMode {
    fn from(value: CliPensionTaxMode) -> Self {
        match value {
            CliPensionTaxMode::UkBands => PensionTaxMode::UkBands,
            CliPensionTaxMode::FlatRate => PensionTaxMode::FlatRate,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum CliWithdrawalStrategy {
    Guardrails,
    GuytonKlinger,
    Vpw,
    FloorUpside,
    Bucket,
}

impl From<CliWithdrawalStrategy> for WithdrawalStrategy {
    fn from(value: CliWithdrawalStrategy) -> Self {
        match value {
            CliWithdrawalStrategy::Guardrails => WithdrawalStrategy::Guardrails,
            CliWithdrawalStrategy::GuytonKlinger => WithdrawalStrategy::GuytonKlinger,
            CliWithdrawalStrategy::Vpw => WithdrawalStrategy::Vpw,
            CliWithdrawalStrategy::FloorUpside => WithdrawalStrategy::FloorUpside,
            CliWithdrawalStrategy::Bucket => WithdrawalStrategy::Bucket,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum AnalysisMode {
    RetirementSweep,
    CoastFire,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ApiWithdrawalOrder {
    #[serde(alias = "proRata", alias = "pro_rata")]
    ProRata,
    #[serde(alias = "isaFirst", alias = "isa_first")]
    IsaFirst,
    #[serde(alias = "taxableFirst", alias = "taxable_first")]
    TaxableFirst,
    #[serde(alias = "pensionFirst", alias = "pension_first")]
    PensionFirst,
}

impl From<ApiWithdrawalOrder> for CliWithdrawalOrder {
    fn from(value: ApiWithdrawalOrder) -> Self {
        match value {
            ApiWithdrawalOrder::ProRata => CliWithdrawalOrder::ProRata,
            ApiWithdrawalOrder::IsaFirst => CliWithdrawalOrder::IsaFirst,
            ApiWithdrawalOrder::TaxableFirst => CliWithdrawalOrder::TaxableFirst,
            ApiWithdrawalOrder::PensionFirst => CliWithdrawalOrder::PensionFirst,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ApiPensionTaxMode {
    #[serde(alias = "ukBands", alias = "uk_bands")]
    UkBands,
    #[serde(alias = "flat", alias = "flatRate", alias = "flat_rate")]
    FlatRate,
}

impl From<ApiPensionTaxMode> for CliPensionTaxMode {
    fn from(value: ApiPensionTaxMode) -> Self {
        match value {
            ApiPensionTaxMode::UkBands => CliPensionTaxMode::UkBands,
            ApiPensionTaxMode::FlatRate => CliPensionTaxMode::FlatRate,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ApiWithdrawalStrategy {
    #[serde(alias = "dynamic-guardrails", alias = "dynamicGuardrails")]
    Guardrails,
    #[serde(alias = "guytonKlinger", alias = "guyton_klinger")]
    GuytonKlinger,
    Vpw,
    #[serde(alias = "floorUpside", alias = "floor_upside")]
    FloorUpside,
    Bucket,
}

impl From<ApiWithdrawalStrategy> for CliWithdrawalStrategy {
    fn from(value: ApiWithdrawalStrategy) -> Self {
        match value {
            ApiWithdrawalStrategy::Guardrails => CliWithdrawalStrategy::Guardrails,
            ApiWithdrawalStrategy::GuytonKlinger => CliWithdrawalStrategy::GuytonKlinger,
            ApiWithdrawalStrategy::Vpw => CliWithdrawalStrategy::Vpw,
            ApiWithdrawalStrategy::FloorUpside => CliWithdrawalStrategy::FloorUpside,
            ApiWithdrawalStrategy::Bucket => CliWithdrawalStrategy::Bucket,
        }
    }
}

impl From<WithdrawalStrategy> for ApiWithdrawalStrategy {
    fn from(value: WithdrawalStrategy) -> Self {
        match value {
            WithdrawalStrategy::Guardrails => ApiWithdrawalStrategy::Guardrails,
            WithdrawalStrategy::GuytonKlinger => ApiWithdrawalStrategy::GuytonKlinger,
            WithdrawalStrategy::Vpw => ApiWithdrawalStrategy::Vpw,
            WithdrawalStrategy::FloorUpside => ApiWithdrawalStrategy::FloorUpside,
            WithdrawalStrategy::Bucket => ApiWithdrawalStrategy::Bucket,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ApiAnalysisMode {
    #[serde(alias = "retirementSweep", alias = "retirement")]
    RetirementSweep,
    #[serde(alias = "coastFire", alias = "coast")]
    CoastFire,
}

impl From<ApiAnalysisMode> for AnalysisMode {
    fn from(value: ApiAnalysisMode) -> Self {
        match value {
            ApiAnalysisMode::RetirementSweep => AnalysisMode::RetirementSweep,
            ApiAnalysisMode::CoastFire => AnalysisMode::CoastFire,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum ResponseMode {
    Retirement,
    Coast,
}

impl From<AnalysisMode> for ResponseMode {
    fn from(value: AnalysisMode) -> Self {
        match value {
            AnalysisMode::RetirementSweep => ResponseMode::Retirement,
            AnalysisMode::CoastFire => ResponseMode::Coast,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SimulatePayload {
    current_age: Option<u32>,
    pension_access_age: Option<u32>,
    max_age: Option<u32>,
    horizon_age: Option<u32>,
    simulations: Option<u32>,
    seed: Option<u64>,

    isa_start: Option<f64>,
    taxable_start: Option<f64>,
    taxable_basis_start: Option<f64>,
    pension_start: Option<f64>,
    cash_start: Option<f64>,

    isa_contribution: Option<f64>,
    isa_limit: Option<f64>,
    taxable_contribution: Option<f64>,
    pension_contribution: Option<f64>,
    contribution_growth: Option<f64>,

    cgt_rate: Option<f64>,
    cgt_allowance: Option<f64>,
    taxable_tax_drag: Option<f64>,

    pension_tax_mode: Option<ApiPensionTaxMode>,
    pension_income_tax_rate: Option<f64>,
    uk_personal_allowance: Option<f64>,
    uk_basic_rate_limit: Option<f64>,
    uk_higher_rate_limit: Option<f64>,
    uk_basic_rate: Option<f64>,
    uk_higher_rate: Option<f64>,
    uk_additional_rate: Option<f64>,
    uk_allowance_taper_start: Option<f64>,
    uk_allowance_taper_end: Option<f64>,
    state_pension_start_age: Option<u32>,
    state_pension_income: Option<f64>,

    isa_mean: Option<f64>,
    isa_vol: Option<f64>,
    taxable_mean: Option<f64>,
    taxable_vol: Option<f64>,
    pension_mean: Option<f64>,
    pension_vol: Option<f64>,
    correlation: Option<f64>,
    inflation_mean: Option<f64>,
    inflation_vol: Option<f64>,

    target_income: Option<f64>,
    mortgage_annual_payment: Option<f64>,
    mortgage_end_age: Option<u32>,
    success_threshold: Option<f64>,
    bad_threshold: Option<f64>,
    good_threshold: Option<f64>,
    bad_cut: Option<f64>,
    good_raise: Option<f64>,
    min_floor: Option<f64>,
    max_ceiling: Option<f64>,
    withdrawal_policy: Option<ApiWithdrawalStrategy>,
    gk_lower_guardrail: Option<f64>,
    gk_upper_guardrail: Option<f64>,
    vpw_real_return: Option<f64>,
    floor_upside_capture: Option<f64>,
    bucket_years_target: Option<f64>,
    extra_to_cash: Option<f64>,
    cash_growth: Option<f64>,
    withdrawal_order: Option<ApiWithdrawalOrder>,

    analysis_mode: Option<ApiAnalysisMode>,
    coast_retirement_age: Option<u32>,
}

#[derive(Parser, Debug)]
#[command(
    name = "fire",
    about = "Monte Carlo FIRE estimator (ISA + taxable account + pension + dynamic withdrawals)"
)]
struct Cli {
    #[arg(long)]
    current_age: u32,
    #[arg(long)]
    pension_access_age: u32,
    #[arg(long)]
    isa_start: f64,
    #[arg(long, default_value_t = 0.0)]
    taxable_start: f64,
    #[arg(
        long,
        default_value_t = 0.0,
        help = "Taxable account cost basis at start; defaults to taxable_start"
    )]
    taxable_cost_basis_start: f64,
    #[arg(long)]
    pension_start: f64,
    #[arg(long, default_value_t = 0.0)]
    cash_start: f64,
    #[arg(long)]
    isa_annual_contribution: f64,
    #[arg(
        long,
        default_value_t = 20000.0,
        help = "Annual ISA contribution allowance"
    )]
    isa_annual_contribution_limit: f64,
    #[arg(long, default_value_t = 0.0)]
    taxable_annual_contribution: f64,
    #[arg(long)]
    pension_annual_contribution: f64,
    #[arg(
        long,
        default_value_t = 0.0,
        help = "Annual growth rate for all pre-retirement contributions in percent (e.g. pay rises)"
    )]
    contribution_growth_rate: f64,
    #[arg(long, help = "Expected annual ISA return in percent, e.g. 5")]
    isa_growth_rate: f64,
    #[arg(
        long,
        default_value_t = 12.0,
        help = "ISA annual return volatility in percent"
    )]
    isa_return_volatility: f64,
    #[arg(
        long,
        help = "Expected annual taxable account return in percent, defaults to isa-growth-rate"
    )]
    taxable_growth_rate: Option<f64>,
    #[arg(
        long,
        help = "Taxable account annual return volatility in percent, defaults to isa-return-volatility"
    )]
    taxable_return_volatility: Option<f64>,
    #[arg(long, help = "Expected annual pension return in percent, e.g. 5")]
    pension_growth_rate: f64,
    #[arg(
        long,
        default_value_t = 12.0,
        help = "Pension annual return volatility in percent"
    )]
    pension_return_volatility: f64,
    #[arg(
        long,
        default_value_t = 0.8,
        help = "Correlation between ISA and pension returns"
    )]
    return_correlation: f64,
    #[arg(
        long,
        default_value_t = 20.0,
        help = "Capital gains tax rate on taxable account gains in percent"
    )]
    capital_gains_tax_rate: f64,
    #[arg(
        long,
        default_value_t = 3000.0,
        help = "Annual CGT allowance when realizing gains"
    )]
    capital_gains_allowance: f64,
    #[arg(
        long,
        default_value_t = 1.0,
        help = "Annual tax drag on taxable account returns in percent"
    )]
    taxable_return_tax_drag: f64,
    #[arg(
        long,
        value_enum,
        default_value_t = CliPensionTaxMode::UkBands,
        help = "Pension tax model: UK progressive bands or legacy flat rate"
    )]
    pension_tax_mode: CliPensionTaxMode,
    #[arg(
        long,
        default_value_t = 20.0,
        help = "Flat pension tax rate in percent, used when --pension-tax-mode=flat-rate"
    )]
    pension_income_tax_rate: f64,
    #[arg(
        long,
        default_value_t = 12570.0,
        help = "UK personal allowance (today's money)"
    )]
    uk_personal_allowance: f64,
    #[arg(
        long,
        default_value_t = 50270.0,
        help = "Upper income bound for UK basic rate band (today's money)"
    )]
    uk_basic_rate_limit: f64,
    #[arg(
        long,
        default_value_t = 125140.0,
        help = "Upper income bound for UK higher rate band (today's money)"
    )]
    uk_higher_rate_limit: f64,
    #[arg(long, default_value_t = 20.0, help = "UK basic tax rate in percent")]
    uk_basic_rate: f64,
    #[arg(long, default_value_t = 40.0, help = "UK higher tax rate in percent")]
    uk_higher_rate: f64,
    #[arg(
        long,
        default_value_t = 45.0,
        help = "UK additional tax rate in percent"
    )]
    uk_additional_rate: f64,
    #[arg(
        long,
        default_value_t = 100000.0,
        help = "Income where personal allowance taper starts (today's money)"
    )]
    uk_allowance_taper_start: f64,
    #[arg(
        long,
        default_value_t = 125140.0,
        help = "Income where personal allowance is fully tapered away (today's money)"
    )]
    uk_allowance_taper_end: f64,
    #[arg(long, default_value_t = 67, help = "State pension start age")]
    state_pension_start_age: u32,
    #[arg(
        long,
        default_value_t = 0.0,
        help = "Annual state pension income in today's money"
    )]
    state_pension_annual_income: f64,
    #[arg(
        long,
        default_value_t = 2.5,
        help = "Expected annual inflation in percent"
    )]
    inflation_rate: f64,
    #[arg(long, default_value_t = 1.0, help = "Inflation volatility in percent")]
    inflation_volatility: f64,
    #[arg(long)]
    target_annual_income: f64,
    #[arg(
        long,
        default_value_t = 0.0,
        help = "Annual mortgage payment in today's money while mortgage is active"
    )]
    mortgage_annual_payment: f64,
    #[arg(
        long,
        help = "Age when mortgage payments stop; required when --mortgage-annual-payment > 0"
    )]
    mortgage_end_age: Option<u32>,
    #[arg(long, default_value_t = 75, help = "Latest retirement age to test")]
    max_age: u32,
    #[arg(long, default_value_t = 95, help = "Age to fund through")]
    horizon_age: u32,
    #[arg(long, default_value_t = 10000)]
    simulations: u32,
    #[arg(
        long,
        default_value_t = 90.0,
        help = "Required Monte Carlo success probability in percent"
    )]
    success_threshold: f64,
    #[arg(long, default_value_t = 42)]
    seed: u64,
    #[arg(long, default_value_t = -5.0, help = "Bad-year real return threshold in percent")]
    bad_year_threshold: f64,
    #[arg(
        long,
        default_value_t = 10.0,
        help = "Good-year real return threshold in percent"
    )]
    good_year_threshold: f64,
    #[arg(
        long,
        default_value_t = 10.0,
        help = "Bad-year spending cut in percent"
    )]
    bad_year_cut: f64,
    #[arg(
        long,
        default_value_t = 5.0,
        help = "Good-year spending raise in percent"
    )]
    good_year_raise: f64,
    #[arg(
        long,
        default_value_t = 80.0,
        help = "Minimum income floor vs target in percent"
    )]
    min_income_floor: f64,
    #[arg(
        long,
        default_value_t = 130.0,
        help = "Maximum income ceiling vs target in percent"
    )]
    max_income_ceiling: f64,
    #[arg(
        long,
        value_enum,
        default_value_t = CliWithdrawalStrategy::Guardrails,
        help = "Withdrawal strategy: guardrails, Guyton-Klinger, VPW, floor+upside, or bucket"
    )]
    withdrawal_strategy: CliWithdrawalStrategy,
    #[arg(
        long,
        default_value_t = 80.0,
        help = "Guyton-Klinger lower guardrail as percent of initial withdrawal rate"
    )]
    gk_lower_guardrail: f64,
    #[arg(
        long,
        default_value_t = 120.0,
        help = "Guyton-Klinger upper guardrail as percent of initial withdrawal rate"
    )]
    gk_upper_guardrail: f64,
    #[arg(
        long,
        default_value_t = 3.5,
        help = "VPW expected real return assumption in percent"
    )]
    vpw_expected_real_return: f64,
    #[arg(
        long,
        default_value_t = 50.0,
        help = "Floor+upside: share of positive real returns converted into spending growth in percent"
    )]
    floor_upside_capture: f64,
    #[arg(
        long,
        default_value_t = 2.0,
        help = "Bucket strategy target cash reserve in years of spending"
    )]
    bucket_target_years: f64,
    #[arg(
        long,
        default_value_t = 10.0,
        help = "In good years, extra withdrawal to store in cash buffer (percent of spending)"
    )]
    good_year_extra_buffer_withdrawal: f64,
    #[arg(long, default_value_t = 1.0, help = "Cash buffer growth in percent")]
    cash_growth_rate: f64,
    #[arg(long, value_enum, default_value_t = CliWithdrawalOrder::ProRata)]
    post_access_withdrawal_order: CliWithdrawalOrder,
}

#[derive(Copy, Clone, Debug)]
struct ApiOptions {
    mode: AnalysisMode,
    coast_retirement_age: Option<u32>,
}

#[derive(Debug)]
struct ApiRequest {
    inputs: Inputs,
    options: ApiOptions,
}

#[derive(Copy, Clone)]
struct CashflowResponse<'a> {
    candidate_age: u32,
    retirement_age: u32,
    contribution_stop_age: u32,
    years: &'a [CashflowYearResult],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SimulateResponse {
    mode: ResponseMode,
    withdrawal_policy: ApiWithdrawalStrategy,
    coast_retirement_age: Option<u32>,
    success_threshold: f64,
    selected_retirement_age: Option<u32>,
    best_retirement_age: u32,
    cashflow_candidate_age: u32,
    cashflow_retirement_age: u32,
    cashflow_contribution_stop_age: u32,
    age_results: Vec<AgeResult>,
    cashflow_years: Vec<CashflowYearResult>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

fn build_inputs(cli: Cli) -> Result<Inputs, String> {
    if cli.pension_access_age < cli.current_age {
        return Err("--pension-access-age must be >= --current-age".to_string());
    }

    if cli.max_age < cli.current_age {
        return Err("--max-age must be >= --current-age".to_string());
    }

    if cli.horizon_age <= cli.max_age {
        return Err("--horizon-age must be > --max-age".to_string());
    }

    if cli.simulations == 0 {
        return Err("--simulations must be > 0".to_string());
    }

    if !(0.0..=100.0).contains(&cli.success_threshold) {
        return Err("--success-threshold must be between 0 and 100".to_string());
    }

    if !(-1.0..=1.0).contains(&cli.return_correlation) {
        return Err("--return-correlation must be between -1 and 1".to_string());
    }

    if cli.target_annual_income <= 0.0 {
        return Err("--target-annual-income must be > 0".to_string());
    }

    if !cli.mortgage_annual_payment.is_finite() || cli.mortgage_annual_payment < 0.0 {
        return Err("--mortgage-annual-payment must be >= 0".to_string());
    }

    if cli.mortgage_annual_payment > 0.0 {
        let Some(end_age) = cli.mortgage_end_age else {
            return Err(
                "--mortgage-end-age is required when --mortgage-annual-payment > 0".to_string(),
            );
        };
        if end_age <= cli.current_age {
            return Err("--mortgage-end-age must be > --current-age".to_string());
        }
    }

    if cli.cash_start < 0.0 {
        return Err("--cash-start must be >= 0".to_string());
    }

    if !(0.0..=100.0).contains(&cli.capital_gains_tax_rate) {
        return Err("--capital-gains-tax-rate must be between 0 and 100".to_string());
    }

    if cli.capital_gains_allowance < 0.0 {
        return Err("--capital-gains-allowance must be >= 0".to_string());
    }

    if !(0.0..=100.0).contains(&cli.taxable_return_tax_drag) {
        return Err("--taxable-return-tax-drag must be between 0 and 100".to_string());
    }

    if cli.taxable_cost_basis_start < 0.0 || cli.taxable_cost_basis_start > cli.taxable_start {
        return Err("--taxable-cost-basis-start must be between 0 and taxable-start".to_string());
    }

    if cli.min_income_floor <= 0.0 || cli.max_income_ceiling <= 0.0 {
        return Err("--min-income-floor and --max-income-ceiling must be > 0".to_string());
    }

    if cli.min_income_floor > cli.max_income_ceiling {
        return Err("--min-income-floor cannot exceed --max-income-ceiling".to_string());
    }

    if !cli.gk_lower_guardrail.is_finite() || cli.gk_lower_guardrail <= 0.0 {
        return Err("--gk-lower-guardrail must be > 0".to_string());
    }

    if !cli.gk_upper_guardrail.is_finite() || cli.gk_upper_guardrail <= 0.0 {
        return Err("--gk-upper-guardrail must be > 0".to_string());
    }

    if cli.gk_upper_guardrail < cli.gk_lower_guardrail {
        return Err("--gk-upper-guardrail must be >= --gk-lower-guardrail".to_string());
    }

    if !cli.vpw_expected_real_return.is_finite() || cli.vpw_expected_real_return <= -100.0 {
        return Err("--vpw-expected-real-return must be > -100".to_string());
    }

    if !(0.0..=300.0).contains(&cli.floor_upside_capture) {
        return Err("--floor-upside-capture must be between 0 and 300".to_string());
    }

    if !cli.bucket_target_years.is_finite() || cli.bucket_target_years < 0.0 {
        return Err("--bucket-target-years must be >= 0".to_string());
    }

    if cli.isa_annual_contribution_limit < 0.0 {
        return Err("--isa-annual-contribution-limit must be >= 0".to_string());
    }

    if !cli.contribution_growth_rate.is_finite() || cli.contribution_growth_rate <= -100.0 {
        return Err("--contribution-growth-rate must be > -100".to_string());
    }

    if !(0.0..=100.0).contains(&cli.pension_income_tax_rate) {
        return Err("--pension-income-tax-rate must be between 0 and 100".to_string());
    }

    for (name, rate) in [
        ("--uk-basic-rate", cli.uk_basic_rate),
        ("--uk-higher-rate", cli.uk_higher_rate),
        ("--uk-additional-rate", cli.uk_additional_rate),
    ] {
        if !(0.0..=100.0).contains(&rate) {
            return Err(format!("{name} must be between 0 and 100"));
        }
    }

    if cli.uk_personal_allowance < 0.0
        || cli.uk_basic_rate_limit < 0.0
        || cli.uk_higher_rate_limit < 0.0
        || cli.uk_allowance_taper_start < 0.0
        || cli.uk_allowance_taper_end < 0.0
    {
        return Err("UK tax thresholds must be >= 0".to_string());
    }

    if cli.uk_basic_rate_limit < cli.uk_personal_allowance {
        return Err("--uk-basic-rate-limit must be >= --uk-personal-allowance".to_string());
    }

    if cli.uk_higher_rate_limit < cli.uk_basic_rate_limit {
        return Err("--uk-higher-rate-limit must be >= --uk-basic-rate-limit".to_string());
    }

    if cli.uk_allowance_taper_end <= cli.uk_allowance_taper_start {
        return Err("--uk-allowance-taper-end must be > --uk-allowance-taper-start".to_string());
    }

    if cli.state_pension_annual_income < 0.0 {
        return Err("--state-pension-annual-income must be >= 0".to_string());
    }

    let taxable_growth_rate = cli.taxable_growth_rate.unwrap_or(cli.isa_growth_rate);
    let taxable_return_volatility = cli
        .taxable_return_volatility
        .unwrap_or(cli.isa_return_volatility);

    Ok(Inputs {
        current_age: cli.current_age,
        pension_access_age: cli.pension_access_age,
        isa_start: cli.isa_start,
        taxable_start: cli.taxable_start,
        taxable_cost_basis_start: if cli.taxable_cost_basis_start == 0.0 && cli.taxable_start > 0.0
        {
            cli.taxable_start
        } else {
            cli.taxable_cost_basis_start
        },
        pension_start: cli.pension_start,
        cash_start: cli.cash_start,
        isa_annual_contribution: cli.isa_annual_contribution,
        isa_annual_contribution_limit: cli.isa_annual_contribution_limit,
        taxable_annual_contribution: cli.taxable_annual_contribution,
        pension_annual_contribution: cli.pension_annual_contribution,
        contribution_growth_rate: cli.contribution_growth_rate / 100.0,
        isa_return_mean: cli.isa_growth_rate / 100.0,
        isa_return_vol: cli.isa_return_volatility / 100.0,
        taxable_return_mean: taxable_growth_rate / 100.0,
        taxable_return_vol: taxable_return_volatility / 100.0,
        pension_return_mean: cli.pension_growth_rate / 100.0,
        pension_return_vol: cli.pension_return_volatility / 100.0,
        return_correlation: cli.return_correlation,
        capital_gains_tax_rate: cli.capital_gains_tax_rate / 100.0,
        capital_gains_allowance: cli.capital_gains_allowance,
        taxable_return_tax_drag: cli.taxable_return_tax_drag / 100.0,
        pension_tax_mode: cli.pension_tax_mode.into(),
        pension_flat_tax_rate: cli.pension_income_tax_rate / 100.0,
        uk_personal_allowance: cli.uk_personal_allowance,
        uk_basic_rate_limit: cli.uk_basic_rate_limit,
        uk_higher_rate_limit: cli.uk_higher_rate_limit,
        uk_basic_rate: cli.uk_basic_rate / 100.0,
        uk_higher_rate: cli.uk_higher_rate / 100.0,
        uk_additional_rate: cli.uk_additional_rate / 100.0,
        uk_allowance_taper_start: cli.uk_allowance_taper_start,
        uk_allowance_taper_end: cli.uk_allowance_taper_end,
        state_pension_start_age: cli.state_pension_start_age,
        state_pension_annual_income: cli.state_pension_annual_income,
        inflation_mean: cli.inflation_rate / 100.0,
        inflation_vol: cli.inflation_volatility / 100.0,
        target_annual_income: cli.target_annual_income,
        mortgage_annual_payment: cli.mortgage_annual_payment,
        mortgage_end_age: cli.mortgage_end_age,
        max_retirement_age: cli.max_age,
        horizon_age: cli.horizon_age,
        simulations: cli.simulations,
        success_threshold: cli.success_threshold / 100.0,
        seed: cli.seed,
        bad_year_threshold: cli.bad_year_threshold / 100.0,
        good_year_threshold: cli.good_year_threshold / 100.0,
        bad_year_cut: cli.bad_year_cut / 100.0,
        good_year_raise: cli.good_year_raise / 100.0,
        min_income_floor: cli.min_income_floor / 100.0,
        max_income_ceiling: cli.max_income_ceiling / 100.0,
        withdrawal_strategy: cli.withdrawal_strategy.into(),
        gk_lower_guardrail: cli.gk_lower_guardrail / 100.0,
        gk_upper_guardrail: cli.gk_upper_guardrail / 100.0,
        vpw_expected_real_return: cli.vpw_expected_real_return / 100.0,
        floor_upside_capture: cli.floor_upside_capture / 100.0,
        bucket_target_years: cli.bucket_target_years,
        good_year_extra_buffer_withdrawal: cli.good_year_extra_buffer_withdrawal / 100.0,
        cash_growth_rate: cli.cash_growth_rate / 100.0,
        post_access_withdrawal_order: cli.post_access_withdrawal_order.into(),
    })
}

pub async fn run_http_server(port: u16) -> std::io::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/index.html", get(index_handler))
        .route("/styles.css", get(styles_handler))
        .route("/app.js", get(app_js_handler))
        .route(
            "/api/simulate",
            get(simulate_get_handler).post(simulate_post_handler),
        )
        .fallback(not_found_handler);

    let listener = TcpListener::bind(addr).await?;
    println!("FIRE HTTP API listening on http://{addr}");
    println!("Local access: http://127.0.0.1:{port}/");

    axum::serve(listener, app).await
}

async fn index_handler() -> impl IntoResponse {
    with_cache_control(Html(INDEX_HTML))
}

async fn styles_handler() -> impl IntoResponse {
    with_cache_control((
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLES_CSS,
    ))
}

async fn app_js_handler() -> impl IntoResponse {
    with_cache_control((
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        APP_JS,
    ))
}

async fn not_found_handler() -> Response {
    error_response(StatusCode::NOT_FOUND, "Not found")
}

async fn simulate_get_handler(Query(payload): Query<SimulatePayload>) -> Response {
    simulate_handler_impl(payload).await
}

async fn simulate_post_handler(Json(payload): Json<SimulatePayload>) -> Response {
    simulate_handler_impl(payload).await
}

async fn simulate_handler_impl(payload: SimulatePayload) -> Response {
    let request = match api_request_from_payload(payload) {
        Ok(request) => request,
        Err(msg) => return error_response(StatusCode::BAD_REQUEST, &msg),
    };

    let inputs = &request.inputs;
    let (model, resolved_coast_retirement_age) = match request.options.mode {
        AnalysisMode::RetirementSweep => (run_model(inputs), None),
        AnalysisMode::CoastFire => {
            let coast_retirement_age = request.options.coast_retirement_age.unwrap_or_else(|| {
                let baseline = run_model(inputs);
                baseline
                    .selected_index
                    .map(|idx| baseline.age_results[idx].retirement_age)
                    .unwrap_or(baseline.age_results[baseline.best_index].retirement_age)
            });
            (
                run_coast_model(inputs, coast_retirement_age),
                Some(coast_retirement_age),
            )
        }
    };

    let trace_index = model.selected_index.unwrap_or(model.best_index);
    let trace_reported_age = model.age_results[trace_index].retirement_age;
    let (trace_retirement_age, trace_contribution_stop_age) = match request.options.mode {
        AnalysisMode::RetirementSweep => (trace_reported_age, trace_reported_age),
        AnalysisMode::CoastFire => (
            resolved_coast_retirement_age.unwrap_or(trace_reported_age),
            trace_reported_age,
        ),
    };
    let cashflow_years = run_yearly_cashflow_trace(
        inputs,
        trace_retirement_age,
        trace_contribution_stop_age,
        trace_reported_age,
    );
    let cashflow = CashflowResponse {
        candidate_age: trace_reported_age,
        retirement_age: trace_retirement_age,
        contribution_stop_age: trace_contribution_stop_age,
        years: &cashflow_years,
    };

    let response = build_simulate_response(
        inputs,
        &model,
        request.options.mode,
        resolved_coast_retirement_age,
        cashflow,
    );
    json_response(StatusCode::OK, response)
}

fn with_cache_control<R: IntoResponse>(response: R) -> Response {
    let mut response = response.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "no-store".parse().expect("valid header"),
    );
    response
}

fn json_response<T: Serialize>(status: StatusCode, body: T) -> Response {
    let mut response = (status, Json(body)).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "no-store".parse().expect("valid header"),
    );
    response
}

fn error_response(status: StatusCode, msg: &str) -> Response {
    json_response(
        status,
        ErrorResponse {
            error: msg.to_string(),
        },
    )
}

#[cfg(test)]
fn api_request_from_json(json: &str) -> Result<ApiRequest, String> {
    let payload = serde_json::from_str::<SimulatePayload>(json)
        .map_err(|e| format!("Invalid API JSON payload: {e}"))?;
    api_request_from_payload(payload)
}

fn api_request_from_payload(payload: SimulatePayload) -> Result<ApiRequest, String> {
    let mut cli = default_cli_for_api();
    let mut options = ApiOptions {
        mode: AnalysisMode::RetirementSweep,
        coast_retirement_age: None,
    };

    if let Some(v) = payload.current_age {
        cli.current_age = v;
    }
    if let Some(v) = payload.pension_access_age {
        cli.pension_access_age = v;
    }
    if let Some(v) = payload.max_age {
        cli.max_age = v;
    }
    if let Some(v) = payload.horizon_age {
        cli.horizon_age = v;
    }
    if let Some(v) = payload.simulations {
        cli.simulations = v;
    }
    if let Some(v) = payload.seed {
        cli.seed = v;
    }

    if let Some(v) = payload.isa_start {
        cli.isa_start = v;
    }
    if let Some(v) = payload.taxable_start {
        cli.taxable_start = v;
    }
    if let Some(v) = payload.taxable_basis_start {
        cli.taxable_cost_basis_start = v;
    }
    if let Some(v) = payload.pension_start {
        cli.pension_start = v;
    }
    if let Some(v) = payload.cash_start {
        cli.cash_start = v;
    }

    if let Some(v) = payload.isa_contribution {
        cli.isa_annual_contribution = v;
    }
    if let Some(v) = payload.isa_limit {
        cli.isa_annual_contribution_limit = v;
    }
    if let Some(v) = payload.taxable_contribution {
        cli.taxable_annual_contribution = v;
    }
    if let Some(v) = payload.pension_contribution {
        cli.pension_annual_contribution = v;
    }
    if let Some(v) = payload.contribution_growth {
        cli.contribution_growth_rate = v;
    }

    if let Some(v) = payload.cgt_rate {
        cli.capital_gains_tax_rate = v;
    }
    if let Some(v) = payload.cgt_allowance {
        cli.capital_gains_allowance = v;
    }
    if let Some(v) = payload.taxable_tax_drag {
        cli.taxable_return_tax_drag = v;
    }

    if let Some(v) = payload.pension_tax_mode {
        cli.pension_tax_mode = v.into();
    }
    if let Some(v) = payload.pension_income_tax_rate {
        cli.pension_income_tax_rate = v;
    }
    if let Some(v) = payload.uk_personal_allowance {
        cli.uk_personal_allowance = v;
    }
    if let Some(v) = payload.uk_basic_rate_limit {
        cli.uk_basic_rate_limit = v;
    }
    if let Some(v) = payload.uk_higher_rate_limit {
        cli.uk_higher_rate_limit = v;
    }
    if let Some(v) = payload.uk_basic_rate {
        cli.uk_basic_rate = v;
    }
    if let Some(v) = payload.uk_higher_rate {
        cli.uk_higher_rate = v;
    }
    if let Some(v) = payload.uk_additional_rate {
        cli.uk_additional_rate = v;
    }
    if let Some(v) = payload.uk_allowance_taper_start {
        cli.uk_allowance_taper_start = v;
    }
    if let Some(v) = payload.uk_allowance_taper_end {
        cli.uk_allowance_taper_end = v;
    }
    if let Some(v) = payload.state_pension_start_age {
        cli.state_pension_start_age = v;
    }
    if let Some(v) = payload.state_pension_income {
        cli.state_pension_annual_income = v;
    }

    if let Some(v) = payload.isa_mean {
        cli.isa_growth_rate = v;
    }
    if let Some(v) = payload.isa_vol {
        cli.isa_return_volatility = v;
    }
    if let Some(v) = payload.taxable_mean {
        cli.taxable_growth_rate = Some(v);
    }
    if let Some(v) = payload.taxable_vol {
        cli.taxable_return_volatility = Some(v);
    }
    if let Some(v) = payload.pension_mean {
        cli.pension_growth_rate = v;
    }
    if let Some(v) = payload.pension_vol {
        cli.pension_return_volatility = v;
    }
    if let Some(v) = payload.correlation {
        cli.return_correlation = v;
    }
    if let Some(v) = payload.inflation_mean {
        cli.inflation_rate = v;
    }
    if let Some(v) = payload.inflation_vol {
        cli.inflation_volatility = v;
    }

    if let Some(v) = payload.target_income {
        cli.target_annual_income = v;
    }
    if let Some(v) = payload.mortgage_annual_payment {
        cli.mortgage_annual_payment = v;
    }
    if let Some(v) = payload.mortgage_end_age {
        cli.mortgage_end_age = Some(v);
    }
    if let Some(v) = payload.success_threshold {
        cli.success_threshold = v;
    }
    if let Some(v) = payload.bad_threshold {
        cli.bad_year_threshold = v;
    }
    if let Some(v) = payload.good_threshold {
        cli.good_year_threshold = v;
    }
    if let Some(v) = payload.bad_cut {
        cli.bad_year_cut = v;
    }
    if let Some(v) = payload.good_raise {
        cli.good_year_raise = v;
    }
    if let Some(v) = payload.min_floor {
        cli.min_income_floor = v;
    }
    if let Some(v) = payload.max_ceiling {
        cli.max_income_ceiling = v;
    }
    if let Some(v) = payload.withdrawal_policy {
        cli.withdrawal_strategy = v.into();
    }
    if let Some(v) = payload.gk_lower_guardrail {
        cli.gk_lower_guardrail = v;
    }
    if let Some(v) = payload.gk_upper_guardrail {
        cli.gk_upper_guardrail = v;
    }
    if let Some(v) = payload.vpw_real_return {
        cli.vpw_expected_real_return = v;
    }
    if let Some(v) = payload.floor_upside_capture {
        cli.floor_upside_capture = v;
    }
    if let Some(v) = payload.bucket_years_target {
        cli.bucket_target_years = v;
    }
    if let Some(v) = payload.extra_to_cash {
        cli.good_year_extra_buffer_withdrawal = v;
    }
    if let Some(v) = payload.cash_growth {
        cli.cash_growth_rate = v;
    }
    if let Some(v) = payload.withdrawal_order {
        cli.post_access_withdrawal_order = v.into();
    }

    if let Some(v) = payload.analysis_mode {
        options.mode = v.into();
    }
    if let Some(v) = payload.coast_retirement_age {
        options.coast_retirement_age = Some(v);
    }

    let inputs = build_inputs(cli)?;
    if let Some(age) = options.coast_retirement_age {
        if age < inputs.current_age {
            return Err("--coastRetirementAge must be >= currentAge".to_string());
        }
        if age >= inputs.horizon_age {
            return Err("--coastRetirementAge must be < horizonAge".to_string());
        }
    }

    Ok(ApiRequest { inputs, options })
}

fn default_cli_for_api() -> Cli {
    Cli {
        current_age: 30,
        pension_access_age: 57,
        isa_start: 100_000.0,
        taxable_start: 15_000.0,
        taxable_cost_basis_start: 12_000.0,
        pension_start: 200_000.0,
        cash_start: 0.0,
        isa_annual_contribution: 30_000.0,
        isa_annual_contribution_limit: 20_000.0,
        taxable_annual_contribution: 5_000.0,
        pension_annual_contribution: 0.0,
        contribution_growth_rate: 0.0,
        isa_growth_rate: 8.0,
        isa_return_volatility: 12.0,
        taxable_growth_rate: Some(8.0),
        taxable_return_volatility: Some(12.0),
        pension_growth_rate: 8.0,
        pension_return_volatility: 12.0,
        return_correlation: 0.8,
        capital_gains_tax_rate: 20.0,
        capital_gains_allowance: 3_000.0,
        taxable_return_tax_drag: 1.0,
        pension_tax_mode: CliPensionTaxMode::UkBands,
        pension_income_tax_rate: 20.0,
        uk_personal_allowance: 12_570.0,
        uk_basic_rate_limit: 50_270.0,
        uk_higher_rate_limit: 125_140.0,
        uk_basic_rate: 20.0,
        uk_higher_rate: 40.0,
        uk_additional_rate: 45.0,
        uk_allowance_taper_start: 100_000.0,
        uk_allowance_taper_end: 125_140.0,
        state_pension_start_age: 67,
        state_pension_annual_income: 0.0,
        inflation_rate: 2.5,
        inflation_volatility: 1.0,
        target_annual_income: 50_000.0,
        mortgage_annual_payment: 0.0,
        mortgage_end_age: None,
        max_age: 70,
        horizon_age: 90,
        simulations: 3_000,
        success_threshold: 90.0,
        seed: 42,
        bad_year_threshold: -5.0,
        good_year_threshold: 10.0,
        bad_year_cut: 10.0,
        good_year_raise: 5.0,
        min_income_floor: 80.0,
        max_income_ceiling: 200.0,
        withdrawal_strategy: CliWithdrawalStrategy::Guardrails,
        gk_lower_guardrail: 80.0,
        gk_upper_guardrail: 120.0,
        vpw_expected_real_return: 3.5,
        floor_upside_capture: 50.0,
        bucket_target_years: 2.0,
        good_year_extra_buffer_withdrawal: 10.0,
        cash_growth_rate: 1.0,
        post_access_withdrawal_order: CliWithdrawalOrder::ProRata,
    }
}

fn build_simulate_response(
    inputs: &Inputs,
    model: &ModelResult,
    mode: AnalysisMode,
    coast_retirement_age: Option<u32>,
    cashflow: CashflowResponse<'_>,
) -> SimulateResponse {
    SimulateResponse {
        mode: mode.into(),
        withdrawal_policy: inputs.withdrawal_strategy.into(),
        coast_retirement_age,
        success_threshold: inputs.success_threshold,
        selected_retirement_age: model
            .selected_index
            .map(|idx| model.age_results[idx].retirement_age),
        best_retirement_age: model.age_results[model.best_index].retirement_age,
        cashflow_candidate_age: cashflow.candidate_age,
        cashflow_retirement_age: cashflow.retirement_age,
        cashflow_contribution_stop_age: cashflow.contribution_stop_age,
        age_results: model.age_results.clone(),
        cashflow_years: cashflow.years.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    const EPS: f64 = 1e-6;

    fn assert_approx(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPS,
            "expected {expected}, got {actual}"
        );
    }

    fn sample_cli() -> Cli {
        default_cli_for_api()
    }

    fn assert_golden_snapshot(path: &str, actual: &str) {
        let update = matches!(
            std::env::var("UPDATE_GOLDEN").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE")
        );
        let snapshot_path = Path::new(path);

        if update {
            if let Some(parent) = snapshot_path.parent() {
                fs::create_dir_all(parent).expect("failed to create snapshot directory");
            }
            fs::write(snapshot_path, actual).expect("failed to write golden snapshot");
            return;
        }

        let expected = fs::read_to_string(snapshot_path).unwrap_or_else(|_| {
            panic!("missing golden snapshot at {path}; run with UPDATE_GOLDEN=1 to generate")
        });
        assert_eq!(
            actual, expected,
            "snapshot mismatch for {path}; run with UPDATE_GOLDEN=1 to refresh if expected"
        );
    }

    #[test]
    fn build_inputs_defaults_taxable_basis_to_start_when_zero() {
        let mut cli = sample_cli();
        cli.taxable_start = 20_000.0;
        cli.taxable_cost_basis_start = 0.0;

        let inputs = build_inputs(cli).expect("valid inputs");
        assert_approx(inputs.taxable_cost_basis_start, 20_000.0);
    }

    #[test]
    fn build_inputs_rejects_invalid_taxable_basis() {
        let mut cli = sample_cli();
        cli.taxable_start = 10_000.0;
        cli.taxable_cost_basis_start = 12_000.0;

        let err = build_inputs(cli).expect_err("must reject invalid basis");
        assert!(err.contains("--taxable-cost-basis-start"));
    }

    #[test]
    fn build_inputs_rejects_invalid_contribution_growth_rate() {
        let mut cli = sample_cli();
        cli.contribution_growth_rate = -100.0;
        let err = build_inputs(cli).expect_err("must reject <= -100 growth rate");
        assert!(err.contains("--contribution-growth-rate"));
    }

    #[test]
    fn build_inputs_rejects_invalid_uk_band_order() {
        let mut cli = sample_cli();
        cli.uk_basic_rate_limit = 10_000.0;
        cli.uk_personal_allowance = 12_570.0;

        let err = build_inputs(cli).expect_err("must reject bad UK threshold order");
        assert!(err.contains("--uk-basic-rate-limit"));
    }

    #[test]
    fn build_inputs_uses_isa_defaults_for_taxable_return_params() {
        let mut cli = sample_cli();
        cli.taxable_growth_rate = None;
        cli.taxable_return_volatility = None;

        let inputs = build_inputs(cli).expect("valid inputs");
        assert_approx(inputs.taxable_return_mean, inputs.isa_return_mean);
        assert_approx(inputs.taxable_return_vol, inputs.isa_return_vol);
    }

    #[test]
    fn api_request_from_json_parses_web_keys() {
        let json = r#"{
          "currentAge": 31,
          "pensionAccessAge": 58,
          "isaStart": 120000,
          "taxableStart": 20000,
          "taxableBasisStart": 15000,
          "pensionStart": 250000,
          "cashStart": 5000,
          "targetIncome": 45000,
          "mortgageAnnualPayment": 12000,
          "mortgageEndAge": 40,
          "withdrawalOrder": "taxable-first",
          "simulations": 1234,
          "contributionGrowth": 3,
          "pensionTaxMode": "uk-bands",
          "statePensionStartAge": 67,
          "statePensionIncome": 12000,
          "withdrawalPolicy": "vpw",
          "vpwRealReturn": 4.2
        }"#;
        let request = api_request_from_json(json).expect("json should parse");
        let inputs = request.inputs;

        assert_eq!(inputs.current_age, 31);
        assert_eq!(inputs.pension_access_age, 58);
        assert_approx(inputs.isa_start, 120_000.0);
        assert_approx(inputs.taxable_start, 20_000.0);
        assert_approx(inputs.taxable_cost_basis_start, 15_000.0);
        assert_approx(inputs.pension_start, 250_000.0);
        assert_approx(inputs.cash_start, 5_000.0);
        assert_approx(inputs.target_annual_income, 45_000.0);
        assert_approx(inputs.mortgage_annual_payment, 12_000.0);
        assert_eq!(inputs.mortgage_end_age, Some(40));
        assert_approx(inputs.contribution_growth_rate, 0.03);
        assert_eq!(inputs.state_pension_start_age, 67);
        assert_approx(inputs.state_pension_annual_income, 12_000.0);
        assert_eq!(inputs.simulations, 1234);
        assert_eq!(inputs.withdrawal_strategy, WithdrawalStrategy::Vpw);
        assert_approx(inputs.vpw_expected_real_return, 0.042);
        assert_eq!(
            inputs.post_access_withdrawal_order,
            WithdrawalOrder::TaxableFirst
        );
        assert_eq!(inputs.pension_tax_mode, PensionTaxMode::UkBands);
    }

    #[test]
    fn build_inputs_rejects_mortgage_payment_without_end_age() {
        let mut cli = sample_cli();
        cli.mortgage_annual_payment = 10_000.0;
        cli.mortgage_end_age = None;

        let err = build_inputs(cli).expect_err("must require mortgage end age");
        assert!(err.contains("--mortgage-end-age"));
    }

    #[test]
    fn api_request_from_json_parses_coast_mode_and_retirement_age() {
        let json = r#"{
          "analysisMode": "coast-fire",
          "coastRetirementAge": 60,
          "currentAge": 31,
          "horizonAge": 90
        }"#;
        let request = api_request_from_json(json).expect("json should parse");
        assert_eq!(request.options.mode, AnalysisMode::CoastFire);
        assert_eq!(request.options.coast_retirement_age, Some(60));
        assert_eq!(request.inputs.current_age, 31);
    }

    #[test]
    fn build_inputs_rejects_invalid_guardrail_range() {
        let mut cli = sample_cli();
        cli.gk_lower_guardrail = 130.0;
        cli.gk_upper_guardrail = 120.0;

        let err = build_inputs(cli).expect_err("must reject invalid guardrail range");
        assert!(err.contains("--gk-upper-guardrail"));
    }

    #[test]
    fn simulate_response_serialization_contains_expected_fields() {
        let mut cli = sample_cli();
        cli.current_age = 30;
        cli.max_age = 30;
        cli.horizon_age = 31;
        cli.simulations = 3;
        cli.target_annual_income = 1.0;
        cli.isa_return_volatility = 0.0;
        cli.taxable_return_volatility = Some(0.0);
        cli.pension_return_volatility = 0.0;
        cli.inflation_volatility = 0.0;

        let inputs = build_inputs(cli).expect("valid inputs");
        let model = run_model(&inputs);
        let trace_index = model.selected_index.unwrap_or(model.best_index);
        let trace_candidate_age = model.age_results[trace_index].retirement_age;
        let cashflow = run_yearly_cashflow_trace(
            &inputs,
            trace_candidate_age,
            trace_candidate_age,
            trace_candidate_age,
        );
        let cashflow_response = CashflowResponse {
            candidate_age: trace_candidate_age,
            retirement_age: trace_candidate_age,
            contribution_stop_age: trace_candidate_age,
            years: &cashflow,
        };
        let response = build_simulate_response(
            &inputs,
            &model,
            AnalysisMode::RetirementSweep,
            None,
            cashflow_response,
        );
        let json = serde_json::to_string(&response).expect("response should serialize");
        assert!(json.contains("\"ageResults\""));
        assert!(json.contains("\"cashflowYears\""));
        assert!(json.contains("\"mode\""));
        assert!(json.contains("\"withdrawalPolicy\""));
        assert!(json.contains("\"selectedRetirementAge\""));
        assert!(json.contains("\"bestRetirementAge\""));
        assert!(json.contains("\"medianRetirementPot\""));
    }

    #[test]
    fn golden_snapshot_retirement_sweep_json() {
        let mut cli = sample_cli();
        cli.current_age = 30;
        cli.max_age = 34;
        cli.horizon_age = 45;
        cli.simulations = 80;
        cli.seed = 7;
        cli.taxable_return_volatility = Some(10.0);
        cli.pension_return_volatility = 10.0;
        cli.inflation_volatility = 0.8;
        cli.withdrawal_strategy = CliWithdrawalStrategy::Guardrails;

        let inputs = build_inputs(cli).expect("valid inputs");
        let model = run_model(&inputs);
        let trace_index = model.selected_index.unwrap_or(model.best_index);
        let trace_candidate_age = model.age_results[trace_index].retirement_age;
        let cashflow = run_yearly_cashflow_trace(
            &inputs,
            trace_candidate_age,
            trace_candidate_age,
            trace_candidate_age,
        );
        let cashflow_response = CashflowResponse {
            candidate_age: trace_candidate_age,
            retirement_age: trace_candidate_age,
            contribution_stop_age: trace_candidate_age,
            years: &cashflow,
        };
        let response = build_simulate_response(
            &inputs,
            &model,
            AnalysisMode::RetirementSweep,
            None,
            cashflow_response,
        );
        let json = format!(
            "{}\n",
            serde_json::to_string(&response).expect("response should serialize")
        );

        assert_golden_snapshot("tests/golden/retirement_sweep_guardrails.json", &json);
    }

    #[test]
    fn golden_snapshot_coast_fire_vpw_json() {
        let mut cli = sample_cli();
        cli.current_age = 30;
        cli.max_age = 36;
        cli.horizon_age = 50;
        cli.simulations = 80;
        cli.seed = 11;
        cli.target_annual_income = 45_000.0;
        cli.withdrawal_strategy = CliWithdrawalStrategy::Vpw;
        cli.vpw_expected_real_return = 3.0;
        cli.taxable_return_volatility = Some(11.0);
        cli.pension_return_volatility = 11.0;
        cli.inflation_volatility = 0.9;

        let inputs = build_inputs(cli).expect("valid inputs");
        let retirement_age = 35;
        let model = run_coast_model(&inputs, retirement_age);
        let trace_index = model.selected_index.unwrap_or(model.best_index);
        let trace_candidate_age = model.age_results[trace_index].retirement_age;
        let cashflow = run_yearly_cashflow_trace(
            &inputs,
            retirement_age,
            trace_candidate_age,
            trace_candidate_age,
        );
        let cashflow_response = CashflowResponse {
            candidate_age: trace_candidate_age,
            retirement_age,
            contribution_stop_age: trace_candidate_age,
            years: &cashflow,
        };
        let response = build_simulate_response(
            &inputs,
            &model,
            AnalysisMode::CoastFire,
            Some(retirement_age),
            cashflow_response,
        );
        let json = format!(
            "{}\n",
            serde_json::to_string(&response).expect("response should serialize")
        );

        assert_golden_snapshot("tests/golden/coast_fire_vpw.json", &json);
    }
}
