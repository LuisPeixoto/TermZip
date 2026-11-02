//! A bench mode for measuring the app the same way every time (see `tools/measure-app.sh` and the performance
//! guide). It does nothing unless `TERMZIP_BENCH` names a file.
//!
//! The webview reports milestones (`mounted`, `painted`, `prompt`, `flood_start`, `done`) and this records how long
//! after the process started each one happened. When `done` arrives the numbers are written to the file, and with
//! `TERMZIP_BENCH_EXIT=1` the app quits, so a script can run it many times.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use serde_json::{Value, json};

/// The milestones the webview may report, in order. Anything else is refused.
const MARKS: [&str; 6] = [
    "mounted",
    "painted",
    "prompt",
    "flood_start",
    "done",
    "failed",
];
const DEFAULT_LINES: u64 = 1_000_000;
const MAX_LINES: u64 = 50_000_000;

pub const FILE_ENV: &str = "TERMZIP_BENCH";
pub const EXIT_ENV: &str = "TERMZIP_BENCH_EXIT";
pub const LINES_ENV: &str = "TERMZIP_BENCH_LINES";

/// What the webview does in bench mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchPlan {
    /// How many lines the flood command prints (`seq 1 N`).
    pub lines: u64,
}

pub struct Bench {
    file: PathBuf,
    exit: bool,
    lines: u64,
    started: Instant,
    marks: Mutex<BTreeMap<String, f64>>,
}

/// Managed by Tauri; `None` is a normal run.
pub struct BenchState(pub Option<Bench>);

impl BenchState {
    /// Reads the environment. Call it first thing, so the clock starts with the process.
    pub fn from_env() -> Self {
        let started = Instant::now();
        let file = std::env::var_os(FILE_ENV)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        let exit = std::env::var(EXIT_ENV).is_ok_and(|value| value == "1");
        let lines = std::env::var(LINES_ENV)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map_or(DEFAULT_LINES, |lines| lines.clamp(1, MAX_LINES));
        Self(file.map(|file| Bench {
            file,
            exit,
            lines,
            started,
            marks: Mutex::new(BTreeMap::new()),
        }))
    }
}

impl Bench {
    fn plan(&self) -> BenchPlan {
        BenchPlan { lines: self.lines }
    }

    /// Notes a milestone; returns the report when it was the last one.
    fn mark(&self, name: &str) -> Result<Option<Value>, String> {
        if !MARKS.contains(&name) {
            return Err(format!("“{name}” is not a bench milestone."));
        }
        let at = self.started.elapsed().as_secs_f64() * 1000.0;
        let mut marks = self
            .marks
            .lock()
            .map_err(|_| "bench lock poisoned".to_owned())?;
        marks.entry(name.to_owned()).or_insert(at);
        Ok((name == "done" || name == "failed").then(|| report(&marks, self.lines)))
    }
}

/// The numbers a run produced, as JSON: every milestone in milliseconds since the process started, and what can
/// be derived from them.
fn report(marks: &BTreeMap<String, f64>, lines: u64) -> Value {
    let get = |name: &str| marks.get(name).copied();
    let flood_ms = get("done")
        .zip(get("flood_start"))
        .map(|(done, start)| done - start);
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "lines": lines,
        "failed": marks.contains_key("failed"),
        "marksMs": marks,
        "launchToPromptMs": get("prompt"),
        "floodMs": flood_ms,
        "floodLinesPerSecond": flood_ms.filter(|ms| *ms > 0.0).map(|ms| lines as f64 / (ms / 1000.0)),
    })
}

/// `Some` in bench mode: what the webview should do. A normal run gets `None` and does nothing.
#[tauri::command]
pub fn bench_plan(state: tauri::State<'_, BenchState>) -> Option<BenchPlan> {
    state.0.as_ref().map(Bench::plan)
}

/// A milestone reached by the webview. The last one writes the report (and quits when asked to).
#[tauri::command]
pub fn bench_mark(
    app: tauri::AppHandle,
    state: tauri::State<'_, BenchState>,
    name: String,
) -> Result<(), String> {
    let Some(bench) = state.0.as_ref() else {
        return Ok(());
    };
    if let Some(report) = bench.mark(&name)? {
        let text = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;
        termzip_core::write_atomic(&bench.file, text.as_bytes())
            .map_err(|error| format!("Could not write the bench report: {error}"))?;
        if bench.exit {
            app.exit(0);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bench() -> Bench {
        Bench {
            file: PathBuf::from("unused.json"),
            exit: false,
            lines: 1_000_000,
            started: Instant::now(),
            marks: Mutex::new(BTreeMap::new()),
        }
    }

    #[test]
    fn milestones_are_recorded_and_the_last_one_gives_the_report() {
        let bench = bench();
        for name in ["mounted", "painted", "prompt", "flood_start"] {
            assert_eq!(bench.mark(name).unwrap(), None, "{name}");
        }

        let report = bench.mark("done").unwrap().expect("done ends the run");

        assert_eq!(report["lines"], 1_000_000);
        assert_eq!(report["failed"], false);
        assert!(report["marksMs"]["prompt"].as_f64().unwrap() >= 0.0);
        assert!(report["floodMs"].as_f64().unwrap() >= 0.0);
        assert!(report["launchToPromptMs"].is_number());
    }

    #[test]
    fn a_milestone_that_is_not_known_is_refused() {
        assert!(bench().mark("rm -rf /").is_err());
        assert!(bench().mark("").is_err());
    }

    #[test]
    fn the_first_report_of_a_milestone_wins() {
        let bench = bench();
        bench.mark("prompt").unwrap();
        let first = bench.marks.lock().unwrap()["prompt"];
        std::thread::sleep(std::time::Duration::from_millis(5));
        bench.mark("prompt").unwrap();

        assert_eq!(bench.marks.lock().unwrap()["prompt"], first);
    }

    #[test]
    fn a_failed_run_still_gives_a_report_that_says_so() {
        let report = bench().mark("failed").unwrap().unwrap();

        assert_eq!(report["failed"], true);
        assert!(report["floodMs"].is_null());
    }

    #[test]
    fn the_flood_rate_is_lines_over_seconds() {
        let marks = BTreeMap::from([
            ("flood_start".to_owned(), 1000.0),
            ("done".to_owned(), 3000.0),
        ]);

        let report = report(&marks, 1_000_000);

        assert_eq!(report["floodMs"], 2000.0);
        assert_eq!(report["floodLinesPerSecond"], 500_000.0);
    }

    #[test]
    fn without_the_variable_there_is_no_bench() {
        // Tests share the process environment, so only the absent case is checked here.
        if std::env::var_os(FILE_ENV).is_none() {
            assert!(BenchState::from_env().0.is_none());
        }
    }
}
