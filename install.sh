#!/bin/sh
set -eu

repo=${1:-${ASKLLM_REPO:-akzin/askllm}}
version=${ASKLLM_VERSION:-latest}
bin_dir=${ASKLLM_BIN_DIR:-"$HOME/.local/bin"}
config_home=${XDG_CONFIG_HOME:-$HOME/.config}
config_dir=${ASKLLM_CONFIG_DIR:-"$config_home/askllm"}
config_file="$config_dir/config.toml"
example_ref=$version
if [ "$example_ref" = "latest" ]; then
    example_ref=master
fi
example_url="https://raw.githubusercontent.com/$repo/$example_ref/askllm.toml.example"

case "$(uname -s)" in
    Linux) ;;
    *)
        printf '%s\n' "askllm releases currently support Linux only." >&2
        exit 1
        ;;
esac

case "$(uname -m)" in
    x86_64|amd64) ;;
    *)
        printf '%s\n' "askllm releases currently support Linux x86_64 only." >&2
        exit 1
        ;;
esac

if [ "$version" = "latest" ]; then
    release_base="https://github.com/$repo/releases/latest/download"
else
    release_base="https://github.com/$repo/releases/download/$version"
fi
url="$release_base/askllm?cachebust=$(date +%s)"
checksum_url="$release_base/SHA256SUMS"

tmp_file=$(mktemp)
checksum_file=
cleanup() {
    rm -f "$tmp_file" "$checksum_file"
}
trap cleanup EXIT HUP INT TERM

printf 'Downloading askllm from %s\n' "$url"
curl --fail --location --silent --show-error "$url" --output "$tmp_file"

checksum_file=$(mktemp)
if curl --fail --location --silent --show-error "$checksum_url" --output "$checksum_file" 2>/dev/null; then
    expected=$(awk '$2 == "askllm" { print $1; exit }' "$checksum_file")
    if [ -n "$expected" ]; then
        actual=$(sha256sum "$tmp_file" | awk '{ print $1 }')
        if [ "$actual" != "$expected" ]; then
            rm -f "$tmp_file" "$checksum_file"
            printf '%s\n' 'Checksum mismatch for askllm.' >&2
            exit 1
        fi
    else
        printf '%s\n' 'Warning: SHA256SUMS does not contain askllm; skipping verification.' >&2
    fi
else
    printf '%s\n' 'Warning: could not download SHA256SUMS; skipping verification.' >&2
fi
rm -f "$checksum_file"
mkdir -p "$bin_dir"
cp "$tmp_file" "$bin_dir/askllm"
chmod 755 "$bin_dir/askllm"

printf 'Installed askllm to %s/askllm\n' "$bin_dir"
case ":${PATH:-}:" in
    *":$bin_dir:"*) ;;
    *) printf 'Add %s to PATH if needed: export PATH="%s:$PATH"\n' "$bin_dir" "$bin_dir" ;;
esac

case "${ASKLLM_NO_CONFIG:-0}" in
    1|true|yes) ;;
    *)
        if [ ! -e "$config_file" ] && [ ! -L "$config_file" ]; then
            if mkdir -p "$config_dir" 2>/dev/null &&
                curl --fail --location --silent --show-error \
                    "$example_url" --output "$config_file.tmp" 2>/dev/null; then
                if [ -e "$config_file" ] || [ -L "$config_file" ]; then
                    rm -f "$config_file.tmp"
                elif mv "$config_file.tmp" "$config_file" 2>/dev/null; then
                    printf 'Created %s\n' "$config_file"
                    printf '%s\n' 'Edit it: set base_url and model before running askllm.'
                else
                    rm -f "$config_file.tmp"
                    printf 'Could not create %s.\n' "$config_file" >&2
                    printf 'Create it from %s\n' "$example_url" >&2
                fi
            else
                rm -f "$config_file.tmp"
                printf 'Could not create %s.\n' "$config_file" >&2
                printf 'Create it from %s\n' "$example_url" >&2
            fi
        fi
        ;;
esac
