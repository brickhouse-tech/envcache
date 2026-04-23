#!/usr/bin/env bash
# envcache — Cache environment secrets resolved from 1Password (or other secret managers)
#
# Usage:
#   Source this from your ~/.envrc or ~/.bashrc/.zshrc:
#
#     export OP_SERVICE_ACCOUNT_TOKEN="your-token"
#     source /path/to/envcache.sh
#
# Configuration (env vars):
#   ENVCACHE_SECRETS_FILE  Path to secrets definition file (default: ~/.envcache.secrets)
#   ENVCACHE_CACHE_FILE    Path to cache file (default: ~/.envrc.cache)
#   ENVCACHE_META_FILE     Path to meta file (default: ~/.envrc.cache.meta)
#   ENVCACHE_TTL           Cache TTL in seconds (default: 28800 = 8 hours)
#   ENVCACHE_FORCE         Set to 1 to force refresh (default: 0)
#
# Secrets file format (~/.envcache.secrets):
#   Each line is: VAR_NAME|op://vault/item/field
#   Lines starting with # are comments. Blank lines are ignored.
#   Special post-processing can be added via: VAR_NAME|op://...|strip_whitespace
#
# Triggers for refresh:
#   1. No cache file exists
#   2. TTL expired (default 8 hours)
#   3. First login of the day (date changed since last cache)
#   4. ENVCACHE_FORCE=1
#   5. Secrets file modified since last cache

set -euo pipefail 2>/dev/null || true  # best-effort in sourced context

# ── Cross-platform file hash ──
_envcache_hash() {
  if command -v md5 > /dev/null 2>&1; then
    md5 -q "$1"
  elif command -v md5sum > /dev/null 2>&1; then
    md5sum "$1" | cut -d' ' -f1
  else
    # Fallback: use cksum
    cksum "$1" | cut -d' ' -f1
  fi
}

# ── Configuration ──
_ENVCACHE_SECRETS="${ENVCACHE_SECRETS_FILE:-$HOME/.envcache.secrets}"
_ENVCACHE_CACHE="${ENVCACHE_CACHE_FILE:-$HOME/.envrc.cache}"
_ENVCACHE_META="${ENVCACHE_META_FILE:-$HOME/.envrc.cache.meta}"
_ENVCACHE_TTL="${ENVCACHE_TTL:-28800}"

# ── Cache freshness check ──
_envcache_needs_refresh() {
  # Force refresh via env var
  [[ "${ENVCACHE_FORCE:-0}" == "1" ]] && return 0

  # No cache, meta, or secrets file
  [[ ! -f "$_ENVCACHE_CACHE" ]] && return 0
  [[ ! -f "$_ENVCACHE_META" ]] && return 0
  [[ ! -f "$_ENVCACHE_SECRETS" ]] && { echo "[envcache] ERROR: secrets file not found: $_ENVCACHE_SECRETS" >&2; return 1; }

  # Read meta: line 1 = date (YYYY-MM-DD), line 2 = epoch timestamp, line 3 = secrets file hash
  local cached_date cached_ts cached_hash now today current_hash
  cached_date=$(sed -n '1p' "$_ENVCACHE_META")
  cached_ts=$(sed -n '2p' "$_ENVCACHE_META")
  cached_hash=$(sed -n '3p' "$_ENVCACHE_META")
  now=$(date +%s)
  today=$(date +%Y-%m-%d)

  # Secrets file changed (new secrets added/removed)
  current_hash=$(_envcache_hash "$_ENVCACHE_SECRETS")
  [[ "$cached_hash" != "$current_hash" ]] && return 0

  # First login of the day
  [[ "$cached_date" != "$today" ]] && return 0

  # TTL expired
  (( now - cached_ts > _ENVCACHE_TTL )) && return 0

  return 1
}

# ── Resolve secrets from 1Password ──
_envcache_resolve() {
  local start_ts total=0 errors=0
  start_ts=$(date +%s)

  echo "[envcache] resolving secrets from 1Password..."

  # Ensure secrets file exists
  if [[ ! -f "$_ENVCACHE_SECRETS" ]]; then
    echo "[envcache] ERROR: secrets file not found: $_ENVCACHE_SECRETS" >&2
    echo "[envcache] Create it with lines like: MY_VAR|op://vault/item/field" >&2
    return 1
  fi

  # Write cache file
  : > "$_ENVCACHE_CACHE"
  chmod 600 "$_ENVCACHE_CACHE"

  while IFS= read -r line || [[ -n "$line" ]]; do
    # Skip comments and blank lines
    [[ -z "$line" || "$line" == \#* ]] && continue

    local varname opuri postprocess=""
    varname="${line%%|*}"
    local rest="${line#*|}"

    # Check for post-processor (third field)
    if [[ "$rest" == *"|"* ]]; then
      opuri="${rest%%|*}"
      postprocess="${rest#*|}"
    else
      opuri="$rest"
    fi

    local value
    if ! value=$(op read "$opuri" 2>/dev/null); then
      echo "[envcache] WARNING: failed to resolve $varname" >&2
      ((errors++))
      continue
    fi

    # Apply post-processing
    case "$postprocess" in
      strip_whitespace)
        value=$(printf '%s' "$value" | tr -d "\n\r ")
        ;;
    esac

    # Write to cache (single-quote the value to prevent expansion)
    printf "export %s='%s'\n" "$varname" "$value" >> "$_ENVCACHE_CACHE"
    ((total++))
  done < "$_ENVCACHE_SECRETS"

  # Write meta: date, timestamp, secrets file hash
  local current_hash
  current_hash=$(_envcache_hash "$_ENVCACHE_SECRETS")
  printf '%s\n%s\n%s\n' "$(date +%Y-%m-%d)" "$(date +%s)" "$current_hash" > "$_ENVCACHE_META"
  chmod 600 "$_ENVCACHE_META"

  local elapsed=$(( $(date +%s) - start_ts ))
  echo "[envcache] resolved $total secrets in ${elapsed}s ($errors errors)"
}

# ── Display cache age ──
_envcache_age() {
  [[ ! -f "$_ENVCACHE_META" ]] && echo "no cache" && return
  local cached_ts now diff hours mins
  cached_ts=$(sed -n '2p' "$_ENVCACHE_META")
  now=$(date +%s)
  diff=$(( now - cached_ts ))
  hours=$(( diff / 3600 ))
  mins=$(( (diff % 3600) / 60 ))
  if (( hours > 0 )); then
    echo "${hours}h${mins}m old"
  else
    echo "${mins}m old"
  fi
}

# ── Main ──
if _envcache_needs_refresh; then
  _envcache_resolve
else
  echo "[envcache] using cache ($(_envcache_age))"
fi

# Source the cache (exports all secrets into current shell)
if [[ -f "$_ENVCACHE_CACHE" ]]; then
  # shellcheck source=/dev/null
  source "$_ENVCACHE_CACHE"
fi

# ── Cleanup (don't pollute shell namespace) ──
unset -f _envcache_needs_refresh _envcache_resolve _envcache_age _envcache_hash
unset _ENVCACHE_SECRETS _ENVCACHE_CACHE _ENVCACHE_META _ENVCACHE_TTL
