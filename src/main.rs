use clap::Parser;
use ctx_guard::config::Config;
use ctx_guard::executor::execute_command_string;
use ctx_guard::llm::LlmClient;
use ctx_guard::output::{format_fallback_output, generate_output_filename, write_output_file};
use std::time::Instant;

#[derive(Parser)]
#[command(name = "cg")]
#[command(about = "Context guard - wrap commands and summarize output for AI agents")]
struct Args {
    /// Command to execute (all remaining arguments)
    #[arg(trailing_var_arg = true, required = true)]
    command: Vec<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let command_str = args.command.join(" ");

    // Load configuration
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Warning: Failed to load config: {}. Using defaults.", e);
            Config::default()
        }
    };

    // Check if command is disabled
    if config.is_command_disabled(&command_str) {
        eprintln!("Command '{}' is disabled in configuration", command_str);
        std::process::exit(1);
    }

    // Execute the command
    let cmd_exec_start_time = Instant::now();
    let result = match execute_command_string(&command_str) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error executing command: {}", e);
            std::process::exit(1);
        }
    };
    let cmd_exec_duration = cmd_exec_start_time.elapsed();

    // Write output to temp file
    let output_file_start_time = Instant::now();
    let filename = generate_output_filename(&command_str);
    let output_path = match write_output_file(&filename, &result.combined_output) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error writing output file: {}", e);
            std::process::exit(1);
        }
    };
    let output_file_duration = output_file_start_time.elapsed();

    // Get summary words for this command
    let summary_words = config.get_summary_words(&command_str);

    // Generate summary
    let summary_start_time = Instant::now();
    let summary = if result.combined_output.trim().is_empty() {
        if result.is_success() {
            format!("Command completed successfully in {:.1} seconds with no output.", cmd_exec_duration.as_secs_f64())
        } else {
            format!("Command failed after {:.1} seconds with exit code {} and no output.", cmd_exec_duration.as_secs_f64(), result.exit_code)
        }
    } else {
        let prompt = config.format_prompt(&command_str, result.exit_code, &result.combined_output, summary_words);
        
        let llm_client = LlmClient::new(&config.provider.url);
        match llm_client.summarize(&prompt).await {
            Ok(summary) => {
                summary
            }
            Err(_) => {
                // Fallback to truncated output
                let truncated = format_fallback_output(&result.combined_output, 20);
                let status = if result.is_success() {
                    "succeeded"
                } else {
                    "failed"
                };
                format!("{} {} after {:.1} seconds. Output:\n\n{}", 
                    command_str, 
                    status, 
                    cmd_exec_duration.as_secs_f64(),
                    truncated
                )
            }
        }
    };
    let summary_duration = summary_start_time.elapsed();

    // Print summary and file path
    println!("{}", summary);
    println!("\nThe complete output is available at {}, prefer reading parts of the output from the file (grep, tail, etc.) instead of the whole thing", output_path.display());

    const DEBUG: bool = false;
    if DEBUG {
        println!("\nGenerated summary in {:.1} seconds", summary_duration.as_secs_f64());
        println!("Command execution took {:.1} seconds", cmd_exec_duration.as_secs_f64());
        println!("Output file writing took {:.1} seconds", output_file_duration.as_secs_f64());
    }

    // Exit with the same code as the original command
    std::process::exit(result.exit_code);
}

