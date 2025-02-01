#!/bin/bash
# compile.sh
case "$LANGUAGE" in
    "python")
        python3 -m py_compile code.py
        ;;
    "cpp")
        g++ -O2 -w -std=c++17 code.cpp -o program
        ;;
    "c")
        gcc -O2 -w -std=c11 code.c -o program
        ;;
    *)
        echo "Unsupported language"
        exit 1
        ;;
esac
