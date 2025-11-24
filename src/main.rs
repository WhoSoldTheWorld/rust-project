use clap::Parser;
use std::process::Command;
use std::time::Instant;
use std::io::{self, Read, Write};
use serde::Serialize;
use anyhow::Result;
use sysinfo::System; // 0.30+: no SystemExt/ProcessExt

#[derive(Parser, Debug)]
struct Args {
    /// How many times to run the command
    #[arg(short = 'n', long, default_value_t = 1)]
    runs: usize,

    /// Emit results in JSON
    #[arg(long)]
    json: bool,

    /// Wait for you to press ENTER before starting timing
    #[arg(long)]
    wait: bool,

    /// Command to run (everything after --)
    #[arg(trailing_var_arg = true, required = true)]
    cmd: Vec<String>,
}

#[derive(Serialize)]
struct RunResult {
    exit_code: Option<i32>,
    times: Vec<f64>,
    mean: f64,
}

fn wait_for_enter_if_requested(wait: bool) -> Result<()> {
    if wait {
        print!("Click what you need in the client, then press ENTER here to start");
        io::stdout().flush()?;
        let _ = io::stdin().read(&mut [0u8])?; // waits for Enter
    }
    Ok(())
}

fn spawn_cross_platform(cmd: &[String]) -> std::io::Result<std::process::ExitStatus> {
    #[cfg(target_os = "windows")]
    {
        // Builtins like `echo` require running through cmd.exe
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&cmd[0]).args(&cmd[1..]).status()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new(&cmd[0]).args(&cmd[1..]).status()
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Only wait if the flag is provided
    wait_for_enter_if_requested(args.wait)?;

    let mut times = Vec::with_capacity(args.runs);
    let mut last_code: Option<i32> = None;

    for _ in 0..args.runs {
        let start = Instant::now();
        let status = spawn_cross_platform(&args.cmd)?;
        let elapsed = start.elapsed().as_secs_f64();

        println!("Command exited with: {:?}", status.code());
        println!("Elapsed time: {:.3} seconds", elapsed);

        last_code = status.code();
        times.push(elapsed);
    }

    let mean = times.iter().sum::<f64>() / times.len() as f64;

    if args.json {
        let result = RunResult { exit_code: last_code, times, mean };
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Exit code: {:?}", last_code);
        println!("Runs: {}", args.runs);
        println!("Times: {:?}", times);
        println!("Mean: {:.3} sec", mean);
    }

    // Print Riot/League processes (single snapshot)
    let mut sys = System::new_all();
    sys.refresh_all();
    println!("\nTop Riot / League processes running:");
    for (pid, process) in sys.processes() {
        let name_lc = process.name().to_ascii_lowercase();
        if name_lc.contains("riot") || name_lc.contains("league") {
            println!(
                "PID: {:<8} Name: {:<25} CPU: {:>5.1}%  Mem: {:>8} KiB",
                pid.as_u32(),
                process.name(),
                process.cpu_usage(),
                process.memory()
            );
        }
    }

    Ok(())
}
