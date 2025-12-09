#!/bin/bash
# Run all evaluations and generate a report

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EVALS_DIR="$SCRIPT_DIR"
RESULTS_DIR="$EVALS_DIR/results"
CONFIGS_DIR="$EVALS_DIR/configs"

echo "=== ctx_guard Evaluation Suite ==="
echo ""

# Check if configs exist
if [ ! -d "$CONFIGS_DIR" ] || [ -z "$(ls -A $CONFIGS_DIR/*.toml 2>/dev/null)" ]; then
    echo "Generating config files..."
    python "$EVALS_DIR/config_manager.py" generate
    echo ""
fi

# Create results directory
mkdir -p "$RESULTS_DIR"

# Run speed evaluation
echo "=== Running Speed Evaluation ==="
python "$EVALS_DIR/speed/runner.py"
echo ""

# Run quality evaluation
echo "=== Running Quality Evaluation ==="
python "$EVALS_DIR/quality/runner.py"
echo ""

# Generate report
echo "=== Generating Report ==="
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_FILE="$RESULTS_DIR/report_$TIMESTAMP.md"
python "$EVALS_DIR/analyze.py" --all --output "$REPORT_FILE"
echo ""

echo "=== Evaluation Complete ==="
echo "Report saved to: $REPORT_FILE"
echo "Results saved to: $RESULTS_DIR"


