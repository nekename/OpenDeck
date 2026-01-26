#!/usr/bin/env bash
set -euo pipefail

GITHUB_REPO="nekename/OpenDeck"
FLATHUB_APP_ID="me.amankhanna.opendeck"
UDEV_RULES_URL="https://raw.githubusercontent.com/nekename/OpenDeck/refs/heads/release/src-tauri/bundle/40-streamdeck.rules"

if [ -t 1 ]; then
    RED="\033[0;31m"
    GREEN="\033[0;32m"
    YELLOW="\033[0;33m"
    BLUE="\033[0;34m"
    BOLD="\033[1m"
    RESET="\033[0m"
else
    RED=""
    GREEN=""
    YELLOW=""
    BLUE=""
    RESET=""
fi

trap 'echo -e "${YELLOW}Need help? Join the Discord: ${BLUE}https://discord.gg/26Nf8rHvaj${RESET}"' EXIT

msg_info() { echo -e "${BLUE}[*]${RESET} $*"; }
msg_ok() { echo -e "${GREEN}[✓] $*${RESET}"; }
msg_warn() { echo -e "${YELLOW}[!] $*${RESET}"; }
msg_error() { echo -e "${RED}[✗] $*${RESET}" >&2; }

has_cmd() { command -v "$1" >/dev/null 2>&1; }

confirm() {
    printf "%b$1 [y/N]: %b" "$YELLOW$BOLD" "$RESET"
    read -r ans
    case "$ans" in
    [yY] | [yY][eE][sS]) return 0 ;;
    *) return 1 ;;
    esac
}

detect_family() {
    if has_cmd rpm-ostree || grep -qi "universal blue" /etc/os-release 2>/dev/null; then
        echo "ublue"
    elif has_cmd pacman; then
        echo "arch"
    elif has_cmd zypper || has_cmd dnf || has_cmd rpm; then
        echo "rpm"
    elif has_cmd apt-get || has_cmd dpkg; then
        echo "debian"
    else
        echo "unknown"
    fi
}

fetch_latest_asset() {
    local ext="$1"
    local api_url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
    local arch arch_deb arch_rpm arch_pattern download_url

    arch="$(uname -m)"
    case "$arch" in
    x86_64)
        arch_deb="amd64"
        arch_rpm="x86_64"
        ;;
    aarch64)
        arch_deb="arm64"
        arch_rpm="aarch64"
        ;;
    *)
        msg_error "Unsupported architecture $arch"
        exit 1
        ;;
    esac

    if [ "$ext" = "deb" ]; then
        arch_pattern="${arch_deb}"
    else
        arch_pattern="${arch_rpm}"
    fi

    if has_cmd curl; then
        download_url=$(curl -s --retry 3 --retry-delay 3 "$api_url" |
            grep -Eo "https://[^ \"]+${arch_pattern}\.${ext}([^\"]*)" |
            head -n1 || true)
    elif has_cmd wget; then
        download_url=$(wget -qO- --tries=3 "$api_url" |
            grep -Eo "https://[^ \"]+${arch_pattern}\.${ext}([^\"]*)" |
            head -n1 || true)
    fi

    if [ -n "$download_url" ]; then
        echo "$download_url"
    else
        msg_error "Failed to find a .${ext} release asset for arch $arch"
        return 1
    fi
}

download_with_retry() {
    local url="$1"
    local dest="$2"

    if has_cmd curl; then
        curl -L --retry 3 --retry-delay 3 -o "$dest" "$url"
    else
        wget --tries=3 -O "$dest" "$url"
    fi
}

reload_udev_rules() {
    msg_info "Reloading udev rules"
    if ! sudo udevadm control --reload-rules || ! sudo udevadm trigger; then
        msg_error "Failed to reload udev rules; you may be able to restart your computer instead"
        return 0
    fi
    msg_ok "Reloaded udev rules"
}

install_flatpak() {
    msg_info "Installing ${FLATHUB_APP_ID} from Flathub"
    if confirm "Install OpenDeck system-wide? (No = user install)"; then
        if ! flatpak remote-list | grep -q flathub; then
            msg_error "Flathub remote not found; please add Flathub before running this script"
            exit 1
        fi
        flatpak install flathub "${FLATHUB_APP_ID}"
    else
        if ! flatpak remote-list --user | grep -q flathub; then
            msg_error "Flathub remote not found; please add Flathub before running this script or try installing system-wide"
            exit 1
        fi
        flatpak install --user flathub "${FLATHUB_APP_ID}"
    fi
    msg_ok "Installed ${FLATHUB_APP_ID} from Flathub"

    msg_info "Installing udev rules"
    tmp_rules="$(mktemp --suffix=.rules)"
    download_with_retry "$UDEV_RULES_URL" "$tmp_rules"
    sudo mv "$tmp_rules" /etc/udev/rules.d/40-streamdeck.rules
    sudo chmod 644 /etc/udev/rules.d/40-streamdeck.rules
    msg_ok "Installed udev rules"

    reload_udev_rules
}

install_deb() {
    local dl tmpf
    dl=$(fetch_latest_asset "deb")
    msg_info "Downloading ${dl##*/}"
    tmpf="$(mktemp --suffix=.deb)"
    download_with_retry "$dl" "$tmpf"
    msg_ok "Downloaded ${dl##*/}"

    msg_info "Installing .deb package"
    sudo apt-get install --fix-broken "$tmpf"
    rm -f "$tmpf"
    msg_ok "Installed .deb package"

    reload_udev_rules
}

