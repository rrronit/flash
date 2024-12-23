#!/bin/bash

box_id=$1
language=$2
temp_dir="$box_id/temp"

if [ "$language" == "cpp" ] || [ "$language" == "c" ]; then
    "$temp_dir/temp.out" 2> "$temp_dir/temp.err"
elif [ "$language" == "java" ]; then
    class_name=$(grep -oP 'class\s+\K\w+' "$temp_dir/temp.java" | head -n 1)
    if [ -z "$class_name" ]; then
        echo "Error: Could not determine Java class name."
        exit 1
    fi
    java -cp "$temp_dir" "$class_name" 2> "$temp_dir/temp.err"
elif [ "$language" == "python" ]; then
    python3 "$temp_dir/temp.py"
elif [ "$language" == "javascript" ]; then
    node "$temp_dir/temp.js"
else
    echo "Unsupported language: $language"
    exit 1
fi
