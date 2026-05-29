#!/bin/bash
# install.sh — Instala cite CLI
set -e

VERSION="${CITE_VERSION:-0.2.0}"
REPO="risaavedraf/aicite"
BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"

detect_platform() {
	local os arch
	os=$(uname -s | tr '[:upper:]' '[:lower:]')
	arch=$(uname -m)

	case "$os" in
	linux) os="linux" ;;
	darwin) os="macos" ;;
	*)
		echo "Unsupported OS: $os" >&2
		exit 1
		;;
	esac

	case "$arch" in
	x86_64) arch="amd64" ;;
	aarch64 | arm64) arch="arm64" ;;
	*)
		echo "Unsupported architecture: $arch" >&2
		exit 1
		;;
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

	echo ""
	read -r -p "Run cite setup now? [Y/n] " answer
	case "$answer" in
	[nN] | [nN][oO]) echo "Skipping setup. Run 'cite setup' later to configure." ;;
	*) "${install_dir}/cite" setup ;;
	esac
}

main "$@"
