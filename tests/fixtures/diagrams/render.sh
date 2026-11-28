#!/bin/sh
# Requires macOS with Homebrew, Graphviz, PlantUML, D2, librsvg, ImageMagick and a Java JDK (javac).
set -eu
export LC_ALL=C
root=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
font="$root/../../fonts/NotoSans-Regular.ttf"
plant_jar="$(brew --prefix plantuml)/libexec/plantuml.jar"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/pixctl-corpus.XXXXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
mkdir -p "$tmp/cache" "$root/png"
# Isolate fontconfig so Graphviz and librsvg resolve the bundled font.
cat > "$tmp/fonts.conf" <<END
<?xml version="1.0"?><!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig><dir>$root/../../fonts</dir><cachedir>$tmp/cache</cachedir></fontconfig>
END
FONTCONFIG_FILE="$tmp/fonts.conf"
export FONTCONFIG_FILE
javac -cp "$plant_jar" -d "$tmp" "$root/PlantFont.java"
: > "$tmp/manifest.toml"
for source in "$root"/src/*; do
    name=${source##*/}
    base=${name%.*}
    size=${base##*-}
    case "$name" in
        *.dot) dot -Tpng "$source" > "$root/png/$base.png" ;;
        *.puml) java -Djava.awt.headless=true -cp "$tmp:$plant_jar" PlantFont "$font" -tpng -pipe < "$source" > "$root/png/$base.png" ;;
        *.d2)
            d2 --font-regular "$font" --font-bold "$font" --font-semibold "$font" "$source" "$tmp/diagram.svg" >/dev/null 2>&1
            # librsvg cannot use the embedded web font; resolve the same font
            # through the isolated fontconfig configuration instead.
            sed -E 's/font-family: "d2-[0-9]+-font-(regular|bold|semibold)"/font-family: "Noto Sans"/g' "$tmp/diagram.svg" > "$tmp/raster.svg"
            rsvg-convert "$tmp/raster.svg" -o "$root/png/$base.png" ;;
        *) echo "unknown source: $name" >&2; exit 1 ;;
    esac
    printf '[[image]]\nfile = "png/%s.png"\npx = %s.0\n\n' "$base" "$size" >> "$tmp/manifest.toml"
    magick "$root/png/$base.png" -resize 75% -strip "$root/png/$base-small.png"
    small=$(awk -v n="$size" 'BEGIN {printf "%.2f", n * 0.75}')
    printf '[[image]]\nfile = "png/%s-small.png"\npx = %s\n\n' "$base" "$small" >> "$tmp/manifest.toml"
    magick "$root/png/$base.png" -background white -alpha remove -alpha off -quality 80 -strip "$root/png/$base.jpg"
    printf '[[image]]\nfile = "png/%s.jpg"\npx = %s.0\n\n' "$base" "$size" >> "$tmp/manifest.toml"
done
mv "$tmp/manifest.toml" "$root/manifest.toml"
