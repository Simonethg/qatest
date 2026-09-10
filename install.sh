#!/bin/sh
set -eu

BIN="qat"
INSTALL_DIR="${QAT_INSTALL_DIR:-$HOME/.local/bin}"
GITHUB_RAW="https://raw.githubusercontent.com/Simonethg/qat/main/latest.json"
GITHUB_REL="https://github.com/Simonethg/qat/releases/latest/download/latest.json"

main() {
  echo ""
  echo "  qat installer"
  echo "  the agent runtime for QA"
  echo "  github.com/Simonethg/qat"
  echo "  Powered by AcademiaQA"
  echo ""

  OS="$(uname -s)"
  case "$OS" in
    Linux) os="linux" ;;
    Darwin) os="macos" ;;
    *) err "unsupported OS: $OS" ;;
  esac

  if [ "$OS" = "Linux" ] && [ "$(uname -o 2>/dev/null || true)" = "Android" ]; then
    err "Android/Termux is not supported by qat release binaries. SSH to a supported host instead."
  fi

  ARCH="$(uname -m)"
  case "$ARCH" in
    x86_64|amd64) arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *) err "unsupported architecture: $ARCH" ;;
  esac

  log "detected ${os}/${arch}"

  need curl
  need awk

  TARGET="${os}-${arch}"

  if [ -n "${QAT_LOCAL_BIN:-}" ] && [ -f "${QAT_LOCAL_BIN}" ]; then
    log "installing local binary ${QAT_LOCAL_BIN}"
    mkdir -p "$INSTALL_DIR"
    cp "${QAT_LOCAL_BIN}" "${INSTALL_DIR}/${BIN}"
    chmod +x "${INSTALL_DIR}/${BIN}"
    finish
    return
  fi

  log "fetching latest release manifest..."
  MANIFEST=""
  if [ -n "${QAT_MANIFEST_URL:-}" ]; then
    MANIFEST="$(curl -fsSL --retry 3 --connect-timeout 10 --max-time 20 "$QAT_MANIFEST_URL")" \
      || err "can't reach ${QAT_MANIFEST_URL}"
  else
    for url in \
      "https://qat.sh/latest.json" \
      "https://getqat.dev/latest.json" \
      "$GITHUB_REL" \
      "$GITHUB_RAW"
    do
      if MANIFEST="$(curl -fsSL --retry 2 --connect-timeout 5 --max-time 15 "$url" 2>/dev/null)"; then
        log "manifest ${url}"
        break
      fi
    done
  fi

  if [ -z "$MANIFEST" ]; then
    err "can't reach a latest.json. Set QAT_MANIFEST_URL, or build from source: cargo install --git https://github.com/Simonethg/qat"
  fi

  URL="$(printf '%s\n' "$MANIFEST" | awk -v target="\"${TARGET}\"" '
    /^[[:space:]]*"assets"[[:space:]]*:/ { in_assets = 1; next }
    in_assets && /^[[:space:]]*}/ { exit }
    in_assets && index($0, target) {
      sub(/^.*:[[:space:]]*"/, "")
      sub(/".*$/, "")
      print
      exit
    }
  ')"
  SHA256="$(printf '%s\n' "$MANIFEST" | awk -v target="\"${TARGET}\"" '
    /^[[:space:]]*"sha256"[[:space:]]*:/ { in_sha256 = 1; next }
    in_sha256 && /^[[:space:]]*}/ { exit }
    in_sha256 && index($0, target) {
      sub(/^.*:[[:space:]]*"/, "")
      sub(/".*$/, "")
      print
      exit
    }
  ')"
  VERSION="$(printf '%s\n' "$MANIFEST" | awk -F '"' '/^[[:space:]]*"version"[[:space:]]*:/ { print $4; exit }')"

  if [ -z "$URL" ] || [ "$URL" = "null" ]; then
    err "release manifest does not include a binary for ${TARGET}. Build from source: cargo install --git https://github.com/Simonethg/qat"
  fi
  if [ "${#SHA256}" -ne 64 ]; then
    err "release manifest does not include a valid SHA-256 checksum for ${TARGET}"
  fi
  if ! printf '%s\n' "$SHA256" | awk '/[^0-9A-Fa-f]/ { exit 1 }'; then
    err "release manifest does not include a valid SHA-256 checksum for ${TARGET}"
  fi
  SHA256="$(printf '%s\n' "$SHA256" | awk '{ print tolower($0) }')"

  if command -v sha256sum >/dev/null 2>&1; then
    SHA256_TOOL="sha256sum"
  elif command -v shasum >/dev/null 2>&1; then
    SHA256_TOOL="shasum"
  elif command -v openssl >/dev/null 2>&1; then
    SHA256_TOOL="openssl"
  else
    err "SHA-256 verification requires sha256sum, shasum, or openssl"
  fi

  if [ -n "$VERSION" ]; then
    log "downloading v${VERSION}..."
  else
    log "downloading latest release..."
  fi
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT

  if ! curl -fsSL --retry 3 --connect-timeout 10 --max-time 120 "$URL" -o "${TMP}/${BIN}"; then
    err "download failed from ${URL}"
  fi

  case "$SHA256_TOOL" in
    sha256sum) ACTUAL_SHA256="$(sha256sum < "${TMP}/${BIN}" | awk '{ print $1 }')" ;;
    shasum) ACTUAL_SHA256="$(shasum -a 256 < "${TMP}/${BIN}" | awk '{ print $1 }')" ;;
    openssl) ACTUAL_SHA256="$(openssl dgst -sha256 < "${TMP}/${BIN}" | awk '{ print $NF }')" ;;
  esac
  if [ "$ACTUAL_SHA256" != "$SHA256" ]; then
    err "downloaded qat checksum did not match"
  fi

  mkdir -p "$INSTALL_DIR"
  mv "${TMP}/${BIN}" "${INSTALL_DIR}/${BIN}"
  chmod +x "${INSTALL_DIR}/${BIN}"

  finish
}

finish() {
  log "installed ${BIN} to ${INSTALL_DIR}/${BIN}"

  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      echo ""
      warn "${INSTALL_DIR} is not in your PATH"
      echo "  add it to your shell config:"
      echo ""
      echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
      echo ""
      ;;
  esac

  if command -v "$BIN" >/dev/null 2>&1 || [ -x "${INSTALL_DIR}/${BIN}" ]; then
    echo ""
    log "ready. run 'qat' to get started."
  fi
  echo ""
}

log() { printf ' \033[32m>\033[0m %s\n' "$1"; }
warn() { printf ' \033[33m!\033[0m %s\n' "$1"; }
err() { printf ' \033[31m✗\033[0m %s\n' "$1" >&2; exit 1; }

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    err "requires '$1' — install it first, or download a binary from https://github.com/Simonethg/qat/releases"
  fi
}

main "$@"
