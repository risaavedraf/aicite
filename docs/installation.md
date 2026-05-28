# Installation Guide — CITE CLI

This document describes how to install the CITE CLI on different platforms. Currently, only manual binary download is available. Automated installation methods are planned.

## Current: Manual binary download

### Linux (x86_64)

```bash
curl -sSfL https://github.com/risaavedraf/aiharness/releases/download/v0.1.0/cite-linux-amd64 -o cite
chmod +x cite
sudo mv cite /usr/local/bin/
cite health --json
```

### macOS (Apple Silicon)

```bash
curl -sSfL https://github.com/risaavedraf/aiharness/releases/download/v0.1.0/cite-macos-arm64 -o cite
chmod +x cite
sudo mv cite /usr/local/bin/
cite health --json
```

### Windows (PowerShell)

```powershell
Invoke-WebRequest -Uri "https://github.com/risaavedraf/aiharness/releases/download/v0.1.0/cite-windows-amd64.exe" -OutFile "cite.exe"
# Move to a directory in PATH, e.g.:
Move-Item cite.exe C:\Users\$env:USERNAME\AppData\Local\Microsoft\WinGet\Links\
cite health --json
```

---

## Planned: Automated installation

### Scoop (Windows)

[Scoop](https://scoop.sh/) is a command-line package manager for Windows.

**Installation:**

```powershell
# Add the cite bucket
scoop bucket add cite https://github.com/risaavedraf/cite-scoop

# Install cite
scoop install cite
```

**Updating:**

```powershell
scoop update cite
```

**Manifest example** (`cite.json`):

```json
{
  "version": "0.1.0",
  "description": "CLI-first semantic document engine for AI agents",
  "homepage": "https://github.com/risaavedraf/aiharness",
  "license": "MIT",
  "architecture": {
    "64bit": {
      "url": "https://github.com/risaavedraf/aiharness/releases/download/v0.1.0/cite-windows-amd64.exe#/cite.exe",
      "hash": "sha256:..."
    }
  },
  "bin": "cite.exe",
  "checkver": {
    "github": "https://github.com/risaavedraf/aiharness"
  },
  "autoupdate": {
    "architecture": {
      "64bit": {
        "url": "https://github.com/risaavedraf/aiharness/releases/download/v$version/cite-windows-amd64.exe#/cite.exe"
      }
    }
  }
}
```

**Steps to set up:**

1. Create repo `risaavedraf/cite-scoop` with the manifest
2. Or submit to the main Scoop bucket
3. Update hash after each release (CI can automate this)

---

### Homebrew (macOS/Linux)

[Homebrew](https://brew.sh/) is the standard package manager for macOS and Linux.

**Installation:**

```bash
# Add the tap
brew tap risaavedraf/cite

# Install cite
brew install cite
```

**Updating:**

```bash
brew upgrade cite
```

**Formula example** (`cite.rb`):

```ruby
class Cite < Formula
  desc "CLI-first semantic document engine for AI agents"
  homepage "https://github.com/risaavedraf/aiharness"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/risaavedraf/aiharness/releases/download/v0.1.0/cite-macos-arm64"
      sha256 "..."
    end
    on_intel do
      url "https://github.com/risaavedraf/aiharness/releases/download/v0.1.0/cite-macos-amd64"
      sha256 "..."
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/risaavedraf/aiharness/releases/download/v0.1.0/cite-linux-amd64"
      sha256 "..."
    end
  end

  def install
    bin.install "cite"
  end

  test do
    system "#{bin}/cite", "health", "--json"
  end
end
```

**Steps to set up:**

1. Create repo `risaavedraf/homebrew-cite`
2. Add `cite.rb` formula
3. Update SHA256 hashes after each release (CI can automate this)

---

### Cargo install (Rust developers)

If the user has a Rust toolchain, they can install directly from the repo:

```bash
# From GitHub (once published)
cargo install --git https://github.com/risaavedraf/aiharness --tag v0.1.0

# Or from crates.io (once published)
cargo install cite
```

**Prerequisites:** Rust 1.75+

---

### Install script (Linux/macOS)

A universal install script that detects OS and architecture:

```bash
#!/bin/bash
# install.sh — Instala cite CLI
set -e

VERSION="${HARNESS_VERSION:-0.1.0}"
REPO="risaavedraf/aiharness"
BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"

detect_platform() {
  local os arch
  os=$(uname -s | tr '[:upper:]' '[:lower:]')
  arch=$(uname -m)

  case "$os" in
    linux)  os="linux" ;;
    darwin) os="macos" ;;
    *)      echo "Unsupported OS: $os" >&2; exit 1 ;;
  esac

  case "$arch" in
    x86_64)  arch="amd64" ;;
    aarch64|arm64) arch="arm64" ;;
    *)       echo "Unsupported architecture: $arch" >&2; exit 1 ;;
  esac

  echo "${os}-${arch}"
}

main() {
  local platform
  platform=$(detect_platform)
  local url="${BASE_URL}/cite-${platform}"
  local install_dir="${INSTALL_DIR:-/usr/local/bin}"

  echo "Installing cite v${VERSION} for ${platform}..."
  echo "Download: ${url}"

  curl -sSfL "$url" -o "${install_dir}/cite"
  chmod +x "${install_dir}/cite"

  echo ""
  echo "Installed: ${install_dir}/cite"
  "${install_dir}/cite" health --json
}

main "$@"
```

**Usage:**

```bash
# Default install to /usr/local/bin
curl -sSf https://raw.githubusercontent.com/risaavedraf/aiharness/main/install.sh | sh

# Custom install directory
INSTALL_DIR=~/.local/bin curl -sSf https://raw.githubusercontent.com/risaavedraf/aiharness/main/install.sh | sh

# Specific version
HARNESS_VERSION=0.2.0 curl -sSf https://raw.githubusercontent.com/risaavedraf/aiharness/main/install.sh | sh
```

---

### Docker (containerized)

For environments where installing binaries isn't preferred:

```dockerfile
FROM debian:bookworm-slim
COPY cite /usr/local/bin/cite
RUN chmod +x /usr/local/bin/cite
ENTRYPOINT ["cite"]
```

**Usage:**

```bash
# Build
docker build -t cite .

# Run
docker run cite health --json
docker run -v ./docs:/docs cite ingest /docs/readme.md
docker run cite context "what is this project about?"
```

**Or from GitHub Container Registry:**

```bash
docker pull ghcr.io/risaavedraf/cite:latest
docker run ghcr.io/risaavedraf/cite:latest context "query" --json
```

---

### apt (Debian/Ubuntu)

For Debian-based systems, a `.deb` package:

**Installation:**

```bash
# Add the repository
curl -sSf https://packages.harness.dev/gpg.key | sudo apt-key add -
echo "deb https://packages.harness.dev stable main" | sudo tee /etc/apt/sources.list.d/cite.list

# Install
sudo apt update
sudo apt install cite
```

**Building the .deb (CI):**

```bash
# Using cargo-deb
cargo install cargo-deb
cargo deb --release
# Output: target/debian/cite_0.1.0_amd64.deb
```

---

## Comparison

| Method | Platform | Auto-update | Prerequisites | Status |
|---|---|---|---|---|
| Manual download | All | No | curl/wget | ✅ Available |
| Scoop | Windows | Yes | Scoop | 📋 Planned |
| Homebrew | macOS/Linux | Yes | Homebrew | 📋 Planned |
| Cargo install | All | No | Rust toolchain | 📋 Planned |
| Install script | Linux/macOS | No | curl | 📋 Planned |
| Docker | All | Pull latest | Docker | 📋 Planned |
| apt | Debian/Ubuntu | Yes | apt | 📋 Planned |

## Post-install setup

> Phase 8 note: runtime configuration names remain `HARNESS_*` for now. The migration to `CITE_*` and data/db path renaming is deferred to Phase 9.
>
> See `docs/sdd/phase-8-rename-cite/migration-checklist.md` for the local migration checklist.

After installing, configure the embedding provider:

```bash
# Copy the example config
cp .env.example .env

# Edit with your API key
# HARNESS_EMBEDDING_API_KEY=your-key-here

# Or set environment variables directly
export HARNESS_EMBEDDING_API_KEY=your-key-here
export HARNESS_EMBEDDING_PROVIDER=gemini

# Verify
cite health --json
```

## Uninstall

### Manual

```bash
sudo rm /usr/local/bin/cite
rm -rf $HARNESS_DATA_DIR
```

### Scoop

```powershell
scoop uninstall cite
scoop bucket rm cite
```

### Homebrew

```bash
brew uninstall cite
brew untap risaavedraf/cite
```
