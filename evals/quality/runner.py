#!/usr/bin/env python3
"""
Quality evaluation runner for ctx_guard.

Tests whether summaries contain enough information for an agent to solve challenges.
"""

import json
import os
import shlex
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))

TIMEOUT_SECONDS = 300

def load_config(config_path: Path) -> Dict:
    """Load evaluation configuration from JSON file."""
    with open(config_path, 'r') as f:
        return json.load(f)

def load_challenge(challenge_path: Path) -> Dict:
    """Load a challenge definition from JSON file."""
    with open(challenge_path, 'r') as f:
        return json.load(f)

def run_command(
    config_file: str,
    command: str,
    cwd: Path,
    timeout: int = TIMEOUT_SECONDS,
    force_summary: bool = True,
) -> Tuple[str, int, Optional[str], bool]:
    """
    Run a command with cg and capture output.
    Returns: (stdout, exit_code, output_file_path, used_raw_output)
    """
    def execute(use_force: bool) -> Tuple[str, str, int]:
        force_flag = "--force-summary" if use_force else ""
        cg_command = f"cg -c {shlex.quote(config_file)} {force_flag} {command}".strip()
        result = subprocess.run(
            ["sh", "-c", cg_command],
            capture_output=True,
            text=True,
            cwd=cwd,
            timeout=timeout,
        )
        return result.stdout, result.stderr, result.returncode

    try:
        stdout, stderr, exit_code = execute(force_summary)
    except subprocess.TimeoutExpired as e:
        stdout = (e.stdout or "") + "\n" + (e.stderr or "")
        stderr = ""
        exit_code = 124

    # Retry without --force-summary if the installed cg doesn't support it
    if exit_code != 0 and force_summary:
        combined = f"{stdout}\n{stderr}".lower()
        if "unexpected argument '--force-summary'" in combined or "--force-summary" in combined:
            print("  Info: --force-summary not supported by cg, retrying without it.")
            stdout, stderr, exit_code = execute(False)

    # Extract output file path from stdout
    output_file = None
    for line in stdout.split('\n'):
        if 'complete output is available at' in line:
            # Extract path from line like "The complete output is available at /path/to/file"
            parts = line.split('available at ')
            if len(parts) > 1:
                output_file = parts[1].strip()
            break

    # Detect when cg skipped summarization and returned raw output
    summary_lower = stdout.lower()
    used_raw_output = "output shorter than" in summary_lower and "returning raw output" in summary_lower

    return stdout, exit_code, output_file, used_raw_output

def evaluate_summary_quality(summary: str, challenge: Dict) -> Dict:
    """
    Evaluate whether the summary contains key information needed to solve the challenge.
    Returns a dict with evaluation metrics.
    """
    expected_issue = challenge.get("expected_issue", "").lower()
    expected_solution = challenge.get("expected_solution", "").lower()
    key_phrases = challenge.get("key_phrases", [])
    
    summary_lower = summary.lower()
    
    # Check if summary mentions the expected issue
    issue_found = expected_issue in summary_lower if expected_issue else True
    
    # Check if summary mentions the expected solution
    solution_found = expected_solution in summary_lower if expected_solution else True
    
    # Check for key phrases
    phrases_found = sum(1 for phrase in key_phrases if phrase.lower() in summary_lower)
    phrase_coverage = phrases_found / len(key_phrases) if key_phrases else 1.0
    
    # Overall quality score (0-1)
    quality_score = 0.0
    if issue_found:
        quality_score += 0.4
    if solution_found:
        quality_score += 0.4
    quality_score += phrase_coverage * 0.2
    
    # Determine if challenge can be solved from summary alone
    can_solve = quality_score >= 0.7
    
    return {
        "can_solve": can_solve,
        "quality_score": quality_score,
        "issue_found": issue_found,
        "solution_found": solution_found,
        "phrase_coverage": phrase_coverage,
        "phrases_found": phrases_found,
        "total_phrases": len(key_phrases)
    }

def write_result_file(result_path: Path, model: str, config_file: str,
                     challenge_name: str, can_solve: bool, needs_full_output: bool,
                     quality_score: float, summary: str, exit_code: int) -> None:
    """Write evaluation result to CSV file."""
    summary_clean = summary.replace('\n', ' ').replace(',', ';')[:500]
    
    with open(result_path, 'a') as f:
        f.write(f"{model},{config_file},{challenge_name},{can_solve},"
                f"{needs_full_output},{quality_score:.3f},{exit_code},{summary_clean}\n")

