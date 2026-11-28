#!/bin/sh
# Renders the calibration sources at a nominal em of 100 px with the same font
# and pipeline as render.sh, measures the cap height of the letter H and prints
# one line per tool. Requires the same tools as render.sh.
set -eu
export LC_ALL=C
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
root=$(dirname -- "$here")
fonts="$root/../../fonts"
font="$fonts/NotoSans-Regular.ttf"
plant_jar="$(brew --prefix plantuml)/libexec/plantuml.jar"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/pixctl-calibrate.XXXXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
mkdir -p "$tmp/cache"
cat > "$tmp/fonts.conf" <<END
<?xml version="1.0"?><!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig><dir>$fonts</dir><cachedir>$tmp/cache</cachedir></fontconfig>
END
FONTCONFIG_FILE="$tmp/fonts.conf"
export FONTCONFIG_FILE
javac -cp "$plant_jar" -d "$tmp" "$root/PlantFont.java"

dot -Tpng "$here/h.dot" > "$tmp/graphviz.png"
java -Djava.awt.headless=true -cp "$tmp:$plant_jar" PlantFont "$font" -tpng -pipe < "$here/h.puml" > "$tmp/plantuml.png"
d2 --font-regular "$font" --font-bold "$font" --font-semibold "$font" "$here/h.d2" "$tmp/d2.svg" > /dev/null
sed -E 's/font-family: "d2-[0-9]+-font-(regular|bold|semibold)"/font-family: "Noto Sans"/g' "$tmp/d2.svg" > "$tmp/d2-raster.svg"
rsvg-convert "$tmp/d2-raster.svg" -o "$tmp/d2.png"

# Cap height of Noto Sans: 714 units of 1000 (see font_metrics.py).
for tool in graphviz plantuml d2; do
    # Flatten on white, threshold at 50 %, and take the height of the dark area.
    rows=$(magick "$tmp/$tool.png" -background white -alpha remove -alpha off \
        -colorspace Gray -threshold 50% -trim +repage -format '%h' info:)
    awk -v tool="$tool" -v rows="$rows" 'BEGIN {
        em = rows / 0.714
        printf "%-8s cap height %3d px  implied em %6.2f px  error %+5.1f %%\n", tool, rows, em, (em - 100)
    }'
done
