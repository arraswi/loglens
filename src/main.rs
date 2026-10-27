use chrono::DateTime;
use clap::{Parser, ValueEnum};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::process;

#[derive(Debug, Parser)]
#[command(name = "loglens", version, about = "Summarize JSONL logs without loading the whole file")]
struct Args {
    #[arg(value_name = "FILE")]
    input: PathBuf,

    #[arg(long, value_enum, help = "Include only this log level")]
    level: Option<LogLevel>,

    #[arg(long, default_value_t = 8, help = "Number of frequent messages to show (maximum 100)")]
    top: usize,

    #[arg(long, help = "Write the report as JSON")]
    json: bool,
}

#[derive(Clone, Debug, ValueEnum)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Serialize)]
struct MessageCount {
    message: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct Report {
    file: String,
    total_lines: usize,
    valid_records: usize,
    malformed_lines: usize,
    matched_records: usize,
    level_filter: Option<String>,
    by_level: BTreeMap<String, usize>,
    first_timestamp: Option<String>,
    last_timestamp: Option<String>,
    top_messages: Vec<MessageCount>,
}

fn main() {
    let args = Args::parse();
    if args.top > 100 {
        eprintln!("loglens: --top precisa ser menor ou igual a 100");
        process::exit(2);
    }

    match analyze(&args) {
        Ok(report) if args.json => {
            if let Err(error) = serde_json::to_writer_pretty(io::stdout().lock(), &report) {
                eprintln!("loglens: não foi possível serializar o relatório: {error}");
                process::exit(1);
            }
            println!();
        }
        Ok(report) => print_report(&report),
        Err(error) => {
            eprintln!("loglens: {error}");
            process::exit(1);
        }
    }
}

fn analyze(args: &Args) -> Result<Report, Box<dyn Error>> {
    let file = File::open(&args.input)?;
    let reader = BufReader::new(file);
    let mut total_lines = 0;
    let mut valid_records = 0;
    let mut malformed_lines = 0;
    let mut matched_records = 0;
    let mut by_level = BTreeMap::<String, usize>::new();
    let mut messages = HashMap::<String, usize>::new();
    let mut first_timestamp: Option<(i64, String)> = None;
    let mut last_timestamp: Option<(i64, String)> = None;
    let filter = args.level.as_ref().map(LogLevel::as_str);

    for line in reader.lines() {
        let line = line?;
        total_lines += 1;
        if line.trim().is_empty() {
            continue;
        }

        let record: Value = match serde_json::from_str(&line) {
            Ok(record) => record,
            Err(_) => {
                malformed_lines += 1;
                continue;
            }
        };
        valid_records += 1;

        let raw_level = record
            .get("level")
            .or_else(|| record.get("severity"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_ascii_lowercase();
        let level: &str = if raw_level == "warning" {
            "warn"
        } else {
            raw_level.as_str()
        };
        if filter.is_some_and(|wanted| wanted != level) {
            continue;
        }
        matched_records += 1;
        *by_level.entry(level.to_string()).or_default() += 1;

        let message = record
            .get("message")
            .or_else(|| record.get("msg"))
            .and_then(Value::as_str)
            .unwrap_or("[message missing]")
            .to_string();
        *messages.entry(message).or_default() += 1;

        if let Some(timestamp) = record
            .get("timestamp")
            .or_else(|| record.get("time"))
            .and_then(Value::as_str)
        {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(timestamp) {
                let value = (parsed.timestamp_millis(), parsed.to_rfc3339());
                if first_timestamp
                    .as_ref()
                    .is_none_or(|current| value.0 < current.0)
                {
                    first_timestamp = Some(value.clone());
                }
                if last_timestamp
                    .as_ref()
                    .is_none_or(|current| value.0 > current.0)
                {
                    last_timestamp = Some(value);
                }
            }
        }
    }

    let mut top_messages: Vec<MessageCount> = messages
        .into_iter()
        .map(|(message, count)| MessageCount { message, count })
        .collect();
    top_messages.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.message.cmp(&b.message)));
    top_messages.truncate(args.top);

    Ok(Report {
        file: args.input.display().to_string(),
        total_lines,
        valid_records,
        malformed_lines,
        matched_records,
        level_filter: args.level.as_ref().map(|value| value.as_str().to_string()),
        by_level,
        first_timestamp: first_timestamp.map(|value| value.1),
        last_timestamp: last_timestamp.map(|value| value.1),
        top_messages,
    })
}

fn print_report(report: &Report) {
    println!("LogLens · {}", report.file);
    println!("Linhas lidas:       {}", report.total_lines);
    println!("Registros válidos:  {}", report.valid_records);
    println!("Linhas inválidas:   {}", report.malformed_lines);
    println!("Registros incluídos: {}", report.matched_records);
    if let Some(level) = &report.level_filter {
        println!("Filtro de nível:    {level}");
    }

    println!();
    println!("Registros por nível");
    for (level, count) in &report.by_level {
        println!("  {level:<8} {count}");
    }

    if let Some(first) = &report.first_timestamp {
        println!();
        println!("Intervalo");
        println!("  início  {first}");
        println!("  fim     {}", report.last_timestamp.as_deref().unwrap_or(first));
    }

    if !report.top_messages.is_empty() {
        println!();
        println!("Mensagens mais frequentes");
        for item in &report.top_messages {
            println!("  {:>5}  {}", item.count, item.message);
        }
    }
}
