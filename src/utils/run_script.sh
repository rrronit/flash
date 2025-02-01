#!/bin/bash
# run.sh
case "$LANGUAGE" in
    "python")
        python3 code.py
        ;;
    "cpp"|"c")
        ./program
        ;;
    *)
        echo "Unsupported language"
        exit 1
        ;;
esac