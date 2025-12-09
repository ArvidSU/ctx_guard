import subprocess
import tempfile
import time
"""
Evaluate the speed and quality of output by varying the size of the command output and model.

We'll write a file of different sizes and cat the file using the cg command.
"""

token_to_bytes = 4

config = {
    "openai/gpt-oss-20b": {
        "max_tokens": 131072,
    },
    "qwen/qwen3-vl-4b": {
        "max_tokens": 131072,
    },
}

size_factors = [0.1, 0.2, 0.5, 1]

def write_test_file(max_tokens: int, size_factor: float) -> str:
    """
    Write a file of the given size to the temporary directory.
    """
    with tempfile.NamedTemporaryFile(mode='w', delete=False) as f:
        f.write("a" * int(size_factor * max_tokens))
    return f.name

def write_result_file(model: str, size_factor: float, execution_time: float, summary: str) -> str:
    """
    Write the result of a execution to a file.
    """
    with open("evals/tps/absolute_tokens.csv", "a") as f:
        f.write(f"{model},{size_factor},{execution_time},{summary}\n")

for model, model_config in config.items():
    for size_factor in size_factors:
        start_time = time.time()
        filename = write_test_file(model_config["max_tokens"], size_factor)
        print(f"Written file {filename} of size {size_factor} tokens")
        result = subprocess.run(["cg", "cat", filename], capture_output=True, text=True)
        end_time = time.time()
        execution_time = end_time - start_time
        write_result_file(model, size_factor, execution_time, result.stdout)