#!/usr/bin/env python3
"""
Configuration manager for evaluation config files.

Generates and manages config files for different models from templates.
"""

import json
import shutil
from pathlib import Path
from typing import Dict, Optional
import sys

# Default config template
DEFAULT_CONFIG_TEMPLATE = """# Number of days to keep temporary output files before cleaning them up
clean_up_days = 5

# Number of minutes to look back for command context (0 = disabled)
command_context_minutes = 0

# The provider to use for the summary generation
[provider]
type = "{provider_type}"
url = "{provider_url}"
model = "{model_name}"

prompt = \"\"\"
You are a command output analyzer that provides concise, actionable summaries for AI agents.

${{recent_commands}}

Command executed: ${{command}}
Exit code: ${{exit_code}}
Output:

${{output}}

Generate a summary in ${{summary_words}} words or less following these guidelines:

1. PRIORITIZE ACTIONABLE INFORMATION:
   - If the command failed, identify the root cause and suggest specific fixes
   - Include relevant file paths, line numbers, or error codes when available
   - Highlight what needs attention vs. what succeeded

2. STRUCTURE FOR CLARITY:
   - Start with the outcome (success/failure) and key metrics if relevant
   - Focus on errors, warnings, or unexpected behavior first
   - Mention important details like test results, build status, or data counts

3. BE SPECIFIC:
   - Use exact error messages, file names, or identifiers when critical
   - Avoid vague statements like "something went wrong"
   - Include numbers, percentages, or counts when they provide context

4. FORMAT FOR TERMINAL:
   - Use plain text only (no markdown, no special formatting)
   - Keep it scannable with clear, short sentences
   - If the output is very long, focus on the most important parts

5. NEXT STEPS:
   - If errors exist, suggest concrete actions to resolve them
   - If successful, note any important results or follow-up actions needed

Remember: This summary will help an AI agent decide whether to investigate the full output file or proceed with the next task.
\"\"\"

summary_words = {summary_words}
output_length_threshold = {output_length_threshold}

# Per-command configuration
[commands]
"""

def load_model_configs(config_path: Path) -> Dict:
    """Load model configurations from JSON file."""
    with open(config_path, 'r') as f:
        return json.load(f)

def generate_config_file(
    output_path: Path,
    model_name: str,
    provider_type: str,
    provider_url: str,
    summary_words: int = 100,
    output_length_threshold: int = None,
) -> None:
    """
    Generate a config file for a specific model.
    """
    if output_length_threshold is None:
        output_length_threshold = summary_words

    config_content = DEFAULT_CONFIG_TEMPLATE.format(
        provider_type=provider_type,
        provider_url=provider_url,
        model_name=model_name,
        summary_words=summary_words,
        output_length_threshold=output_length_threshold,
    )
    
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, 'w') as f:
        f.write(config_content)
    
    print(f"Generated config file: {output_path}")

def generate_configs_from_models(models_config_path: Path, output_dir: Path) -> None:
    """
    Generate config files for all models defined in the models config.
    """
    models = load_model_configs(models_config_path)
    
    output_dir.mkdir(parents=True, exist_ok=True)
    
    for model_name, model_config in models.items():
        provider_type = model_config.get("provider_type", "lmstudio")
        provider_url = model_config.get("provider_url", "http://127.0.0.1:1234")
        summary_words = model_config.get("summary_words", 100)
        output_length_threshold = model_config.get(
            "output_length_threshold", summary_words
        )
        
        # Generate filename from model name (replace / with -)
        filename = model_name.replace("/", "-") + ".toml"
        output_path = output_dir / filename
        
        generate_config_file(
            output_path,
            model_name,
            provider_type,
            provider_url,
            summary_words,
            output_length_threshold,
        )

def validate_config_file(config_path: Path) -> bool:
    """
    Validate that a config file exists and is readable.
    Returns True if valid, False otherwise.
    """
    if not config_path.exists():
        print(f"Error: Config file does not exist: {config_path}")
        return False
    
    try:
        # Try to read the file
        with open(config_path, 'r') as f:
            content = f.read()
            # Basic validation: check for required sections
            if "[provider]" not in content:
                print(f"Error: Config file missing [provider] section: {config_path}")
                return False
    except Exception as e:
        print(f"Error reading config file {config_path}: {e}")
        return False
    
    return True

def list_configs(configs_dir: Path) -> None:
    """List all config files in the configs directory."""
    if not configs_dir.exists():
        print(f"Configs directory does not exist: {configs_dir}")
        return
    
    config_files = sorted(configs_dir.glob("*.toml"))
    
    if not config_files:
        print(f"No config files found in {configs_dir}")
        return
    
    print(f"Found {len(config_files)} config file(s):")
    for config_file in config_files:
        print(f"  - {config_file.name}")

def main():
    """Main entry point."""
    evals_dir = Path(__file__).parent
    configs_dir = evals_dir / "configs"
    
    if len(sys.argv) < 2:
        print("Usage:")
        print("  python config_manager.py generate [models.json] [output_dir]")
        print("  python config_manager.py validate <config_file>")
        print("  python config_manager.py list [configs_dir]")
        sys.exit(1)
    
    command = sys.argv[1]
    
    if command == "generate":
        models_config = evals_dir / "models.json"
        output_dir = configs_dir
        
        if len(sys.argv) > 2:
            models_config = Path(sys.argv[2])
        if len(sys.argv) > 3:
            output_dir = Path(sys.argv[3])
        
        if not models_config.exists():
            print(f"Error: Models config file not found: {models_config}")
            print("Create a models.json file with the following structure:")
            print("""
{
  "model-name": {
    "provider_type": "lmstudio",
    "provider_url": "http://127.0.0.1:1234",
    "summary_words": 100
  }
}
""")
            sys.exit(1)
        
        generate_configs_from_models(models_config, output_dir)
    
    elif command == "validate":
        if len(sys.argv) < 3:
            print("Error: validate command requires a config file path")
            sys.exit(1)
        
        config_path = Path(sys.argv[2])
        if validate_config_file(config_path):
            print(f"Config file is valid: {config_path}")
        else:
            sys.exit(1)
    
    elif command == "list":
        if len(sys.argv) > 2:
            configs_dir = Path(sys.argv[2])
        list_configs(configs_dir)
    
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)

if __name__ == "__main__":
    main()