def run_quality_evaluation(config_path: Path, results_dir: Path) -> None:
    """Run quality evaluation for all models, configurations, and challenges."""
    config = load_config(config_path)
    
    # Create results directory if it doesn't exist
    results_dir.mkdir(parents=True, exist_ok=True)

    # Use repo root as working directory
    repo_root = Path(__file__).resolve().parent.parent.parent
    
    # Generate result filename with timestamp
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    result_file = results_dir / f"quality_{timestamp}.csv"
    
    # Write CSV header
    with open(result_file, 'w') as f:
        f.write("model,config_file,challenge,can_solve,needs_full_output,"
                "quality_score,exit_code,summary\n")
    
    models = config.get("models", {})
    challenges_dir = Path(__file__).parent / "challenges"
    configs_dir = Path(__file__).parent.parent / "configs"
    
    # Get list of challenges
    challenge_files = sorted(challenges_dir.glob("*.json"))
    
    if not challenge_files:
        print(f"Warning: No challenge files found in {challenges_dir}")
        return
    
    print(f"Starting quality evaluation...")
    print(f"Results will be written to: {result_file}")
    print(f"Models: {list(models.keys())}")
    print(f"Challenges: {[c.stem for c in challenge_files]}")
    print()
    
    for model_name, model_config in models.items():
        config_files = model_config.get("config_files", [])
        
        if not config_files:
            print(f"Warning: No config files specified for model {model_name}, skipping.")
            continue
        
        for config_file in config_files:
            config_path_full = configs_dir / config_file
            if not config_path_full.exists():
                print(f"Warning: Config file {config_path_full} does not exist, skipping.")
                continue
            
            print(f"Evaluating {model_name} with config {config_file}...")
            
            for challenge_file in challenge_files:
                challenge = load_challenge(challenge_file)
                challenge_name = challenge_file.stem
                command = challenge.get("command", "")
                
                if not command:
                    print(f"  Warning: Challenge {challenge_name} has no command, skipping.")
                    continue
                
                print(f"  Challenge: {challenge_name}...", end=" ", flush=True)
                
                try:
                    # Run the command
                    stdout, exit_code, output_file, used_raw_output = run_command(
                        str(config_path_full),
                        command,
                        cwd=repo_root,
                        force_summary=True,
                    )
                    
                    # Evaluate summary quality (treat raw-output responses as insufficient)
                    if used_raw_output:
                        evaluation = {
                            "can_solve": False,
                            "quality_score": 0.0,
                            "issue_found": False,
                            "solution_found": False,
                            "phrase_coverage": 0.0,
                            "phrases_found": 0,
                            "total_phrases": len(challenge.get("key_phrases", [])),
                        }
                        needs_full_output = True
                    else:
                        evaluation = evaluate_summary_quality(stdout, challenge)
                        # For now, assume full output is needed if quality_score < 0.5
                        needs_full_output = evaluation["quality_score"] < 0.5
                    
                    print(f"Quality: {evaluation['quality_score']:.2f}, "
                          f"Can solve: {evaluation['can_solve']}")
                    
                    # Write result
                    write_result_file(
                        result_file, model_name, config_file, challenge_name,
                        evaluation["can_solve"], needs_full_output,
                        evaluation["quality_score"], stdout, exit_code
                    )
                    
                except Exception as e:
                    print(f"ERROR: {e}")
    
    print(f"\nEvaluation complete. Results saved to: {result_file}")

def main():
    """Main entry point."""
    script_dir = Path(__file__).parent
    evals_dir = script_dir.parent
    
    # Default paths
    config_path = script_dir / "config.json"
    results_dir = evals_dir / "results"
    
    # Allow override via command line
    if len(sys.argv) > 1:
        config_path = Path(sys.argv[1])
    if len(sys.argv) > 2:
        results_dir = Path(sys.argv[2])
    
    if not config_path.exists():
        print(f"Error: Config file not found: {config_path}")
        print("Create a config.json file with the following structure:")
        print("""
{
  "models": {
    "model-name": {
      "config_files": ["model-name.toml"]
    }
  }
}
""")
        sys.exit(1)
    
    run_quality_evaluation(config_path, results_dir)

if __name__ == "__main__":
    main()


