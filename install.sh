#!/bin/bash
# install.sh — Install cite CLI
set -e

REPO="risaavedraf/aicite"
GITHUB_API="https://api.github.com/repos/${REPO}/releases"

# Colors (disabled if not a terminal)
if [ -t 1 ]; then
	RED='\033[0;31m'
	GREEN='\033[0;32m'
	YELLOW='\033[0;33m'
	BOLD='\033[1m'
	RESET='\033[0m'
else
	RED='' GREEN='' YELLOW='' BOLD='' RESET=''
fi

info() { echo -e "${GREEN}✓${RESET} $*"; }
warn() { echo -e "${YELLOW}⚠${RESET} $*" >&2; }
error() {
	echo -e "${RED}✗${RESET} $*" >&2
	exit 1
}

detect_platform() {
	local os arch
	os=$(uname -s | tr '[:upper:]' '[:lower:]')
	arch=$(uname -m)

	case "$os" in
	linux) os="linux" ;;
	darwin) os="macos" ;;
	*)
		error "Unsupported OS: $os. Download manually from https://github.com/${REPO}/releases"
		;;
	esac

	case "$arch" in
	x86_64) arch="amd64" ;;
	aarch64 | arm64) arch="arm64" ;;
	*)
		error "Unsupported architecture: $arch. Download manually from https://github.com/${REPO}/releases"
		;;
	esac

	echo "${os}-${arch}"
}

# Resolve the version to install.
# Priority: CITE_VERSION env var > latest GitHub release.
resolve_version() {
	if [ -n "${CITE_VERSION:-}" ]; then
		echo "$CITE_VERSION"
		return
	fi

	# Fetch the latest release tag from GitHub API (no jq needed)
	local tag
	tag=$(curl -sSfL "$GITHUB_API/latest" 2>/dev/null |
		grep '"tag_name"' |
		head -1 |
		sed -E 's/.*"tag_name":\s*"v?([^"]+)".*/\1/')

	if [ -z "$tag" ]; then
		error "Could not determine latest release version. Set CITE_VERSION=<version> and retry."
	fi

	echo "$tag"
}

# Download a file and verify its SHA256 checksum if a checksums file is available.
download_and_verify() {
	local url="$1"
	local dest="$2"
	local filename="$3"
	local version="$4"

	# Download the binary
	curl -sSfL "$url" -o "$dest"
	chmod +x "$dest"

	# Try to fetch and verify SHA256 checksum
	local checksums_url="${GITHUB_API}/download/v${version}/checksums-${version}.txt"
	local checksums
	checksums=$(curl -sSfL "$checksums_url" 2>/dev/null || true)

	if [ -n "$checksums" ]; then
		local expected
		expected=$(echo "$checksums" | grep "$filename" | awk '{print $1}')
		if [ -n "$expected" ]; then
			local actual
			actual=$(sha256sum "$dest" | awk '{print $1}')
			if [ "$actual" != "$expected" ]; then
				rm -f "$dest"
				error "Checksum verification failed for $filename. Expected: $expected, Got: $actual"
			fi
			info "Checksum verified"
		else
			warn "No checksum entry found for $filename (skipping verification)"
		fi
	else
		warn "Checksums file not available (skipping verification)"
	fi
}

main() {
	local platform version install_dir url filename tmp_file
	platform=$(detect_platform)
	version=$(resolve_version)
	install_dir="${INSTALL_DIR:-/usr/local/bin}"
	filename="cite-${platform}"
	url="https://github.com/${REPO}/releases/download/v${version}/${filename}"

	echo -e "${BOLD}Installing cite v${version} for ${platform}...${RESET}"
	echo "Download: ${url}"

	# Download to a temporary file first (atomic install)
	tmp_file=$(mktemp "${install_dir}/.cite-install.XXXXXX")
	trap 'rm -f "$tmp_file"' EXIT

	download_and_verify "$url" "$tmp_file" "$filename" "$version"

	# Move into place
	mv "$tmp_file" "$install_dir/cite"
	trap - EXIT

	info "Installed: ${install_dir}/cite"

	# Verify installation
	if command -v cite >/dev/null 2>&1 || [ -x "${install_dir}/cite" ]; then
		"${install_dir}/cite" health --json 2>/dev/null || true
	fi

	# Offer setup (skip in non-interactive mode or CI)
	if [ -n "${NONINTERACTIVE:-}" ] || [ ! -t 0 ]; then
		info "Run 'cite setup' to configure your embedding provider."
		return
	fi

	echo ""
	read -r -p "Run cite setup now? [Y/n] " answer
	case "$answer" in
	[nN] | [nN][oO]) echo "Skipping setup. Run 'cite setup' later to configure." ;;
	*) "${install_dir}/cite" setup ;;
	esac
}

main "$@"
