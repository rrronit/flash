import gdb

class CppDebugger:
    def __init__(self, program):
        self.steps = []
        gdb.execute(f"file {program}")
        gdb.execute("set pagination off")
        gdb.execute("break main")
        gdb.execute("run")

    def step(self):
        gdb.execute("step")
        frame = gdb.selected_frame()
        
        # Check if frame.find_sal() is None
        sal = frame.find_sal()
        if sal is None:
            return
        
        # Check if sal.symtab is None
        if sal.symtab is None:
            return
        
        line = sal.line
        filename = sal.symtab.filename
        
        # Get local variables
        locals_dict = {}
        try:
            for symbol in frame.block():
                if symbol.is_variable:
                    locals_dict[symbol.name] = str(symbol.value(frame))
        except gdb.error:
            pass  # Ignore errors when accessing locals
        
        # Record the step
        self.steps.append({
            "line": line,
            "filename": filename,
            "locals": locals_dict
        })

    def run(self):
        while True:
            try:
                self.step()
            except gdb.error:
                break

    def get_steps(self):
        return self.steps

def main(program):
    debugger = CppDebugger(program)
    debugger.run()
    steps = debugger.get_steps()
    for step in steps:
        print(step)

# Read the program name from GDB's command line
if __name__ == "__main__":
    import sys
    
    program = "./debugger/a.out"
    main(program)