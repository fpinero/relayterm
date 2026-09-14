#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "usage: install_release.sh EXTRACTED_RELEASE_ROOT DESTINATION_DIRECTORY" >&2
    exit 2
fi

release_root=$1
destination=$2
source_binary="$release_root/rt"
destination_binary="$destination/rt"

if [ -L "$release_root" ] || [ ! -f "$source_binary" ] || [ -L "$source_binary" ] || [ ! -x "$source_binary" ]; then
    echo "installation refused: the source rt is not a regular executable" >&2
    exit 1
fi
for required in LICENSE THIRD_PARTY_NOTICES.txt THIRD_PARTY_LICENSES.txt INSTALL.md RECOVERY.md manifest.json install_release.sh install_release.ps1; do
    if [ ! -f "$release_root/$required" ] || [ -L "$release_root/$required" ]; then
        echo "installation refused: the extracted release inventory is incomplete" >&2
        exit 1
    fi
done
if [ -e "$destination_binary" ] || [ -L "$destination_binary" ]; then
    echo "installation refused: the destination rt already exists" >&2
    exit 1
fi
if [ ! -d "$destination" ] || [ -L "$destination" ]; then
    echo "installation refused: the destination directory must already exist" >&2
    exit 1
fi

temporary="$destination/.rt-install-$$"
if [ -e "$temporary" ] || [ -L "$temporary" ]; then
    echo "installation refused: a private staging name already exists" >&2
    exit 1
fi
trap 'rm -f "$temporary"' EXIT HUP INT TERM
umask 077
cp "$source_binary" "$temporary"
chmod 755 "$temporary"
mv "$temporary" "$destination_binary"
trap - EXIT HUP INT TERM
printf '%s\n' "installed: $destination_binary"
