# envcache

Cache environment secrets resolved from 1Password so you don't pay the cost of `op read` on every shell session.

## Problem

A typical `~/.envrc` with 16 `op read` calls takes **~10 seconds** to source. That's brutal when opening terminals, tmux panes, or new shell sessions.

## Solution

`envcache` resolves secrets once, caches the results, and reuses them until a refresh is triggered.

### Refresh triggers

- **TTL expired** — default 8 hours, configurable
- **First login of the day** — date changed since last cache
- **Force refresh** — `ENVCACHE_FORCE=1 source ~/.envrc`
- **Secrets file changed** — new secrets added/removed

### Cache hit: instant. Cache miss: one-time resolve.

## Quick start

### 1. Create your secrets file

```bash
cp envcache.secrets.example ~/.envcache.secrets
# Edit with your actual 1Password references:
# VAR_NAME|op://vault/item/field
```

### 2. Add to your `~/.envrc`

```bash
# Bootstrap 1Password
export OP_SERVICE_ACCOUNT_TOKEN="your-service-account-token"

# Non-secrets (always set directly)
export SOME_URL="https://api.example.com"

# Cached secrets
source /path/to/envcache.sh
```

### 3. Source it

```bash
source ~/.envrc
# [envcache] resolving secrets from 1Password...
# [envcache] resolved 16 secrets in 9s (0 errors)

source ~/.envrc
# [envcache] using cache (0m old)
```

## Configuration

| Env var | Default | Description |
|---------|---------|-------------|
| `ENVCACHE_SECRETS_FILE` | `~/.envcache.secrets` | Path to secrets definition file |
| `ENVCACHE_CACHE_FILE` | `~/.envrc.cache` | Path to cache file |
| `ENVCACHE_META_FILE` | `~/.envrc.cache.meta` | Path to meta/timestamp file |
| `ENVCACHE_TTL` | `28800` (8h) | Cache TTL in seconds |
| `ENVCACHE_FORCE` | `0` | Set to `1` to force refresh |

## Secrets file format

```
# Comments start with #
VAR_NAME|op://vault/item/field
ANOTHER_VAR|op://vault/item/field|strip_whitespace
```

### Post-processors

| Name | Effect |
|------|--------|
| `strip_whitespace` | Removes `\n`, `\r`, and spaces from the resolved value |

## Security

- Cache file is created with `chmod 600` (owner read/write only)
- Meta file is also `chmod 600`
- Add `*.cache` and `*.cache.meta` to your global `.gitignore`
- The cache contains **plaintext secrets** — same security model as environment variables

## Roadmap

- [ ] Rust CLI (`envcache`) for faster resolution and cross-platform distribution
- [ ] Pluggable resolvers (AWS SSM, HashiCorp Vault, etc.)
- [ ] OS keychain storage option (macOS Keychain, Linux secret-service)
- [ ] `brew install envcache`

## License

MIT
