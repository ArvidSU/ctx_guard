# Evaluations

We want to know how ctx_guard performs under different conditions. We are interested in maximizing the speed and quality of the output.

## Overview

The evaluation system measures two key metrics:
1. **Speed**: How long it takes to execute `cg` with different models and configurations
2. **Quality**: Whether summaries contain enough information for an agent to solve challenges

## Prerequisites

- `cg` command must be built and available in PATH
- Python 3.6+ for evaluation scripts
- LLM provider configured and running (e.g., LM Studio)

## Quick Start

1. **Generate config files for your models:**
   ```bash
   python evals/config_manager.py generate evals/models.json
   ```

2. **Run speed evaluation:**
   ```bash
   python evals/speed/runner.py
   ```

3. **Run quality evaluation:**
   ```bash
   python evals/quality/runner.py
   ```

4. **Analyze results:**
   ```bash
   python evals/analyze.py --all --output evals/results/report.md
   ```

## Configuration

### Model Configuration

Edit `evals/models.json` to define your models:

```json
{
  "openai/gpt-oss-20b": {
    "provider_type": "lmstudio",
    "provider_url": "http://127.0.0.1:1234",
    "summary_words": 100
  }
}
```

Then generate config files:
```bash
python evals/config_manager.py generate
```

Config files will be created in `evals/configs/` directory.

### Speed Evaluation Configuration

Edit `evals/speed/config.json`:

```json
{
  "models": {
    "model-name": {
      "max_tokens": 131072,
      "config_files": ["model-name.toml"]
    }
  },
  "size_factors": [0.1, 0.2, 0.5, 1.0]
}
```

- `max_tokens`: Context window size for the model
- `config_files`: List of config files to test (from `evals/configs/`)
- `size_factors`: Multipliers for test file sizes (relative to max_tokens)

### Quality Evaluation Configuration

Edit `evals/quality/config.json`:

```json
{
  "models": {
    "model-name": {
      "config_files": ["model-name.toml"]
    }
  }
}
```

## Measuring Speed

Speed evaluation measures the time it takes to execute `cg -c <config> cat <file>` using different models and configurations.

The evaluation:
- Generates test files of varying sizes (based on model context windows)
- Executes `cg` with each model/config combination
- Records execution time, summary length, and exit code
- Saves results to `evals/results/speed_YYYYMMDD_HHMMSS.csv`

**Run speed evaluation:**
```bash
python evals/speed/runner.py [config.json] [results_dir]
```

**Results format:**
- `model`: Model name
- `config_file`: Config file used
- `size_factor`: Size multiplier (0.1, 0.2, 0.5, 1.0)
- `execution_time`: Total execution time in seconds
- `summary_length`: Length of generated summary in characters
- `exit_code`: Command exit code

## Measuring Quality

Quality evaluation tests whether summaries contain enough information for an agent to solve challenges.

A higher quality output is one that results in the agent being able to make a better decision based on the output. We measure that by posing a number of challenges to the summary models and seeing how many they can solve.

**Challenges:**
1. `cg -c <config> npx jest --testPathPattern=api.test.ts`
2. `cg -c <config> cargo build`
3. `cg -c <config> python script.py`

**Run quality evaluation:**
```bash
python evals/quality/runner.py [config.json] [results_dir]
```

**Results format:**
- `model`: Model name
- `config_file`: Config file used
- `challenge`: Challenge name
- `can_solve`: Whether challenge can be solved from summary (boolean)
- `needs_full_output`: Whether full output file is needed (boolean)
- `quality_score`: Quality score (0.0-1.0)
- `exit_code`: Command exit code

### Adding New Challenges

Create a new JSON file in `evals/quality/challenges/`:

```json
{
  "name": "Challenge Name",
  "description": "Description of the challenge",
  "command": "command to run",
  "expected_issue": "key issue to identify",
  "expected_solution": "key solution to mention",
  "key_phrases": [
    "important",
    "phrases",
    "to",
    "check"
  ]
}
```

The evaluation system will automatically discover and run all challenges in the directory.

## Results Analysis

The `analyze.py` script aggregates results and generates reports.

**Analyze specific result files:**
```bash
python evals/analyze.py evals/results/speed_20231208_120000.csv evals/results/quality_20231208_120000.csv --output report.md
```

**Analyze most recent results:**
```bash
python evals/analyze.py --all --output report.md
```

**Output formats:**
- Markdown (`.md`): Human-readable table format
- JSON (`.json`): Machine-readable structured data

## Config Manager

The `config_manager.py` utility helps manage evaluation config files.

**Generate configs from models.json:**
```bash
python evals/config_manager.py generate [models.json] [output_dir]
```

**Validate a config file:**
```bash
python evals/config_manager.py validate evals/configs/model.toml
```

**List all configs:**
```bash
python evals/config_manager.py list [configs_dir]
```

## File Structure

```
evals/
├── README.md                    # This file
├── config_manager.py            # Config file management utility
├── analyze.py                   # Results analysis tool
├── models.json                  # Model definitions
├── configs/                     # Generated config files
│   ├── model1.toml
│   └── model2.toml
├── results/                     # Evaluation results
│   ├── speed_YYYYMMDD_HHMMSS.csv
│   └── quality_YYYYMMDD_HHMMSS.csv
├── speed/
│   ├── runner.py               # Speed evaluation runner
│   ├── config.json             # Speed evaluation config
│   ├── absolute_tokens.py      # Legacy script (may be refactored)
│   └── relative_context_window.py  # Legacy script (may be refactored)
└── quality/
    ├── runner.py                # Quality evaluation runner
    ├── config.json              # Quality evaluation config
    └── challenges/              # Challenge definitions
        ├── jest_test_failure.json
        ├── cargo_build_error.json
        └── python_script_error.json
```

## Using Custom Config Files

The `cg` command now supports a `-c/--config` flag to specify a custom config file:

```bash
cg -c evals/configs/my-model.toml cat file.txt
```

This allows you to test different configurations without modifying your default config.

## Result Interpretation

### Speed Results

- **Lower execution time is better**: Faster summaries mean less waiting
- **Summary length**: Should be reasonable (not too short, not too verbose)
- **Compare across models**: See which models are fastest for your use case

### Quality Results

- **Quality score (0.0-1.0)**: Higher is better
  - 0.7+: Summary likely sufficient for agent decision-making
  - 0.5-0.7: Summary may need supplementation with full output
  - <0.5: Summary likely insufficient, full output needed
- **Solve rate**: Percentage of challenges that can be solved from summary alone
- **Full output rate**: Percentage of challenges requiring full output file

## Troubleshooting

**Config file not found:**
- Run `python evals/config_manager.py generate` to create config files
- Check that config files are in `evals/configs/` directory

**Command execution fails:**
- Ensure `cg` is in PATH: `which cg`
- Check that LLM provider is running and accessible
- Verify config file paths are correct

**No results generated:**
- Check that challenge files exist in `evals/quality/challenges/`
- Verify model configs in `evals/speed/config.json` and `evals/quality/config.json`
- Ensure config files referenced in configs exist in `evals/configs/`
