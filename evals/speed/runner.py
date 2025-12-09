#!/usr/bin/env python3
"""
Unified speed evaluation runner for ctx_guard.

Measures execution time for different models and configurations across
various output sizes.
"""

import json
import os
import shlex
import subprocess
import sys
import tempfile
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Tuple

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent))

def load_config(config_path: Path) -> Dict:
    """Load evaluation configuration from JSON file."""
    with open(config_path, 'r') as f:
        return json.load(f)

def write_test_file(max_tokens: int, size_factor: float) -> str:
    """
    Write a test file of the given size to the temporary directory.
    Size is calculated as size_factor * max_tokens characters.
    """
    size = int(size_factor * max_tokens)
    with tempfile.NamedTemporaryFile(mode='w', delete=False, suffix='.txt') as f:
        f.write("a" * size)
    return f.name

TIMEOUT_SECONDS = 300


def run_command(
    config_file: str,
    command: str,
    cwd: Path,
    timeout: int = TIMEOUT_SECONDS,
    force_summary: bool = True,
) -> Tuple[float, str, int]:
    """
    Run a command with cg and measure execution time.
    Returns: (execution_time, stdout, exit_code)
    """
    def execute(use_force: bool) -> subprocess.CompletedProcess:
        force_flag = "--force-summary" if use_force else ""
        cg_command = f"cg -c {shlex.quote(config_file)} {force_flag} {command}".strip()
        return subprocess.run(
            ["sh", "-c", cg_command],
            capture_output=True,
            text=True,
            cwd=cwd,
            timeout=timeout,
        )

    start_time = time.time()
    try:
        result = execute(force_summary)
    except subprocess.TimeoutExpired as e:
        end_time = time.time()
        execution_time = end_time - start_time
        stdout = e.stdout or ""
        stderr = e.stderr or ""
        message = (
            f"Command timed out after {timeout}s. stdout: {stdout} stderr: {stderr}"
        )
        return execution_time, message, 124

    # Retry without --force-summary if the installed cg doesn't support it
    if result.returncode != 0 and force_summary:
        combined = f"{result.stdout}\n{result.stderr}".lower()
        if "unexpected argument '--force-summary'" in combined or "--force-summary" in combined:
            print("  Info: --force-summary not supported by cg, retrying without it.")
            result = execute(False)

    end_time = time.time()
    execution_time = end_time - start_time
    return execution_time, result.stdout, result.returncode

def write_result_file(result_path: Path, model: str, config_file: str, 
                     size_factor: float, execution_time: float, 
                     summary: str, exit_code: int) -> None:
    """Write evaluation result to CSV file."""
    # Clean summary for CSV (remove newlines, limit length)
    summary_clean = summary.replace('\n', ' ').replace(',', ';')[:500]
    summary_length = len(summary)
    
    with open(result_path, 'a') as f:
        f.write(f"{model},{config_file},{size_factor},{execution_time:.3f},"
                f"{summary_length},{exit_code},{summary_clean}\n")

def run_speed_evaluation(config_path: Path, results_dir: Path) -> None:
    """Run speed evaluation for all models and configurations."""
    config = load_config(config_path)
    
    # Create results directory if it doesn't exist
    results_dir.mkdir(parents=True, exist_ok=True)

    # Use repo root as working directory
    repo_root = Path(__file__).resolve().parent.parent.parent
    
    # Generate result filename with timestamp
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    result_file = results_dir / f"speed_{timestamp}.csv"
    
    # Write CSV header
    with open(result_file, 'w') as f:
        f.write("model,config_file,size_factor,execution_time,summary_length,exit_code,summary\n")
    
    models = config.get("models", {})
    size_factors = config.get("size_factors", [0.1, 0.2, 0.5, 1.0])
    configs_dir = Path(__file__).parent.parent / "configs"
    
    print(f"Starting speed evaluation...")
    print(f"Results will be written to: {result_file}")
    print(f"Models: {list(models.keys())}")
    print(f"Size factors: {size_factors}")
    print()
    
    for model_name, model_config in models.items():
        max_tokens = model_config.get("max_tokens", 131072)
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
            
            for size_factor in size_factors:
                # Generate test file
                test_file = write_test_file(max_tokens, size_factor)
                file_size = os.path.getsize(test_file)
                
                try:
                    print(f"  Size factor {size_factor} ({file_size} bytes)...", end=" ", flush=True)
                    
                    # Run command
                    exec_time, stdout, exit_code = run_command(
                        str(config_path_full),
                        f"cat {test_file}",
                        cwd=repo_root,
                        force_summary=True,
                    )
                    
                    print(f"{exec_time:.3f}s")
                    
                    # Write result
                    write_result_file(
                        result_file, model_name, config_file, size_factor,
                        exec_time, stdout, exit_code
                    )
                    
                except Exception as e:
                    print(f"ERROR: {e}")
                finally:
                    # Clean up test file
                    try:
                        os.unlink(test_file)
                    except:
                        pass
    
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
      "max_tokens": 131072,
      "config_files": ["model-name.toml"]
    }
  },
  "size_factors": [0.1, 0.2, 0.5, 1.0]
}
""")
        sys.exit(1)
    
    run_speed_evaluation(config_path, results_dir)

if __name__ == "__main__":
    main()


