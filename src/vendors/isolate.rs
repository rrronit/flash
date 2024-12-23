use std::{
    fs::{self, Permissions},
    process::{Command, Output},
    time,
};

use rand::Rng;

use crate::core::pool::Code;

pub struct Isolate {
    code: Code,
    isolate_dir: String,
    box_dir: String,
    tmp_dir: String,
}

impl Isolate {
    /// Creates a new `Isolate` instance with the specified program name.
    pub fn new(code: Code) -> Self {
        //generate random id
        let output = Command::new("isolate")
            .arg("--cg")
            .arg("-b")
            .arg(&code._id.to_string())
            .arg("--init")
            .output()
            .expect("Failed to initialize isolate");

        let path_output = String::from_utf8_lossy(&output.stdout);
        let isolate_dir = path_output.trim().to_string();
        let box_dir = format!("{}/box", isolate_dir);
        let tmp_dir = format!("{}/tmp", isolate_dir);
        println!("Isolate dir: {}", isolate_dir);
        println!("Box dir: {}", box_dir);
        println!("tmp_dir: {}", tmp_dir);

        // Copy compile.sh to the sandbox
        let compile_script_path = format!("{}/compile.sh", box_dir);
        fs::copy("/api/src/utils/compile.sh", &compile_script_path)
            .expect("Failed to copy compile script");

        // Set the executable permission for the compile script
        Command::new("chown")
            .arg("${whoami}")
            .arg(&compile_script_path)
            .output()
            .expect("Failed to change permission");

        Self {
            code,
            isolate_dir,
            box_dir,
            tmp_dir,
        }
    }

    /// Compile the user's code inside the sandbox.
    pub fn compile(&self) -> Result<Output, String> {
        println!("{}/compile.sh", self.box_dir);
        let output = Command::new("isolate")
            .arg("-v")
            .arg("-b")
            .arg(&self.code._id.to_string())
            .arg("--stderr-to-stdout")
            .arg("-i")
            .arg("/dev/null")
            .arg("--cg")
            .arg("-E")
            .arg("HOME=/tmp")
            .arg("-E")
            .arg("PATH=\"/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"")
            .arg("-E")
            .arg("LANG=en_US.UTF-8")
            .arg("-E")
            .arg("LANGUAGE=en_US:en")
            .arg("--run")
            .arg("--")
            .arg("/bin/echo")
            .arg("Hello, World!")
            .output()
            .map_err(|e| format!("Failed to execute compile command: {}", e))?;

        println!("Compile output: {:?}", output);

        if !output.status.success() {
            return Err(format!(
                "Compilation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(output)
    }

    /// Run the compiled code inside the sandbox.
    pub fn run(&self) -> Result<Output, String> {
        let run_script = fs::read_to_string("./src/utils/run_script.sh")
            .map_err(|e| format!("Failed to read run script: {}", e))?;

        let output = Command::new("firejail")
            .arg("--net=none")
            .arg("--private")
            .arg("--private-tmp")
            .arg("--seccomp")
            .arg("bash")
            .arg("-c")
            .arg(&run_script)
            .arg(&self.code._id.to_string())
            .arg(&self.code._language)
            .arg(&self.code._input)
            .output()
            .map_err(|e| format!("Failed to execute run command: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Runtime error: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(output)
    }

    /// Cleanup the sandbox after execution.
    pub fn cleanup(&self) -> Result<(), String> {
        let output = Command::new("isolate")
            .arg("--cg")
            .arg("-b")
            .arg(&self.code._id.to_string())
            .arg("--cleanup")
            .output()
            .map_err(|e| format!("Failed to execute cleanup command: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Cleanup failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }
}
