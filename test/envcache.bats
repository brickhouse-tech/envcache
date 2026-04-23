#!/usr/bin/env bats

setup() {
  TEST_DIR="$BATS_TMPDIR/envcache-test-$$"
  mkdir -p "$TEST_DIR"

  # Resolve absolute path to the script under test
  ENVCACHE_SCRIPT="$(cd "$BATS_TEST_DIRNAME/.." && pwd)/src/sh/envcache.sh"

  export ENVCACHE_SECRETS_FILE="$TEST_DIR/.envcache.secrets"
  export ENVCACHE_CACHE_FILE="$TEST_DIR/.envrc.cache"
  export ENVCACHE_META_FILE="$TEST_DIR/.envrc.cache.meta"
  export ENVCACHE_TTL=28800

  # Create a mock op that returns predictable values
  mkdir -p "$TEST_DIR/bin"
  cat > "$TEST_DIR/bin/op" << 'MOCKOP'
#!/usr/bin/env bash
if [[ "$1" == "read" ]]; then
  uri="$2"
  field="${uri##*/}"
  echo "mock-${field}-value"
  exit 0
fi
exit 1
MOCKOP
  chmod +x "$TEST_DIR/bin/op"
  export PATH="$TEST_DIR/bin:$PATH"

  # Create a runner script that sources envcache in a clean subshell
  cat > "$TEST_DIR/run-envcache.sh" << RUNNER
#!/usr/bin/env bash
export ENVCACHE_SECRETS_FILE="$ENVCACHE_SECRETS_FILE"
export ENVCACHE_CACHE_FILE="$ENVCACHE_CACHE_FILE"
export ENVCACHE_META_FILE="$ENVCACHE_META_FILE"
export ENVCACHE_TTL="$ENVCACHE_TTL"
export PATH="$TEST_DIR/bin:\$PATH"
[[ -n "\${ENVCACHE_FORCE:-}" ]] && export ENVCACHE_FORCE
source "$ENVCACHE_SCRIPT"
# Run any extra command passed as args
if [[ \$# -gt 0 ]]; then
  eval "\$@"
fi
RUNNER
  chmod +x "$TEST_DIR/run-envcache.sh"
}

teardown() {
  rm -rf "$TEST_DIR"
}

@test "creates cache on first run with secrets file" {
  cat > "$ENVCACHE_SECRETS_FILE" << 'EOF'
MY_TOKEN|op://vault/item/password
MY_KEY|op://vault/item/key
EOF

  run "$TEST_DIR/run-envcache.sh"
  [ "$status" -eq 0 ]
  [[ "$output" == *"resolving secrets from 1Password"* ]]
  [[ "$output" == *"resolved 2 secrets"* ]]
  [ -f "$ENVCACHE_CACHE_FILE" ]
  [ -f "$ENVCACHE_META_FILE" ]
}

@test "uses cache on second run" {
  cat > "$ENVCACHE_SECRETS_FILE" << 'EOF'
MY_TOKEN|op://vault/item/password
EOF

  "$TEST_DIR/run-envcache.sh" > /dev/null 2>&1

  run "$TEST_DIR/run-envcache.sh"
  [ "$status" -eq 0 ]
  [[ "$output" == *"using cache"* ]]
}

@test "force refresh with ENVCACHE_FORCE=1" {
  cat > "$ENVCACHE_SECRETS_FILE" << 'EOF'
MY_TOKEN|op://vault/item/password
EOF

  "$TEST_DIR/run-envcache.sh" > /dev/null 2>&1

  run env ENVCACHE_FORCE=1 "$TEST_DIR/run-envcache.sh"
  [ "$status" -eq 0 ]
  [[ "$output" == *"resolving secrets from 1Password"* ]]
}

@test "refreshes on new day" {
  cat > "$ENVCACHE_SECRETS_FILE" << 'EOF'
MY_TOKEN|op://vault/item/password
EOF

  "$TEST_DIR/run-envcache.sh" > /dev/null 2>&1

  # Fake yesterday's date in meta, preserve hash
  local hash
  hash=$(sed -n '3p' "$ENVCACHE_META_FILE")
  printf '%s\n%s\n%s\n' "1999-01-01" "$(date +%s)" "$hash" > "$ENVCACHE_META_FILE"

  run "$TEST_DIR/run-envcache.sh"
  [ "$status" -eq 0 ]
  [[ "$output" == *"resolving secrets from 1Password"* ]]
}

@test "refreshes when secrets file changes" {
  cat > "$ENVCACHE_SECRETS_FILE" << 'EOF'
MY_TOKEN|op://vault/item/password
EOF

  "$TEST_DIR/run-envcache.sh" > /dev/null 2>&1

  echo "NEW_SECRET|op://vault/item/new" >> "$ENVCACHE_SECRETS_FILE"

  run "$TEST_DIR/run-envcache.sh"
  [ "$status" -eq 0 ]
  [[ "$output" == *"resolving secrets from 1Password"* ]]
}

@test "cache file has correct permissions" {
  cat > "$ENVCACHE_SECRETS_FILE" << 'EOF'
MY_TOKEN|op://vault/item/password
EOF

  "$TEST_DIR/run-envcache.sh" > /dev/null 2>&1

  local perms
  perms=$(stat -f '%Lp' "$ENVCACHE_CACHE_FILE" 2>/dev/null || stat -c '%a' "$ENVCACHE_CACHE_FILE")
  [ "$perms" = "600" ]
}

@test "exports correct values from cache" {
  cat > "$ENVCACHE_SECRETS_FILE" << 'EOF'
MY_TOKEN|op://vault/item/password
EOF

  run "$TEST_DIR/run-envcache.sh" 'echo "VALUE=$MY_TOKEN"'
  [ "$status" -eq 0 ]
  [[ "$output" == *"VALUE=mock-password-value"* ]]
}

@test "skips comments and blank lines in secrets file" {
  cat > "$ENVCACHE_SECRETS_FILE" << 'EOF'
# This is a comment
MY_TOKEN|op://vault/item/password

# Another comment
MY_KEY|op://vault/item/key
EOF

  run "$TEST_DIR/run-envcache.sh"
  [ "$status" -eq 0 ]
  [[ "$output" == *"resolved 2 secrets"* ]]
}

@test "strip_whitespace post-processor works" {
  cat > "$ENVCACHE_SECRETS_FILE" << 'EOF'
MY_TOKEN|op://vault/item/password|strip_whitespace
EOF

  "$TEST_DIR/run-envcache.sh" > /dev/null 2>&1

  local cached_value
  cached_value=$(grep MY_TOKEN "$ENVCACHE_CACHE_FILE")
  [[ "$cached_value" == *"mock-password-value"* ]]
}

@test "errors on missing secrets file" {
  rm -f "$ENVCACHE_SECRETS_FILE"

  run "$TEST_DIR/run-envcache.sh"
  [[ "$output" == *"ERROR"* ]] || [[ "$output" == *"not found"* ]]
}
