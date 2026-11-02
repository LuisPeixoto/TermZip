//! `termzip-bench run [--json file]` measures the parts of TermZip that handle a lot of data, and
//! `termzip-bench check <budgets.toml> <results.json>...` compares measurements with the budgets.
//! Build it with `--release`; a debug build says nothing about speed.

use std::collections::BTreeMap;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use serde_json::json;
use termzip_bench::{SLACK, check, parse_budgets, read_results};

/// Runs `work` this many times and keeps the median, so one slow run does not decide.
const RUNS: usize = 5;

fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

fn time(mut work: impl FnMut()) -> Duration {
    median(
        (0..RUNS)
            .map(|_| {
                let start = Instant::now();
                work();
                start.elapsed()
            })
            .collect(),
    )
}

/// Microseconds for one call of `work`, over `iterations` calls.
fn micros_per_call(iterations: u32, mut work: impl FnMut()) -> f64 {
    time(|| (0..iterations).for_each(|_| work())).as_secs_f64() * 1e6 / f64::from(iterations)
}

fn mb_per_second(bytes: usize, took: Duration) -> f64 {
    bytes as f64 / (1024.0 * 1024.0) / took.as_secs_f64()
}

/// Output the way a shell with integration prints it: commands of about 4 KiB, each between its markers.
fn terminal_stream(commands: usize) -> Vec<u8> {
    let mut stream = Vec::new();
    for number in 0..commands {
        stream.extend_from_slice(
            b"\x1b]133;A\x07$ \x1b]133;B\x07\x1b]1337;CommandB64=bHMgLWxh\x07\x1b]133;C\x07",
        );
        for line in 0..60 {
            stream.extend_from_slice(
                format!("line {number}-{line} of the output of a command\r\n").as_bytes(),
            );
        }
        stream.extend_from_slice(b"\x1b]133;D;0\x07\x1b]1337;CurrentDir=/work/project\x07");
    }
    stream
}

fn osc_parser() -> f64 {
    let stream = terminal_stream(20_000);
    let took = time(|| {
        let mut parser = termzip_shell::OscParser::new();
        for chunk in stream.chunks(4096) {
            std::hint::black_box(parser.feed_ranges(chunk));
        }
    });
    mb_per_second(stream.len(), took)
}

fn block_tracker() -> f64 {
    let stream = terminal_stream(20_000);
    let took = time(|| {
        let mut parser = termzip_shell::OscParser::new();
        let mut tracker = termzip_shell::BlockTracker::new();
        for chunk in stream.chunks(4096) {
            let mut cursor = 0;
            for hit in parser.feed_ranges(chunk) {
                if hit.start > cursor {
                    tracker.on_region_bytes(&chunk[cursor..hit.start]);
                }
                tracker.on_marker(&hit.marker);
                cursor = hit.end;
            }
            if cursor < chunk.len() {
                tracker.on_region_bytes(&chunk[cursor..]);
            }
        }
        std::hint::black_box(tracker.blocks().len());
    });
    mb_per_second(stream.len(), took)
}

fn theme_import() -> f64 {
    let dracula = include_bytes!("../../termzip-core/tests/fixtures/themes/dracula.itermcolors");
    micros_per_call(300, || {
        std::hint::black_box(termzip_core::import_theme("dracula.itermcolors", dracula).ok());
    })
}

fn task_files() -> f64 {
    let package = include_str!("../../termzip-tasks/tests/fixtures/package.json");
    let makefile = include_str!("../../termzip-tasks/tests/fixtures/Makefile");
    let justfile = include_str!("../../termzip-tasks/tests/fixtures/justfile");
    micros_per_call(2_000, || {
        std::hint::black_box(termzip_tasks::package_scripts(package));
        std::hint::black_box(termzip_tasks::makefile_targets(makefile));
        std::hint::black_box(termzip_tasks::justfile_recipes(justfile));
    })
}

fn git_status() -> f64 {
    let busy = include_bytes!("../../termzip-git/tests/fixtures/busy.bin");
    micros_per_call(20_000, || {
        std::hint::black_box(termzip_git::parse_status(busy));
    })
}

fn snippet_render() -> f64 {
    let snippet = termzip_core::Snippet {
        name: "Checkout".to_owned(),
        command: "git checkout {{branch:main}} && echo {{message}}".to_owned(),
        ..termzip_core::Snippet::default()
    };
    let values = BTreeMap::from([("message".to_owned(), "done".to_owned())]);
    micros_per_call(50_000, || {
        std::hint::black_box(termzip_core::render(&snippet, &values).ok());
    })
}

