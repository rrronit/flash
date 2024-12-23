#!/bin/bash

echo "Compiling and running the code..." > app.log
# temp_dir=$0
# language=$1
# content=$2
# input=$3

# # Create the sandbox directory
# mkdir -p "$temp_dir" || { echo "Error: Failed to create temporary directory."; exit 1; }


# case "$language" in
#     cpp)
#         printf '%s' "$content" > "$temp_dir/temp.cpp"
#         g++ "$temp_dir/temp.cpp" -o "$temp_dir/temp.out" || { echo "Compilation failed."; exit 1; }
#         echo "$input" | "$temp_dir/temp.out"
#         ;;
#     c)
#         printf '%s' "$content" > "$temp_dir/temp.c"
#         gcc "$temp_dir/temp.c" -o "$temp_dir/temp.out" || { echo "Compilation failed."; exit 1; }
#         echo "$input" | "$temp_dir/temp.out"
#         ;;
#     java)
#         printf '%s' "$content" > "$temp_dir/temp.java"
#         javac "$temp_dir/temp.java" || { echo "Compilation failed."; exit 1; }
#         class_name=$(grep -oP 'class\s+\K\w+' "$temp_dir/temp.java" | head -n 1)
#         if [ -z "$class_name" ]; then
#             echo "Error: Could not determine Java class name."
#             exit 1
#         fi
#         echo "$input" | java -cp "$temp_dir" "$class_name"
#         ;;
#     python)
#         printf '%s' "$content" > "$temp_dir/temp.py"
#         echo "$input" | python3 "$temp_dir/temp.py" || { echo "Execution failed."; exit 1; }
#         ;;
#     javascript)
#         printf '%s' "$content" > "$temp_dir/temp.js"
#         echo "$input" | node "$temp_dir/temp.js" || { echo "Execution failed."; exit 1; }
#         ;;
#     *)
#         echo "Unsupported language: $language"
#         exit 1
#         ;;
# esac
