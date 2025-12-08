# ctx_guard (context guard)

Let your agent run commands without blowing up the context window.

## Description

A Rust based command line tool that wraps commands, pipes the output to a temporary file, passes a summary and pointer back to the agent. It can then choose to dig deeper into the output or not based on the summary.

## Installation

```bash
cargo install --path .
```

## Configuration
Edit the configuration file at:
```
~/.ctx_guard/config.toml
```

## Usage

```bash
cg <command>
```

### Example
Command:
```bash
cg npx jest
```

Output:
```
npx jest failed after 1.2 seconds. The main issue is that 'api.test.ts' could not aquire a token.

The complete output is available at /tmp/ctx_guard/npx_jest_20251208_123456.txt.
```

## Agent Integration

Since `ctx_guard` is designed to be used by AI agents, you'll need to configure your Cursor or VS Code rules to automatically wrap commands.

### Cursor/VS Code Rules Configuration

Add the following rule to your Cursor or VS Code agent configuration:

```
When executing any terminal command with potentially long output like tests, builds, etc., prepend it with `cg` (e.g., `cg <original-command>`). 
This wrapper will:
- Capture the full command output to a temporary file
- Generate a concise summary using the configured LLM provider
- Return only the summary and file path to the agent

Benefits:
- Prevents context window overflow from verbose command outputs
- Reduces token usage and processing time
- Allows the agent to selectively read full output only when needed
```

### Example Rule Implementation

For commands like `npm test` or `cargo build`, the agent should execute:
- `cg npm test` instead of `npm test`
- `cg cargo build` instead of `cargo build`

The agent will receive a summary and can choose to read the full output file if the summary indicates it's necessary.
```