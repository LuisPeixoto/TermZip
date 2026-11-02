//! The performance budgets: a metric, a limit, and the check that a measurement is not far past it.
//!
//! A budget is the number TermZip is expected to meet on the reference hardware (see `docs/performance.md`). A
//! shared CI runner is not that hardware and is noisy, so the check only fails when a measurement is more than
//! [`SLACK`] times worse than the budget: it catches a regression, not a slow afternoon.

use std::collections::BTreeMap;

use serde_json::Value;

/// How many times worse than its budget a measurement may be before the check fails.
pub const SLACK: f64 = 2.0;

/// One limit: a metric that must be at least `min` (a rate) or at most `max` (a time or a size).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budget {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// The outcome for one metric.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub name: String,
    pub value: f64,
    pub budget: Budget,
    /// Within the budget itself.
    pub met: bool,
    /// Within the budget with the slack: this is what fails the check.
    pub acceptable: bool,
}

/// Reads `docs/performance-budgets.toml`: `[metrics.name]` tables with `min` or `max`.
pub fn parse_budgets(text: &str) -> Result<BTreeMap<String, Budget>, String> {
    let table: toml::Table = toml::from_str(text).map_err(|error| error.to_string())?;
    let metrics = table
        .get("metrics")
        .and_then(toml::Value::as_table)
        .ok_or("there is no [metrics] table")?;
    let number = |value: Option<&toml::Value>| {
        value.and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
    };
    let mut budgets = BTreeMap::new();
    for (name, entry) in metrics {
        let entry = entry
            .as_table()
            .ok_or_else(|| format!("{name} is not a table"))?;
        let budget = Budget {
            min: number(entry.get("min")),
            max: number(entry.get("max")),
        };
        if budget.min.is_none() == budget.max.is_none() {
            return Err(format!("{name} needs exactly one of min and max"));
        }
        budgets.insert(name.clone(), budget);
    }
    Ok(budgets)
}

/// Compares measurements with budgets. A metric with no measurement (the PTY on Windows, say) is left out.
pub fn check(budgets: &BTreeMap<String, Budget>, metrics: &BTreeMap<String, f64>) -> Vec<Verdict> {
    budgets
        .iter()
        .filter_map(|(name, budget)| {
            let value = *metrics.get(name)?;
            let met = budget.min.is_none_or(|min| value >= min)
                && budget.max.is_none_or(|max| value <= max);
            let acceptable = budget.min.is_none_or(|min| value >= min / SLACK)
                && budget.max.is_none_or(|max| value <= max * SLACK);
            Some(Verdict {
                name: name.clone(),
                value,
                budget: *budget,
                met,
                acceptable,
            })
        })
        .collect()
}

/// The numbers of an app bench report (`TERMZIP_BENCH`) under the names the budgets use.
pub fn flatten_app_report(report: &Value) -> BTreeMap<String, f64> {
    let mut metrics = BTreeMap::new();
    let pairs = [
        ("launchToPromptMs", "app_launch_to_prompt_ms"),
        ("floodLinesPerSecond", "app_flood_lines_per_s"),
        ("idleCpuPercent", "app_idle_cpu_percent"),
        ("idleMemoryMb", "app_idle_memory_mb"),
        ("floodMemoryMb", "app_flood_memory_mb"),
    ];
    for (key, name) in pairs {
        if let Some(value) = report.get(key).and_then(Value::as_f64) {
            metrics.insert(name.to_owned(), value);
        }
    }
    metrics
}

/// The measurements of a results file: `{"metrics": {...}}` from this crate, or an app report.
pub fn read_results(text: &str) -> Result<BTreeMap<String, f64>, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    if let Some(metrics) = value.get("metrics").and_then(Value::as_object) {
        return Ok(metrics
            .iter()
            .filter_map(|(name, value)| Some((name.clone(), value.as_f64()?)))
            .collect());
    }
    Ok(flatten_app_report(&value))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUDGETS: &str = "
[metrics.rate]
min = 100

[metrics.time]
max = 50.5
";

    fn metrics(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), *value))
            .collect()
    }

    #[test]
    fn budgets_are_read() {
        let budgets = parse_budgets(BUDGETS).unwrap();

        assert_eq!(
            budgets["rate"],
            Budget {
                min: Some(100.0),
                max: None
            }
        );
        assert_eq!(
            budgets["time"],
            Budget {
                min: None,
                max: Some(50.5)
            }
        );
    }

    #[test]
    fn a_budget_needs_exactly_one_limit() {
        assert!(parse_budgets("[metrics.a]\nmin = 1\nmax = 2\n").is_err());
        assert!(parse_budgets("[metrics.a]\n").is_err());
        assert!(parse_budgets("nothing = 1\n").is_err());
        assert!(parse_budgets("not toml [").is_err());
    }

    #[test]
    fn a_measurement_within_the_budget_is_met() {
        let verdicts = check(
            &parse_budgets(BUDGETS).unwrap(),
            &metrics(&[("rate", 150.0), ("time", 40.0)]),
        );

        assert!(
            verdicts
                .iter()
                .all(|verdict| verdict.met && verdict.acceptable)
        );
    }

    #[test]
    fn a_measurement_a_little_past_the_budget_passes_with_the_slack() {
        let verdicts = check(
            &parse_budgets(BUDGETS).unwrap(),
            &metrics(&[("rate", 70.0), ("time", 80.0)]),
        );

        for verdict in &verdicts {
            assert!(!verdict.met, "{}", verdict.name);
            assert!(verdict.acceptable, "{}", verdict.name);
        }
    }

    #[test]
    fn a_measurement_past_the_slack_fails() {
        let verdicts = check(
            &parse_budgets(BUDGETS).unwrap(),
            &metrics(&[("rate", 49.0), ("time", 102.0)]),
        );

        assert!(verdicts.iter().all(|verdict| !verdict.acceptable));
    }

    #[test]
    fn exactly_at_the_edge_of_the_slack_passes() {
        let verdicts = check(
            &parse_budgets(BUDGETS).unwrap(),
            &metrics(&[("rate", 50.0), ("time", 101.0)]),
        );

        assert!(verdicts.iter().all(|verdict| verdict.acceptable));
    }

    #[test]
    fn a_metric_that_was_not_measured_is_left_out() {
        let verdicts = check(
            &parse_budgets(BUDGETS).unwrap(),
            &metrics(&[("rate", 150.0)]),
        );

        assert_eq!(verdicts.len(), 1);
        assert_eq!(verdicts[0].name, "rate");
    }

    #[test]
    fn an_app_report_is_flattened_to_the_names_of_the_budgets() {
        let report = serde_json::json!({"launchToPromptMs": 900.5, "floodLinesPerSecond": 1.2e6, "failed": false, "lines": 5});

        let flat = flatten_app_report(&report);

        assert_eq!(flat["app_launch_to_prompt_ms"], 900.5);
        assert_eq!(flat["app_flood_lines_per_s"], 1.2e6);
        assert!(!flat.contains_key("lines"));
    }

    #[test]
    fn results_are_read_from_either_kind_of_file() {
        let ours = read_results(r#"{"metrics": {"a": 1.5, "b": "no"}}"#).unwrap();
        let app = read_results(r#"{"launchToPromptMs": 10.0}"#).unwrap();

        assert_eq!(ours, metrics(&[("a", 1.5)]));
        assert_eq!(app["app_launch_to_prompt_ms"], 10.0);
        assert!(read_results("nope").is_err());
    }
}
