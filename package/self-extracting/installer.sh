#!/bin/bash

DEST="${PREFIX:=/opt/bottled-shell}"
echo "installing bottled-shell into $DEST"

mkdir -p ${DEST}

ARCHIVE=$(awk '/^__ARCHIVE__/ {print NR + 1; exit 0; }' "${0}")
tail -n+${ARCHIVE} "${0}" | tar -xpv -C ${DEST}

echo "done"

exit 0

__ARCHIVE__
