# omni-code

Unified AI coding proxy — route any LLM CLI through a single control plane.

## Quick Start

```bash
cargo install omni-code && omni-code proxy
```

## Features

- **Proxy mode** — intercept and route LLM API calls
- **Multi-provider** — OpenAI, Anthropic, and more behind one endpoint
- **Cost tracking** — per-session and per-project token/cost accounting
- **Single binary** — no runtime dependencies, just `omni-code`

## Supported Providers

All OpenAI-compatible providers work out of the box. Configure in `~/.omni-code/config.toml`:

| Provider | Endpoint | Notes |
|----------|----------|-------|
| **Ollama** | `http://localhost:11434/v1` | Local, free, no API key needed |
| **OpenRouter** | `https://openrouter.ai/api/v1` | Aggregator — access many models with one key |
| **NVIDIA** | `https://integrate.api.nvidia.com/v1` | Llama, Nemotron |
| **DeepSeek** | `https://api.deepseek.com/v1` | DeepSeek-Chat, DeepSeek-Reasoner |
| **Groq** | `https://api.groq.com/openai/v1` | Ultra-fast inference |
| **Together** | `https://api.together.xyz/v1` | Open-source models |
| **xAI** | `https://api.x.ai/v1` | Grok-3 |
| **OpenAI** | `https://api.openai.com` | GPT-4o, GPT-4o-mini |

### API Key Configuration

Use environment variables to avoid committing secrets:

```toml
[[providers]]
name = "openrouter"
endpoint = "https://openrouter.ai/api/v1"
api_key = "${OPENROUTER_API_KEY}"
models = ["openai/gpt-4o", "anthropic/claude-sonnet"]
```

Set the env var in your shell:
```bash
export OPENROUTER_API_KEY="sk-or-..."
```

## License

MIT