install_rpm() {
    local dl tmpf
    dl=$(fetch_latest_asset "rpm")
    msg_info "Downloading ${dl##*/}"
    tmpf="$(mktemp --suffix=.rpm)"
    download_with_retry "$dl" "$tmpf"
    msg_ok "Downloaded ${dl##*/}"

    msg_info "Installing .rpm package"
    if has_cmd zypper; then
        sudo zypper install --allow-unsigned-rpm "$tmpf"
    elif has_cmd dnf; then
        sudo dnf install --nogpgcheck "$tmpf"
    else
        sudo rpm -i --nosignature "$tmpf"
    fi
    rm -f "$tmpf"
    msg_ok "Installed .rpm package"

    reload_udev_rules
}

install_aur() {
    msg_info "Installing from AUR"
    msg_info "${BOLD}This script will attempt to use yay, paru, aura, pikaur, or trizen, in that order"
    confirm "If you use another AUR helper, you should install OpenDeck manually. Continue?"

    if has_cmd yay; then
        yay -Sy opendeck
    elif has_cmd paru; then
        paru -Sy opendeck
    elif has_cmd aura; then
        aura -Ak opendeck
    elif has_cmd pikaur; then
        pikaur -Sy opendeck
    elif has_cmd trizen; then
        trizen -Sy opendeck
    else
        msg_error "No AUR helper found; install yay, paru, aura, pikaur, or trizen, or install manually"
        return 1
    fi
    msg_ok "Installed from AUR"

    reload_udev_rules
}

install_wine_if_needed() {
    if has_cmd wine; then
        msg_info "Wine already installed"
        return
    fi

    if confirm "Wine is required for some plugins. If you're not sure if you need to install Wine, you can run this script again later. Install Wine now?"; then
        msg_info "Installing Wine"
        case "$PKG_FAMILY" in
        debian)
            sudo apt-get update && sudo apt-get install wine
            ;;
        rpm | ublue)
            if has_cmd rpm-ostree; then
                sudo rpm-ostree install wine wine-mono
            elif has_cmd zypper; then
                sudo zypper install wine wine-mono
            elif has_cmd dnf; then
                sudo dnf install wine wine-mono
            else
                msg_error "No supported package manager found to install Wine; please install it manually"
            fi
            ;;
        arch)
            sudo pacman -Sy wine wine-mono
            ;;
        *)
            msg_error "No supported package manager found to install Wine; please install it manually"
            ;;
        esac
        msg_ok "Installed Wine"
    else
        msg_warn "Not installing Wine"
    fi
}

install_node_if_needed() {
    if has_cmd node; then
        msg_info "Node.js already installed"
        return
    fi

    if confirm "Node.js is required for some plugins. If you're not sure if you need to install Node.js, you can run this script again later. Install Node.js now?"; then
        msg_info "Installing Node.js"
        case "$PKG_FAMILY" in
        debian)
            sudo apt-get update && sudo apt-get install nodejs npm
            ;;
        rpm | ublue)
            if has_cmd rpm-ostree; then
                sudo rpm-ostree install nodejs npm
            elif has_cmd zypper; then
                sudo zypper install nodejs npm
            elif has_cmd dnf; then
                sudo dnf install nodejs npm
            else
                msg_error "No supported package manager found to install Node.js; please install it manually"
            fi
            ;;
        arch)
            sudo pacman -Sy nodejs npm
            ;;
        *)
            msg_error "No supported package manager found to install Node.js; please install it manually"
            ;;
        esac
        msg_ok "Installed Node.js"
    else
        msg_warn "Not installing Node.js"
    fi
}

if [ -n "${OPENDECK_PKG_FAMILY:-}" ]; then
    PKG_FAMILY="$OPENDECK_PKG_FAMILY"
    case "$PKG_FAMILY" in
    debian | rpm | arch | ublue)
        msg_info "Using package family '${PKG_FAMILY}' from the OPENDECK_PKG_FAMILY environment variable"
        ;;
    *)
        msg_error "Invalid OPENDECK_PKG_FAMILY value '${PKG_FAMILY}'"
        msg_error "Supported values: debian, rpm, arch, ublue"
        exit 1
        ;;
    esac
else
    PKG_FAMILY="$(detect_family)"
    msg_info "Detected '${PKG_FAMILY}' package family (set the OPENDECK_PKG_FAMILY environment variable to debian, rpm, arch, or ublue to override)"
fi

case "$PKG_FAMILY" in
debian)
    install_deb
    ;;
rpm)
    install_rpm
    ;;
arch)
    install_aur
    ;;
ublue)
    install_flatpak
    ;;
unknown)
    if has_cmd flatpak; then
        msg_warn "No native package method found"
        msg_info "You can continue by installing with Flatpak; if you experience issues, manually install OpenDeck natively"
        if confirm "Install with Flatpak?"; then
            install_flatpak
        else
            msg_error "Installation aborted"
            exit 1
        fi
    else
        msg_error "No usable installation method found; please install OpenDeck manually"
        exit 1
    fi
    ;;
esac

install_wine_if_needed
install_node_if_needed

msg_ok "Installation complete!"
echo -e "${YELLOW}If you enjoy OpenDeck, please consider starring the project on GitHub: ${BLUE}https://github.com/${GITHUB_REPO}${RESET}"

if confirm "Launch OpenDeck now?"; then
    if [[ "$PKG_FAMILY" == "ublue" ]] || { [[ "$PKG_FAMILY" == "unknown" ]] && has_cmd flatpak; }; then
        flatpak run "${FLATHUB_APP_ID}" &
        msg_ok "Launched OpenDeck using Flatpak"
    else
        if [ -x /bin/opendeck ]; then
            /bin/opendeck &
            msg_ok "Launched OpenDeck from /bin"
        elif has_cmd opendeck; then
            opendeck &
            msg_ok "Launched OpenDeck from PATH"
        else
            msg_warn "OpenDeck executable not found"
        fi
    fi
fi
