(() => {
  const FORM_STATE_KEY = "fire.form.v1";
  const PRESET_STORE_KEY = "fire.presets.v1";
  const INPUT_MODE_KEY = "fire.input_mode.v1";
  const STORAGE_SCHEMA_VERSION = 1;
  const AUTOSAVE_DELAY_MS = 300;

  const BASIC_VISIBLE_FIELDS = new Set([
    "currentAge",
    "pensionAccessAge",
    "isaStart",
    "taxableStart",
    "pensionStart",
    "isaContribution",
    "taxableContribution",
    "pensionContribution",
    "isaLimit",
    "riskProfile",
    "targetIncome",
    "mortgageAnnualPayment",
    "mortgageEndAge"
  ]);

  const BASIC_RISK_PROFILES = {
    conservative: {
      withdrawalPolicy: "guardrails",
      isaMean: "5",
      taxableMean: "5",
      pensionMean: "5",
      isaVol: "9",
      taxableVol: "9",
      pensionVol: "9",
      badCut: "12",
      goodRaise: "3",
      minFloor: "90",
      maxCeiling: "130",
      successThreshold: "92",
      inflationMean: "2.5",
      inflationVol: "1.0",
      correlation: "0.8"
    },
    balanced: {
      withdrawalPolicy: "guardrails",
      isaMean: "8",
      taxableMean: "8",
      pensionMean: "8",
      isaVol: "12",
      taxableVol: "12",
      pensionVol: "12",
      badCut: "10",
      goodRaise: "5",
      minFloor: "80",
      maxCeiling: "180",
      successThreshold: "90",
      inflationMean: "2.5",
      inflationVol: "1.0",
      correlation: "0.8"
    },
    growth: {
      withdrawalPolicy: "guardrails",
      isaMean: "10",
      taxableMean: "10",
      pensionMean: "10",
      isaVol: "18",
      taxableVol: "18",
      pensionVol: "18",
      badCut: "12",
      goodRaise: "6",
      minFloor: "70",
      maxCeiling: "220",
      successThreshold: "85",
      inflationMean: "2.5",
      inflationVol: "1.0",
      correlation: "0.8"
    }
  };

  const QUICK_PRESETS = {
    conservative: {
      isaMean: "5",
      taxableMean: "5",
      pensionMean: "5",
      isaVol: "10",
      taxableVol: "10",
      pensionVol: "10",
      withdrawalPolicy: "guardrails",
      minFloor: "85",
      maxCeiling: "130",
      badCut: "12",
      goodRaise: "3",
      successThreshold: "92"
    },
    balanced: {
      isaMean: "8",
      taxableMean: "8",
      pensionMean: "8",
      isaVol: "12",
      taxableVol: "12",
      pensionVol: "12",
      withdrawalPolicy: "guardrails",
      minFloor: "80",
      maxCeiling: "180",
      badCut: "10",
      goodRaise: "5",
      successThreshold: "90"
    },
    aggressive: {
      isaMean: "10",
      taxableMean: "10",
      pensionMean: "10",
      isaVol: "18",
      taxableVol: "18",
      pensionVol: "18",
      withdrawalPolicy: "vpw",
      vpwRealReturn: "4",
      minFloor: "70",
      maxCeiling: "220",
      successThreshold: "85"
    },
    coastfire: {
      analysisMode: "coast-fire",
      withdrawalPolicy: "vpw",
      vpwRealReturn: "3.5",
      contributionGrowth: "2",
      successThreshold: "90",
      maxAge: "75",
      minFloor: "80",
      maxCeiling: "180"
    }
  };

  const CURRENCY_FIELDS = new Set([
    "isaStart",
    "taxableStart",
    "taxableBasisStart",
    "pensionStart",
    "cashStart",
    "isaContribution",
    "isaLimit",
    "taxableContribution",
    "pensionContribution",
    "cgtAllowance",
    "statePensionIncome",
    "ukPersonalAllowance",
    "ukBasicRateLimit",
    "ukHigherRateLimit",
    "ukAllowanceTaperStart",
    "ukAllowanceTaperEnd",
    "targetIncome",
    "mortgageAnnualPayment"
  ]);

  const PERCENT_FIELDS = new Set([
    "contributionGrowth",
    "cgtRate",
    "pensionIncomeTaxRate",
    "ukBasicRate",
    "ukHigherRate",
    "ukAdditionalRate",
    "isaMean",
    "isaVol",
    "taxableMean",
    "taxableVol",
    "taxableTaxDrag",
    "pensionMean",
    "pensionVol",
    "inflationMean",
    "inflationVol",
    "badThreshold",
    "goodThreshold",
    "badCut",
    "goodRaise",
    "minFloor",
    "maxCeiling",
    "extraToCash",
    "cashGrowth",
    "gkLowerGuardrail",
    "gkUpperGuardrail",
    "vpwRealReturn",
    "floorUpsideCapture",
    "successThreshold"
  ]);

  const AGE_FIELDS = new Set([
    "currentAge",
    "pensionAccessAge",
    "maxAge",
    "horizonAge",
    "statePensionStartAge",
    "coastRetirementAge",
    "mortgageEndAge"
  ]);

  const COUNT_FIELDS = new Set(["simulations", "seed"]);
  const YEAR_FIELDS = new Set(["bucketYearsTarget"]);
  const RATIO_FIELDS = new Set(["correlation"]);

  const form = document.getElementById("config-form");
  const inputModeSelect = document.getElementById("input-mode");
  const runBtn = document.getElementById("run-btn");
  const csvBtn = document.getElementById("csv-btn");
  const inlineValidation = document.getElementById("inline-validation");
  const presetNameInput = document.getElementById("preset-name");
  const presetSelect = document.getElementById("preset-select");
  const presetSaveBtn = document.getElementById("preset-save-btn");
  const presetLoadBtn = document.getElementById("preset-load-btn");
  const presetDeleteBtn = document.getElementById("preset-delete-btn");
  const resetDefaultsBtn = document.getElementById("reset-defaults-btn");
  const presetMeta = document.getElementById("preset-meta");
  const summaryCards = document.getElementById("summary-cards");
  const runMeta = document.getElementById("run-meta");
  const tableBody = document.querySelector("#age-table tbody");
  const ageColHeader = document.getElementById("age-col-header");
  const chart = document.getElementById("success-chart");
  const quickPresetButtons = Array.from(
    document.querySelectorAll(".quick-preset")
  );
  const liveSummaryEls = {
    startTotal: document.querySelector("[data-summary='startTotal']"),
    annualContribution: document.querySelector(
      "[data-summary='annualContribution']"
    ),
    isaOverflow: document.querySelector("[data-summary='isaOverflow']"),
    targetIncome: document.querySelector("[data-summary='targetIncome']"),
    mortgage: document.querySelector("[data-summary='mortgage']"),
    modeStrategy: document.querySelector("[data-summary='modeStrategy']"),
    retirementWindow: document.querySelector("[data-summary='retirementWindow']"),
    simScale: document.querySelector("[data-summary='simScale']")
  };

  const gbpFormatter = new Intl.NumberFormat("en-GB", {
    style: "currency",
    currency: "GBP",
    maximumFractionDigits: 0
  });

  let lastResults = null;
  let successChart = null;
  let autosaveTimer = null;
  let isRunning = false;
  let hasClientErrors = false;
  const fieldNames = collectFieldNames();
  const defaultFormValues = serializeForm();
  let presetStore = loadPresetStore();

  initFieldTooltips();
  initValueHintPlaceholders();
  restoreFormState();
  restoreInputMode();
  initializePresetControls();
  initializeQuickPresets();
  attachAutosaveListeners();

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    runSimulation();
  });

  if (inputModeSelect) {
    inputModeSelect.addEventListener("change", () => {
      persistInputMode();
      refreshDynamicUI();
    });
  }

  if (presetSaveBtn) {
    presetSaveBtn.addEventListener("click", savePreset);
  }

  if (presetLoadBtn) {
    presetLoadBtn.addEventListener("click", loadPreset);
  }

  if (presetDeleteBtn) {
    presetDeleteBtn.addEventListener("click", deletePreset);
  }

  if (resetDefaultsBtn) {
    resetDefaultsBtn.addEventListener("click", resetToDefaults);
  }

  csvBtn.addEventListener("click", () => {
    if (!lastResults) {
      return;
    }

    const header = [
      "age",
      "success_rate",
      "median_retirement_total_today",
      "p10_retirement_total_today",
      "median_retirement_isa_today",
      "median_retirement_taxable_today",
      "median_retirement_pension_today",
      "median_retirement_cash_today",
      "p10_retirement_isa_today",
      "p10_retirement_taxable_today",
      "p10_retirement_pension_today",
      "p10_retirement_cash_today",
      "median_total_today",
      "p10_total_today",
      "median_isa_today",
      "median_taxable_today",
      "median_pension_today",
      "median_cash_today",
      "p10_isa_today",
      "p10_taxable_today",
      "p10_pension_today",
      "p10_cash_today",
      "p10_min_income_ratio",
      "median_avg_income_ratio"
    ];

    const rows = lastResults.ageResults.map((r) => [
      r.retirementAge,
      r.successRate,
      r.medianRetirementPot,
      r.p10RetirementPot,
      r.medianRetirementIsa,
      r.medianRetirementTaxable,
      r.medianRetirementPension,
      r.medianRetirementCash,
      r.p10RetirementIsa,
      r.p10RetirementTaxable,
      r.p10RetirementPension,
      r.p10RetirementCash,
      r.medianTerminalPot,
      r.p10TerminalPot,
      r.medianTerminalIsa,
      r.medianTerminalTaxable,
      r.medianTerminalPension,
      r.medianTerminalCash,
      r.p10TerminalIsa,
      r.p10TerminalTaxable,
      r.p10TerminalPension,
      r.p10TerminalCash,
      r.p10MinIncomeRatio,
      r.medianAvgIncomeRatio
    ]);

    const csv = [header.join(",")]
      .concat(rows.map((row) => row.join(",")))
      .join("\n");

    const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = "fire-age-sweep.csv";
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
  });

  refreshDynamicUI();
  runMeta.className = "";
  runMeta.textContent = "Ready. Click Run Simulation.";
  updateRunButtonState();

  async function runSimulation() {
    refreshDynamicUI();
    if (hasClientErrors) {
      runMeta.className = "warn";
      runMeta.textContent = "Fix highlighted input errors before running.";
      return;
    }

    persistFormState();
    isRunning = true;
    updateRunButtonState();
    csvBtn.disabled = true;
    runMeta.className = "";
    runMeta.textContent = "Running via Rust API...";

    try {
      const params = buildApiParams();

      const started = performance.now();
      const response = await fetch(`/api/simulate?${params.toString()}`);
      const payload = await response.json();

      if (!response.ok) {
        throw new Error(payload.error || "Simulation failed");
      }

      const ageResults = payload.ageResults || [];
      const mode = payload.mode === "coast" ? "coast" : "retirement";
      const withdrawalPolicy = String(payload.withdrawalPolicy || "guardrails");
      const coastRetirementAge =
        payload.coastRetirementAge === null ||
        payload.coastRetirementAge === undefined
          ? null
          : Number(payload.coastRetirementAge);
      const selected =
        ageResults.find((r) => r.retirementAge === payload.selectedRetirementAge) ||
        null;
      const best =
        ageResults.find((r) => r.retirementAge === payload.bestRetirementAge) ||
        ageResults[0] ||
        null;

      if (!best) {
        throw new Error("No results returned from API");
      }

      lastResults = {
        mode,
        withdrawalPolicy,
        coastRetirementAge,
        successThreshold: Number(payload.successThreshold || 0),
        selectedRetirementAge: payload.selectedRetirementAge,
        bestRetirementAge: payload.bestRetirementAge,
        ageResults
      };

      renderSummary(lastResults, selected, best);
      renderTable(ageResults, mode);
      renderChart(ageResults, lastResults.successThreshold, mode);

      const seconds = (performance.now() - started) / 1000;
      runMeta.className = "";
      runMeta.textContent = `Completed in ${seconds.toFixed(2)}s.`;
      csvBtn.disabled = false;
      persistFormState();
    } catch (error) {
      runMeta.className = "warn";
      runMeta.textContent =
        error instanceof Error ? error.message : "Simulation failed";
    } finally {
      isRunning = false;
      updateRunButtonState();
    }
  }

  function renderSummary(results, selected, best) {
    const thresholdPct = (results.successThreshold * 100).toFixed(1);
    const isCoastMode = results.mode === "coast";
    const chosen = selected || best;
    const targetRetirementAgeText =
      results.coastRetirementAge === null
        ? "model-picked"
        : String(results.coastRetirementAge);
    const resultText = isCoastMode
      ? selected
        ? `Earliest coast age for retiring at ${targetRetirementAgeText}: ${selected.retirementAge}`
        : `No coast age met ${thresholdPct}% for retiring at ${targetRetirementAgeText}; best was ${best.retirementAge}`
      : selected
        ? `Earliest retirement age meeting ${thresholdPct}%: ${selected.retirementAge}`
        : `No retirement age met ${thresholdPct}%; best was ${best.retirementAge}`;

    const cards = [
      ["Analysis Mode", isCoastMode ? "CoastFIRE" : "Retirement Age Sweep"],
      [
        "Withdrawal Strategy",
        humanizeWithdrawalPolicy(results.withdrawalPolicy)
      ],
      ["Result", resultText],
      ["Success Probability", `${(chosen.successRate * 100).toFixed(1)}%`],
      ["Median Pot at Retirement (Today £)", money(chosen.medianRetirementPot)],
      ["P10 Pot at Retirement (Today £)", money(chosen.p10RetirementPot)],
      [
        "Median Retirement ISA / Taxable",
        `${money(chosen.medianRetirementIsa)} / ${money(chosen.medianRetirementTaxable)}`
      ],
      [
        "Median Retirement Pension / Cash",
        `${money(chosen.medianRetirementPension)} / ${money(chosen.medianRetirementCash)}`
      ],
      [
        "P10 Retirement ISA / Taxable",
        `${money(chosen.p10RetirementIsa)} / ${money(chosen.p10RetirementTaxable)}`
      ],
      [
        "P10 Retirement Pension / Cash",
        `${money(chosen.p10RetirementPension)} / ${money(chosen.p10RetirementCash)}`
      ],
      [
        "Median Terminal Total at Horizon (Today £)",
        money(chosen.medianTerminalPot)
      ],
      [
        "P10 Terminal Total at Horizon (Today £)",
        money(chosen.p10TerminalPot)
      ],
      [
        "Median Terminal ISA / Taxable",
        `${money(chosen.medianTerminalIsa)} / ${money(chosen.medianTerminalTaxable)}`
      ],
      [
        "Median Terminal Pension / Cash",
        `${money(chosen.medianTerminalPension)} / ${money(chosen.medianTerminalCash)}`
      ],
      [
        "P10 Minimum Income",
        `${(chosen.p10MinIncomeRatio * 100).toFixed(1)}% of target`
      ],
      [
        "Median Average Income",
        `${(chosen.medianAvgIncomeRatio * 100).toFixed(1)}% of target`
      ]
    ];
    if (isCoastMode) {
      cards.splice(1, 0, ["Coast Target Retirement Age", targetRetirementAgeText]);
    }

    summaryCards.innerHTML = cards
      .map(
        ([title, value]) => `<article class="card"><h3>${title}</h3><p>${value}</p></article>`
      )
      .join("");
  }

  function renderTable(ageResults, mode) {
    if (ageColHeader) {
      ageColHeader.textContent = mode === "coast" ? "Coast Age" : "Retirement Age";
    }

    tableBody.innerHTML = ageResults
      .map(
        (r) => `<tr>
          <td>${r.retirementAge}</td>
          <td>${(r.successRate * 100).toFixed(1)}%</td>
          <td>${money(r.medianRetirementPot)}</td>
          <td>${money(r.p10RetirementPot)}</td>
          <td>${money(r.medianRetirementIsa)}</td>
          <td>${money(r.medianRetirementTaxable)}</td>
          <td>${money(r.medianRetirementPension)}</td>
          <td>${money(r.medianRetirementCash)}</td>
          <td>${money(r.p10RetirementIsa)}</td>
          <td>${money(r.p10RetirementTaxable)}</td>
          <td>${money(r.p10RetirementPension)}</td>
          <td>${money(r.p10RetirementCash)}</td>
          <td>${money(r.medianTerminalPot)}</td>
          <td>${money(r.p10TerminalPot)}</td>
          <td>${money(r.medianTerminalIsa)}</td>
          <td>${money(r.medianTerminalTaxable)}</td>
          <td>${money(r.medianTerminalPension)}</td>
          <td>${money(r.medianTerminalCash)}</td>
          <td>${money(r.p10TerminalIsa)}</td>
          <td>${money(r.p10TerminalTaxable)}</td>
          <td>${money(r.p10TerminalPension)}</td>
          <td>${money(r.p10TerminalCash)}</td>
        </tr>`
      )
      .join("");
  }

  function renderChart(ageResults, threshold, mode) {
    const ctx = chart.getContext("2d");
    if (!ctx || ageResults.length === 0) {
      return;
    }

    if (typeof Chart === "undefined") {
      runMeta.className = "warn";
      runMeta.textContent =
        "Chart.js failed to load. Check your internet connection and refresh.";
      return;
    }

    if (successChart) {
      successChart.destroy();
      successChart = null;
    }

    const labels = ageResults.map((r) => String(r.retirementAge));
    const successRatesPct = ageResults.map((r) =>
      Number((r.successRate * 100).toFixed(2))
    );
    const thresholdPct = Number((threshold * 100).toFixed(2));
    const thresholdLine = labels.map(() => thresholdPct);
    const isCoastMode = mode === "coast";
    const xAxisTitle = isCoastMode ? "Coast Age" : "Retirement Age";
    const tooltipDetails = ageResults.map((r) => [
      `Median retirement total: ${money(r.medianRetirementPot)}`,
      `P10 retirement total: ${money(r.p10RetirementPot)}`,
      `Median terminal total: ${money(r.medianTerminalPot)}`,
      `P10 terminal total: ${money(r.p10TerminalPot)}`,
      `P10 min income: ${(r.p10MinIncomeRatio * 100).toFixed(1)}%`
    ]);

    successChart = new Chart(ctx, {
      type: "line",
      data: {
        labels,
        datasets: [
          {
            label: "Success %",
            data: successRatesPct,
            borderColor: "#1f7a7a",
            pointBackgroundColor: "#204e82",
            pointRadius: 0,
            pointHoverRadius: 4,
            pointHitRadius: 14,
            borderWidth: 2,
            tension: 0.18,
            fill: false
          },
          {
            label: "Threshold %",
            data: thresholdLine,
            borderColor: "#9b3d2d",
            borderDash: [6, 6],
            pointRadius: 0,
            borderWidth: 2,
            tension: 0
          }
        ]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: false,
        normalized: true,
        interaction: {
          mode: "index",
          intersect: false,
          axis: "x"
        },
        scales: {
          x: {
            title: {
              display: true,
              text: xAxisTitle
            },
            grid: {
              color: "#e2ebf2"
            }
          },
          y: {
            min: 0,
            max: 100,
            title: {
              display: true,
              text: "Success Probability (%)"
            },
            ticks: {
              stepSize: 5
            },
            grid: {
              color: "#dbe5ef"
            }
          }
        },
        plugins: {
          legend: {
            display: true
          },
          tooltip: {
            displayColors: false,
            callbacks: {
              title(items) {
                const idx = items[0]?.dataIndex ?? 0;
                return isCoastMode
                  ? `Coast age ${ageResults[idx].retirementAge}`
                  : `Retirement age ${ageResults[idx].retirementAge}`;
              },
              label(context) {
                if (context.datasetIndex === 1) {
                  return `Threshold: ${thresholdPct.toFixed(1)}%`;
                }
                return `Success: ${context.parsed.y.toFixed(1)}%`;
              },
              afterLabel(context) {
                if (context.datasetIndex === 1) {
                  return "";
                }
                const idx = context.dataIndex;
                return tooltipDetails[idx];
              }
            }
          }
        }
      }
    });
  }

  function initializePresetControls() {
    refreshPresetSelect("");

    if (presetSelect && presetNameInput) {
      presetSelect.addEventListener("change", () => {
        if (presetSelect.value) {
          presetNameInput.value = presetSelect.value;
        }
      });
    }
  }

  function initializeQuickPresets() {
    quickPresetButtons.forEach((button) => {
      button.addEventListener("click", () => {
        const key = String(button.dataset.quickPreset || "").trim();
        applyQuickPreset(key);
      });
    });
  }

  function applyQuickPreset(key) {
    const values = QUICK_PRESETS[key];
    if (!values) {
      return;
    }
    applyFormValues(values);
    persistFormState();
    refreshDynamicUI();
    setPresetMeta(`Applied quick preset "${buttonTitle(key)}".`);
  }

  function buttonTitle(key) {
    switch (key) {
      case "conservative":
        return "Conservative";
      case "balanced":
        return "Balanced";
      case "aggressive":
        return "Aggressive";
      case "coastfire":
        return "CoastFIRE Setup";
      default:
        return key;
    }
  }

  function attachAutosaveListeners() {
    form.addEventListener("input", (event) => {
      if (!isPersistableField(event.target)) {
        return;
      }
      scheduleAutosave();
      refreshDynamicUI();
    });

    form.addEventListener("change", (event) => {
      if (!isPersistableField(event.target)) {
        return;
      }
      scheduleAutosave();
      refreshDynamicUI();
    });
  }

  function isPersistableField(target) {
    return Boolean(target && typeof target.name === "string" && target.name.length > 0);
  }

  function scheduleAutosave() {
    if (autosaveTimer !== null) {
      window.clearTimeout(autosaveTimer);
    }
    autosaveTimer = window.setTimeout(() => {
      autosaveTimer = null;
      persistFormState();
    }, AUTOSAVE_DELAY_MS);
  }

  function collectFieldNames() {
    const names = [];
    for (const element of Array.from(form.elements)) {
      if (!element || typeof element.name !== "string" || element.name.length === 0) {
        continue;
      }
      if (!names.includes(element.name)) {
        names.push(element.name);
      }
    }
    return names;
  }

  function buildApiParams() {
    if (currentInputMode() === "basic") {
      return buildBasicApiParams();
    }

    const params = new URLSearchParams();
    for (const [key, value] of new FormData(form).entries()) {
      params.set(String(key), String(value));
    }
    return params;
  }

  function buildBasicApiParams() {
    const params = new URLSearchParams();
    const riskProfile =
      BASIC_RISK_PROFILES[selectedValue("riskProfile")] || BASIC_RISK_PROFILES.balanced;
    const isaLimit = Math.max(parseNumber("isaLimit"), 0);
    const requestedIsaContribution = Math.max(parseNumber("isaContribution"), 0);
    const requestedTaxableContribution = Math.max(parseNumber("taxableContribution"), 0);
    const requestedPensionContribution = Math.max(parseNumber("pensionContribution"), 0);
    const isaContribution = Math.min(requestedIsaContribution, isaLimit);
    const overflowToTaxable = Math.max(requestedIsaContribution - isaContribution, 0);
    const taxableContribution = requestedTaxableContribution + overflowToTaxable;
    const mortgagePayment = Math.max(parseNumber("mortgageAnnualPayment"), 0);
    const mortgageEndAge = selectedValue("mortgageEndAge").trim();
    const taxableStart = Math.max(parseNumber("taxableStart"), 0);

    for (const name of [
      "currentAge",
      "pensionAccessAge",
      "isaStart",
      "taxableStart",
      "pensionStart",
      "targetIncome"
    ]) {
      setFormParam(params, name);
    }

    params.set("analysisMode", "retirement-sweep");
    params.set("withdrawalPolicy", String(riskProfile.withdrawalPolicy));
    params.set("isaLimit", String(isaLimit));
    params.set("isaContribution", String(isaContribution));
    params.set("taxableContribution", String(taxableContribution));
    params.set("pensionContribution", String(requestedPensionContribution));
    params.set("contributionGrowth", "0");
    params.set("taxableBasisStart", String(taxableStart));
    params.set("mortgageAnnualPayment", String(mortgagePayment));
    if (mortgagePayment > 0 && mortgageEndAge !== "") {
      params.set("mortgageEndAge", mortgageEndAge);
    }

    // Keep planning window stable with existing hidden values.
    for (const name of ["maxAge", "horizonAge", "simulations", "seed"]) {
      setFormParam(params, name);
    }

    for (const [key, value] of Object.entries(riskProfile)) {
      if (key === "withdrawalPolicy") {
        continue;
      }
      params.set(key, String(value));
    }

    return params;
  }

  function setFormParam(params, name) {
    const value = selectedValue(name);
    if (value !== "") {
      params.set(name, value);
    }
  }

  function serializeForm() {
    const values = {};
    for (const [key, value] of new FormData(form).entries()) {
      values[String(key)] = String(value);
    }
    return values;
  }

  function persistFormState() {
    const payload = {
      version: STORAGE_SCHEMA_VERSION,
      savedAt: Date.now(),
      values: serializeForm()
    };

    if (!writeStorage(FORM_STATE_KEY, payload)) {
      setPresetMeta("Could not save form inputs in browser storage.", true);
    }
  }

  function restoreFormState() {
    const payload = readStorage(FORM_STATE_KEY);
    if (!isValidVersionedPayload(payload)) {
      return;
    }

    const values = sanitizeFormValues(payload.values);
    applyFormValues(values);
  }

  function savePreset() {
    const name = normalizePresetName((presetNameInput && presetNameInput.value) || "");
    if (!name) {
      setPresetMeta("Enter a preset name first.", true);
      return;
    }

    presetStore.presets[name] = {
      values: serializeForm(),
      savedAt: Date.now()
    };

    if (!persistPresetStore()) {
      return;
    }

    if (presetNameInput) {
      presetNameInput.value = name;
    }
    refreshPresetSelect(name);
    persistFormState();
    setPresetMeta(`Saved preset "${name}".`);
  }

  function loadPreset() {
    const name = getActivePresetName();
    if (!name) {
      setPresetMeta("Select or enter a preset name to load.", true);
      return;
    }

    const preset = presetStore.presets[name];
    if (!preset || typeof preset !== "object") {
      setPresetMeta(`Preset "${name}" was not found.`, true);
      return;
    }

    applyFormValues(sanitizeFormValues(preset.values));
    persistFormState();
    if (presetSelect) {
      presetSelect.value = name;
    }
    if (presetNameInput) {
      presetNameInput.value = name;
    }
    refreshDynamicUI();
    setPresetMeta(`Loaded preset "${name}".`);
  }

  function deletePreset() {
    const name = getActivePresetName();
    if (!name) {
      setPresetMeta("Select or enter a preset name to delete.", true);
      return;
    }

    if (!Object.prototype.hasOwnProperty.call(presetStore.presets, name)) {
      setPresetMeta(`Preset "${name}" was not found.`, true);
      return;
    }

    delete presetStore.presets[name];
    if (!persistPresetStore()) {
      return;
    }

    refreshPresetSelect("");
    if (presetNameInput && presetNameInput.value === name) {
      presetNameInput.value = "";
    }
    setPresetMeta(`Deleted preset "${name}".`);
  }

  function resetToDefaults() {
    applyFormValues(defaultFormValues);
    persistFormState();
    refreshDynamicUI();
    setPresetMeta("Reset form to default values.");
  }

  function getActivePresetName() {
    const selected = normalizePresetName((presetSelect && presetSelect.value) || "");
    if (selected && Object.prototype.hasOwnProperty.call(presetStore.presets, selected)) {
      return selected;
    }

    const typed = normalizePresetName((presetNameInput && presetNameInput.value) || "");
    if (typed && Object.prototype.hasOwnProperty.call(presetStore.presets, typed)) {
      return typed;
    }

    return "";
  }

  function normalizePresetName(value) {
    return String(value || "").trim();
  }

  function refreshPresetSelect(selectedName) {
    if (!presetSelect) {
      return;
    }

    const names = Object.keys(presetStore.presets).sort((a, b) => a.localeCompare(b));
    while (presetSelect.options.length > 0) {
      presetSelect.remove(0);
    }

    presetSelect.add(new Option("Select preset...", ""));
    for (const name of names) {
      presetSelect.add(new Option(name, name));
    }

    const desired = normalizePresetName(selectedName);
    if (desired && names.includes(desired)) {
      presetSelect.value = desired;
    } else {
      presetSelect.value = "";
    }
  }

  function loadPresetStore() {
    const payload = readStorage(PRESET_STORE_KEY);
    if (!isValidVersionedPayload(payload) || !isObject(payload.presets)) {
      return {
        version: STORAGE_SCHEMA_VERSION,
        presets: {}
      };
    }

    const sanitizedPresets = {};
    for (const [name, preset] of Object.entries(payload.presets)) {
      const cleanName = normalizePresetName(name);
      if (!cleanName || !isObject(preset)) {
        continue;
      }

      const values = sanitizeFormValues(preset.values);
      if (Object.keys(values).length === 0) {
        continue;
      }

      sanitizedPresets[cleanName] = {
        values,
        savedAt: Number(preset.savedAt || 0)
      };
    }

    return {
      version: STORAGE_SCHEMA_VERSION,
      presets: sanitizedPresets
    };
  }

  function persistPresetStore() {
    const payload = {
      version: STORAGE_SCHEMA_VERSION,
      presets: presetStore.presets
    };

    if (!writeStorage(PRESET_STORE_KEY, payload)) {
      setPresetMeta("Could not save presets in browser storage.", true);
      return false;
    }

    return true;
  }

  function applyFormValues(values) {
    for (const [name, value] of Object.entries(values || {})) {
      if (!fieldNames.includes(name)) {
        continue;
      }

      const field = form.elements.namedItem(name);
      if (!field) {
        continue;
      }

      if (field instanceof RadioNodeList) {
        field.value = String(value);
        continue;
      }

      const next = String(value);
      if (field instanceof HTMLSelectElement) {
        const hasOption = Array.from(field.options).some((opt) => opt.value === next);
        if (!hasOption) {
          continue;
        }
      }

      field.value = next;
    }
  }

  function sanitizeFormValues(rawValues) {
    const values = {};
    if (!isObject(rawValues)) {
      return values;
    }

    for (const name of fieldNames) {
      if (!Object.prototype.hasOwnProperty.call(rawValues, name)) {
        continue;
      }
      const value = rawValues[name];
      if (value === null || value === undefined) {
        continue;
      }
      values[name] = String(value);
    }

    return values;
  }

  function isValidVersionedPayload(payload) {
    return Boolean(
      isObject(payload) &&
        payload.version === STORAGE_SCHEMA_VERSION &&
        isObject(payload.values || payload.presets)
    );
  }

  function restoreInputMode() {
    if (!inputModeSelect) {
      return;
    }
    let mode = "basic";
    try {
      const raw = window.localStorage.getItem(INPUT_MODE_KEY);
      if (raw === "basic" || raw === "advanced") {
        mode = raw;
      }
    } catch (_) {}
    inputModeSelect.value = mode;
  }

  function persistInputMode() {
    if (!inputModeSelect) {
      return;
    }
    try {
      window.localStorage.setItem(INPUT_MODE_KEY, inputModeSelect.value);
    } catch (_) {}
  }

  function currentInputMode() {
    if (!inputModeSelect) {
      return "advanced";
    }
    return inputModeSelect.value === "advanced" ? "advanced" : "basic";
  }

  function refreshDynamicUI() {
    normalizeDerivedValuesForMode();
    applyConditionalVisibility();
    updateValueHints();
    updateLiveSummary();
    validateInline();
  }

  function normalizeDerivedValuesForMode() {
    if (currentInputMode() !== "basic") {
      return;
    }

    const taxableStart = parseNumber("taxableStart");
    const basisField = form.elements.namedItem("taxableBasisStart");
    if (!basisField || basisField instanceof RadioNodeList) {
      return;
    }

    const basisValue = Number(basisField.value);
    if (!Number.isFinite(basisValue)) {
      basisField.value = String(Math.max(0, taxableStart));
      return;
    }

    const clamped = Math.max(0, Math.min(basisValue, taxableStart));
    if (Math.abs(clamped - basisValue) > 1e-9) {
      basisField.value = String(clamped);
    }

  }

  function applyConditionalVisibility() {
    const mode = currentInputMode();
    const values = serializeForm();
    form.classList.toggle("basic-mode", mode === "basic");

    const labels = Array.from(form.querySelectorAll("label"));
    labels.forEach((label) => {
      const control = label.querySelector("input[name], select[name], textarea[name]");
      let hidden = false;

      if (label.classList.contains("advanced-only") && mode !== "advanced") {
        hidden = true;
      }
      if (label.classList.contains("basic-only") && mode !== "basic") {
        hidden = true;
      }
      if (!hidden && mode === "basic" && control) {
        hidden = !BASIC_VISIBLE_FIELDS.has(control.name);
      }
      if (!hidden && label.hasAttribute("data-show-when")) {
        const expr = String(label.getAttribute("data-show-when") || "");
        hidden = !evaluateShowWhen(expr, values);
      }

      label.classList.toggle("is-hidden", hidden);
    });

    // Handle non-label conditional elements if any are added later.
    document.querySelectorAll("[data-show-when]").forEach((el) => {
      if (el.tagName.toLowerCase() === "label") {
        return;
      }
      const expr = String(el.getAttribute("data-show-when") || "");
      const show = evaluateShowWhen(expr, values);
      el.classList.toggle("is-hidden", !show);
    });

    document.querySelectorAll(".quick-presets").forEach((el) => {
      el.classList.toggle("is-hidden", mode === "basic");
    });

    document.querySelectorAll(".config-section").forEach((section) => {
      const visibleControls = Array.from(section.querySelectorAll("label")).filter(
        (label) => !label.classList.contains("is-hidden")
      );
      const hasVisibleControls = visibleControls.length > 0;
      section.classList.toggle("is-hidden", !hasVisibleControls);
      if (!hasVisibleControls) {
        section.removeAttribute("open");
        return;
      }

      if (mode === "basic") {
        section.setAttribute("open", "");
        return;
      }

      const level = String(section.getAttribute("data-section-level") || "basic");
      if (level === "advanced") {
        section.setAttribute("open", "");
      }
    });
  }

  function evaluateShowWhen(expr, values) {
    if (!expr) {
      return true;
    }
    const clauses = expr
      .split(";")
      .map((token) => token.trim())
      .filter(Boolean);
    if (clauses.length === 0) {
      return true;
    }

    return clauses.every((clause) => {
      const parts = clause.split("=");
      if (parts.length !== 2) {
        return true;
      }
      const field = parts[0].trim();
      const accepted = parts[1]
        .split("|")
        .map((entry) => entry.trim())
        .filter(Boolean);
      if (!field || accepted.length === 0) {
        return true;
      }
      const actual = String(values[field] || "");
      return accepted.includes(actual);
    });
  }

  function initValueHintPlaceholders() {
    const labels = form.querySelectorAll("label");
    labels.forEach((label) => {
      const control = label.querySelector("input[type='number']");
      if (!control || label.querySelector(".value-hint")) {
        return;
      }
      const hint = document.createElement("span");
      hint.className = "value-hint";
      label.appendChild(hint);
    });
  }

  function updateValueHints() {
    const labels = form.querySelectorAll("label");
    labels.forEach((label) => {
      const control = label.querySelector("input[type='number']");
      const hint = label.querySelector(".value-hint");
      if (!control || !hint || !control.name) {
        return;
      }
      hint.textContent = formatValueHint(control.name, control.value);
    });
  }

  function formatValueHint(name, rawValue) {
    const value = Number(rawValue);
    if (rawValue === "" || !Number.isFinite(value)) {
      return "";
    }

    if (CURRENCY_FIELDS.has(name)) {
      return `≈ ${money(value)}`;
    }
    if (PERCENT_FIELDS.has(name)) {
      return `≈ ${trimZeros(value)}%`;
    }
    if (AGE_FIELDS.has(name)) {
      return `${Math.round(value)} years`;
    }
    if (COUNT_FIELDS.has(name)) {
      return `${Math.round(value).toLocaleString()} samples`;
    }
    if (YEAR_FIELDS.has(name)) {
      return `${trimZeros(value)} years`;
    }
    if (RATIO_FIELDS.has(name)) {
      return trimZeros(value, 2);
    }
    return trimZeros(value);
  }

  function updateLiveSummary() {
    const mode = currentInputMode();
    const currentAge = parseNumber("currentAge");
    const maxAge = parseNumber("maxAge");
    const horizonAge = parseNumber("horizonAge");
    const simulations = parseNumber("simulations");

    const isaStart = parseNumber("isaStart");
    const taxableStart = parseNumber("taxableStart");
    const pensionStart = parseNumber("pensionStart");
    const cashStart = parseNumber("cashStart");
    const startTotal = isaStart + taxableStart + pensionStart + cashStart;

    const isaLimit = Math.max(parseNumber("isaLimit"), 0);
    const isaContribution = parseNumber("isaContribution");
    const taxableContribution = parseNumber("taxableContribution");
    const pensionContribution = parseNumber("pensionContribution");
    const isaEffective = Math.min(Math.max(isaContribution, 0), isaLimit);
    const isaOverflowFromDetailed = Math.max(isaContribution - isaEffective, 0);
    const annualContribution =
      isaEffective +
      Math.max(taxableContribution, 0) +
      isaOverflowFromDetailed +
      Math.max(pensionContribution, 0);
    const isaOverflow = isaOverflowFromDetailed;

    const targetIncome = parseNumber("targetIncome");
    const mortgageAnnualPayment = parseNumber("mortgageAnnualPayment");
    const mortgageEndAgeRaw = selectedValue("mortgageEndAge");
    const mortgageEndAge =
      mortgageEndAgeRaw.trim() === "" ? null : Number(mortgageEndAgeRaw);
    const analysisMode =
      mode === "basic" ? "Retirement Age Sweep" : selectedOptionText("analysisMode");
    const strategy =
      mode === "basic"
        ? `${selectedOptionText("riskProfile")} profile`
        : selectedOptionText("withdrawalPolicy");
    const mortgageSummary =
      mortgageAnnualPayment <= 0
        ? "None"
        : mortgageEndAge !== null && Number.isFinite(mortgageEndAge)
          ? `${money(mortgageAnnualPayment)}/yr until age ${Math.round(mortgageEndAge)}`
          : `${money(mortgageAnnualPayment)}/yr (end age not set)`;

    setSummaryText("startTotal", money(startTotal));
    setSummaryText("annualContribution", money(annualContribution));
    setSummaryText("isaOverflow", money(isaOverflow));
    setSummaryText("targetIncome", money(targetIncome));
    setSummaryText("mortgage", mortgageSummary);
    setSummaryText("modeStrategy", `${analysisMode} / ${strategy}`);
    setSummaryText(
      "retirementWindow",
      `${Math.round(currentAge)}-${Math.round(maxAge)} (to ${Math.round(horizonAge)})`
    );
    setSummaryText("simScale", `${Math.round(simulations).toLocaleString()} per age`);
  }

  function setSummaryText(key, value) {
    const node = liveSummaryEls[key];
    if (!node) {
      return;
    }
    node.textContent = value;
  }

  function validateInline() {
    const messages = [];
    const errors = [];
    const mode = currentInputMode();

    const currentAge = parseNumber("currentAge");
    const pensionAccessAge = parseNumber("pensionAccessAge");
    const maxAge = parseNumber("maxAge");
    const horizonAge = parseNumber("horizonAge");
    const taxableStart = parseNumber("taxableStart");
    const taxableBasisStart = parseNumber("taxableBasisStart");
    const minFloor = parseNumber("minFloor");
    const maxCeiling = parseNumber("maxCeiling");
    const gkLower = parseNumber("gkLowerGuardrail");
    const gkUpper = parseNumber("gkUpperGuardrail");
    const targetIncome = parseNumber("targetIncome");
    const mortgageAnnualPayment = parseNumber("mortgageAnnualPayment");
    const mortgageEndAgeRaw = selectedValue("mortgageEndAge");
    const mortgageEndAge =
      mortgageEndAgeRaw.trim() === "" ? null : Number(mortgageEndAgeRaw);
    const simulations = parseNumber("simulations");
    const isaContribution = parseNumber("isaContribution");
    const isaLimit = parseNumber("isaLimit");
    const analysisMode = mode === "basic" ? "retirement-sweep" : selectedValue("analysisMode");
    const coastAgeRaw = selectedValue("coastRetirementAge");
    const coastAge = coastAgeRaw === "" ? null : Number(coastAgeRaw);
    const strategy = mode === "basic" ? "guardrails" : selectedValue("withdrawalPolicy");

    if (maxAge < currentAge) {
      errors.push("Max retirement age must be greater than or equal to current age.");
    }
    if (horizonAge <= maxAge) {
      errors.push("Horizon age must be greater than max retirement age.");
    }
    if (pensionAccessAge < currentAge) {
      errors.push("Pension access age must be greater than or equal to current age.");
    }
    if (targetIncome <= 0) {
      errors.push("Target income must be greater than zero.");
    }
    if (mortgageAnnualPayment < 0) {
      errors.push("Mortgage payment must be zero or positive.");
    }
    if (mortgageAnnualPayment > 0) {
      if (mortgageEndAgeRaw.trim() === "") {
        errors.push("Mortgage end age is required when mortgage payment is greater than zero.");
      } else if (!Number.isFinite(mortgageEndAge) || mortgageEndAge <= currentAge) {
        errors.push("Mortgage end age must be greater than current age.");
      }
    }
    if (taxableBasisStart > taxableStart && mode === "advanced") {
      errors.push("Taxable cost basis cannot exceed taxable starting value.");
    }
    if (minFloor > maxCeiling) {
      errors.push("Min income floor cannot be above max income ceiling.");
    }
    if (strategy === "guyton-klinger" && gkLower > gkUpper) {
      errors.push("GK lower guardrail must be less than or equal to upper guardrail.");
    }
    if (mode !== "basic" && analysisMode === "coast-fire" && coastAge !== null) {
      if (coastAge < currentAge) {
        errors.push("Coast retirement age must be at least current age.");
      }
      if (coastAge >= horizonAge) {
        errors.push("Coast retirement age must be below horizon age.");
      }
    }

    if (isaContribution > isaLimit) {
      messages.push({
        level: "info",
        text: "ISA contribution above ISA limit: overflow will automatically route into taxable."
      });
    }
    if (simulations < 1000) {
      messages.push({
        level: "warn",
        text: "Low simulation count may produce noisy results; consider 1,000+."
      });
    }
    if (simulations > 25000) {
      messages.push({
        level: "warn",
        text: "Very high simulation count may slow down browser responsiveness."
      });
    }
    if (
      mortgageAnnualPayment > 0 &&
      mortgageEndAge !== null &&
      Number.isFinite(mortgageEndAge) &&
      mortgageEndAge >= horizonAge
    ) {
      messages.push({
        level: "info",
        text: "Mortgage does not end before horizon age, so no mortgage drop occurs in the simulation window."
      });
    }

    errors.forEach((text) => {
      messages.unshift({ level: "error", text });
    });

    if (messages.length === 0) {
      messages.push({
        level: "ok",
        text: "Inputs look valid. Run simulation when ready."
      });
    }

    if (inlineValidation) {
      inlineValidation.innerHTML = messages
        .map(
          (message) =>
            `<div class="validation-item ${message.level}">${message.text}</div>`
        )
        .join("");
    }

    hasClientErrors = errors.length > 0;
    updateRunButtonState();
  }

  function updateRunButtonState() {
    runBtn.disabled = isRunning || hasClientErrors;
  }

  function selectedValue(name) {
    const field = form.elements.namedItem(name);
    if (!field) {
      return "";
    }
    if (field instanceof RadioNodeList) {
      return String(field.value || "");
    }
    return String(field.value || "");
  }

  function selectedOptionText(name) {
    const field = form.elements.namedItem(name);
    if (!field) {
      return "";
    }
    if (field instanceof HTMLSelectElement) {
      return field.options[field.selectedIndex]?.textContent || "";
    }
    return String(field.value || "");
  }

  function parseNumber(name) {
    const raw = selectedValue(name);
    const value = Number(raw);
    if (!Number.isFinite(value)) {
      return 0;
    }
    return value;
  }

  function trimZeros(value, decimals = 1) {
    const text = Number(value).toFixed(decimals);
    return text.replace(/\.0+$/, "").replace(/(\.\d*?)0+$/, "$1");
  }

  function money(value) {
    return gbpFormatter.format(Number(value || 0));
  }

  function humanizeWithdrawalPolicy(value) {
    switch (String(value || "")) {
      case "guyton-klinger":
        return "Guyton-Klinger";
      case "vpw":
        return "VPW";
      case "floor-upside":
        return "Floor + Upside";
      case "bucket":
        return "Bucket";
      case "guardrails":
      default:
        return "Dynamic Guardrails";
    }
  }

  function isObject(value) {
    return Boolean(value && typeof value === "object");
  }

  function readStorage(key) {
    try {
      const raw = window.localStorage.getItem(key);
      if (!raw) {
        return null;
      }
      return JSON.parse(raw);
    } catch (_) {
      return null;
    }
  }

  function writeStorage(key, payload) {
    try {
      window.localStorage.setItem(key, JSON.stringify(payload));
      return true;
    } catch (_) {
      return false;
    }
  }

  function setPresetMeta(message, isWarn = false) {
    if (!presetMeta) {
      return;
    }
    presetMeta.textContent = message;
    presetMeta.className = isWarn ? "warn" : "";
  }

  function initFieldTooltips() {
    const labels = form.querySelectorAll("label");
    labels.forEach((label) => {
      const control = label.querySelector("input, select, textarea");
      if (!control) {
        return;
      }
      const tip = control.getAttribute("title");
      if (!tip) {
        return;
      }
      label.dataset.tooltip = tip;
      control.removeAttribute("title");

      if (!label.querySelector(".help-icon")) {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "help-icon";
        button.textContent = "?";
        button.setAttribute("aria-label", "Show input help");
        button.addEventListener("click", (event) => {
          event.preventDefault();
          event.stopPropagation();
          const willOpen = !label.classList.contains("tooltip-open");
          closeAllTooltips(label);
          if (willOpen) {
            label.classList.add("tooltip-open");
          }
        });
        label.appendChild(button);
      }
    });

    document.addEventListener("click", (event) => {
      const target = event.target;
      if (!(target instanceof Element) || !target.closest("label[data-tooltip]")) {
        closeAllTooltips(null);
      }
    });
  }

  function closeAllTooltips(exceptLabel) {
    const labels = form.querySelectorAll("label[data-tooltip]");
    labels.forEach((label) => {
      if (exceptLabel && label === exceptLabel) {
        return;
      }
      label.classList.remove("tooltip-open");
    });
  }
})();
