#!/bin/sh
set -eu

repo=${1:-${ASKLLM_REPO:-akzin/askllm}}
version=${ASKLLM_VERSION:-latest}
bin_dir=${ASKLLM_BIN_DIR:-"$HOME/.local/bin"}

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
    url="https://github.com/$repo/releases/latest/download/askllm"
else
    url="https://github.com/$repo/releases/download/$version/askllm"
fi

tmp_file=$(mktemp)
cleanup() {
    rm -f "$tmp_file"
}
trap cleanup EXIT HUP INT TERM

printf 'Downloading askllm from %s\n' "$url"
curl --fail --location --silent --show-error "$url" --output "$tmp_file"
mkdir -p "$bin_dir"
install -m 755 "$tmp_file" "$bin_dir/askllm"

printf 'Installed askllm to %s/askllm\n' "$bin_dir"
case ":${PATH:-}:" in
    *":$bin_dir:"*) ;;
    *) printf 'Add %s to PATH if needed: export PATH="%s:$PATH"\n' "$bin_dir" "$bin_dir" ;;
esac
printf '%s\n' 'Create ~/.config/askllm/config.toml before running askllm.'
