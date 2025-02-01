import sys
import json
import bdb
import io
from contextlib import redirect_stdout
import types

class Debugger(bdb.Bdb):
    def __init__(self):
        super().__init__()
        self.steps = []
        self.stdout_capture = io.StringIO()
        self.current_stdout = ""
        
    def user_line(self, frame):
        # Get the current line of code
        filename = frame.f_code.co_filename
        line_no = frame.f_lineno
        
        # Get the actual line of code
        with open(filename, 'r') as f:
            lines = f.readlines()
            current_line = lines[line_no - 1].strip()
        
        # Get local variables
        locals_dict = {
            key: value for key, value in frame.f_locals.items()
            if not key.startswith('__') and not isinstance(value, (type, types.ModuleType, types.FunctionType))
        }
        
        # Capture any new stdout content
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
    """Sanitize a dictionary to make it JSON serializable."""
    sanitized = {}
    for key, value in d.items():
        if key.startswith('__'):
            continue
        if isinstance(value, (types.ModuleType, types.FunctionType, types.BuiltinFunctionType)):
            continue
        elif isinstance(value, (int, float, str, bool, list, dict, tuple, type(None))):
            sanitized[key] = value
        else:
            try:
                sanitized[key] = str(value)
            except:
                continue
    return sanitized


def debug_file(filename):
    debugger = Debugger()
    
    # Redirect stdout to capture print statements
    with redirect_stdout(debugger.stdout_capture):
        try:
            # Run the file under the debugger
            with open(filename, 'r') as f:
                code = compile(f.read(), filename, 'exec')
                debugger.run(code)
        except:
            pass  # Ignore any errors in the target file
    
    # Return the collected debug information
    return {
        "steps": debugger.steps
    }

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python3 debug.py <filename>")
        sys.exit(1)
        
    filename = sys.argv[1]
    result = debug_file(filename)
    print(json.dumps(result, indent=2))