/// Bytes a real PTY carries from a program to the reader of the session, in megabytes per second.
#[cfg(unix)]
fn pty_throughput() -> Option<f64> {
    use termzip_pty::{PtyOptions, PtySession};

    const MEGABYTES: usize = 64;
    let mut speeds = Vec::new();
    for _ in 0..3 {
        let options = PtyOptions {
            shell: Some("/bin/sh".into()),
            args: vec![
                "-c".to_owned(),
                format!("dd if=/dev/zero bs=1048576 count={MEGABYTES} 2>/dev/null | tr '\\0' 'x'"),
            ],
            ..PtyOptions::default()
        };
        let (_session, receiver) = PtySession::spawn(options, || {}).ok()?;
        let start = Instant::now();
        let mut bytes = 0;
        while let Ok(chunk) = receiver.recv_timeout(Duration::from_secs(30)) {
            bytes += chunk.len();
        }
        speeds.push(mb_per_second(bytes, start.elapsed()));
    }
    speeds.sort_by(f64::total_cmp);
    Some(speeds[speeds.len() / 2])
}

#[cfg(not(unix))]
fn pty_throughput() -> Option<f64> {
    None
}

fn run(json_path: Option<&str>) -> ExitCode {
    let mut metrics = BTreeMap::new();
    let mut note = |name: &str, unit: &str, value: Option<f64>| {
        if let Some(value) = value {
            println!("{name:<22} {value:>12.1} {unit}");
            metrics.insert(name.to_owned(), value);
        } else {
            println!("{name:<22} {:>12} {unit}", "skipped");
        }
    };
    note("osc_parser_mb_s", "MB/s", Some(osc_parser()));
    note("block_tracker_mb_s", "MB/s", Some(block_tracker()));
    note("theme_import_us", "µs", Some(theme_import()));
    note("task_files_us", "µs", Some(task_files()));
    note("git_status_us", "µs", Some(git_status()));
    note("snippet_render_us", "µs", Some(snippet_render()));
    note("pty_mb_s", "MB/s", pty_throughput());

    if let Some(path) = json_path {
        let report = json!({ "os": std::env::consts::OS, "arch": std::env::consts::ARCH, "metrics": metrics });
        if let Err(error) = std::fs::write(
            path,
            serde_json::to_string_pretty(&report).unwrap_or_default(),
        ) {
            eprintln!("could not write {path}: {error}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

fn verify(budgets_path: &str, results: &[String]) -> ExitCode {
    let budgets = match std::fs::read_to_string(budgets_path)
        .map_err(|error| error.to_string())
        .and_then(|text| parse_budgets(&text))
    {
        Ok(budgets) => budgets,
        Err(error) => {
            eprintln!("budgets: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut metrics = BTreeMap::new();
    for path in results {
        match std::fs::read_to_string(path)
            .map_err(|error| error.to_string())
            .and_then(|text| read_results(&text))
        {
            Ok(found) => metrics.extend(found),
            Err(error) => {
                eprintln!("{path}: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    let mut failed = false;
    // A shared two-core CI runner is much further from the reference hardware than 2x (Windows measures the block
    // tracker at 260 MB/s against 738 here), so the smoke jobs widen the slack; a regression is still orders away.
    let slack = std::env::var("TERMZIP_BENCH_SLACK")
        .ok()
        .and_then(|text| text.parse::<f64>().ok())
        .filter(|slack| *slack >= 1.0)
        .unwrap_or(SLACK);
    for verdict in check(&budgets, &metrics, slack) {
        let limit = match (verdict.budget.min, verdict.budget.max) {
            (Some(min), _) => format!("at least {min}"),
            (_, Some(max)) => format!("at most {max}"),
            _ => String::new(),
        };
        let state = if verdict.met {
            "ok"
        } else if verdict.acceptable {
            "over budget, within the slack"
        } else {
            failed = true;
            "FAILED"
        };
        println!(
            "{:<26} {:>14.1}  budget {limit}: {state}",
            verdict.name, verdict.value
        );
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") => run(args
            .windows(2)
            .find(|pair| pair[0] == "--json")
            .map(|pair| pair[1].as_str())),
        Some("check") if args.len() >= 3 => verify(&args[1], &args[2..]),
        _ => {
            eprintln!(
                "usage: termzip-bench run [--json file]\n       termzip-bench check <budgets.toml> <results.json>..."
            );
            ExitCode::from(2)
        }
    }
}
