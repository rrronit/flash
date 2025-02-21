import sys
import json
import bdb
import io
from contextlib import redirect_stdout
import types
import os

class Debugger(bdb.Bdb):
    def __init__(self, user_code_length: int):
        super().__init__()
        self.steps = []
        self.stdout_capture = io.StringIO()
        self.current_stdout = ""
        self.user_code_length = user_code_length
        
    def user_line(self, frame):
        # Skip debugger internals or non-user code
        if (
            frame.f_code.co_filename != os.path.abspath("debugger/temp.py")  # Skip non-user files
            or frame.f_lineno > self.user_code_length  # Skip lines beyond user code
        ):
            return

        # Get the current line of code
        filename = frame.f_code.co_filename
        line_no = frame.f_lineno
        
        # Get the actual line of code
        with open(filename, 'r') as f:
            lines = f.readlines()
            current_line = lines[line_no - 1].strip()
        
        # Get local variables (filter out debugger internals)
        locals_dict = {
            key: value for key, value in frame.f_locals.items()
            if not key.startswith('__') 
            and not isinstance(value, (type, types.ModuleType, types.FunctionType))
        }
        
        # Capture stdout
        new_stdout = self.stdout_capture.getvalue()[len(self.current_stdout):]
        self.current_stdout = self.stdout_capture.getvalue()
        
        # Record the step
        self.steps.append({
            "line": line_no,
            "code": current_line,
            "locals": sanitize_dict(locals_dict),
            "stdout": new_stdout
        })

def sanitize_dict(d):
    """Sanitize to remove non-serializable values."""
    sanitized = {}
    for key, value in d.items():
        if isinstance(value, (int, float, str, bool, list, dict, type(None))):
            sanitized[key] = value
        else:
            sanitized[key] = str(value)
    return sanitized

def debug_file(filename):
    # First, get the length of the user's code
    with open(filename, 'r') as f:
        user_code = f.read()
        user_code_length = len(user_code.splitlines())

    debugger = Debugger(user_code_length)
    
    with redirect_stdout(debugger.stdout_capture):
        try:
            with open(filename, 'r') as f:
                code = compile(f.read(), filename, 'exec')
                debugger.run(code)
        except:
            pass  # Ignore errors in user code

    return {"steps": debugger.steps}

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python3 debug.py <filename>")
        sys.exit(1)
        
    filename = sys.argv[1]
    result = debug_file(filename)
    print(json.dumps(result, indent=2))