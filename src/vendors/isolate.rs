use crate::{
    client::redis::RedisClient,
    core::{Job, JobError, JobStatus},
};
use std::{fs::File, io::Write, path::PathBuf};
use tokio::process::Command;

#[derive(Clone)]
pub struct IsolateExecutor {
    redis: RedisClient,
}

impl IsolateExecutor {
    pub fn new(redis: RedisClient) -> Self {
        Self { redis }
    }

    pub async fn execute(&self, job: &mut Job) -> Result<(), JobError> {
        let box_id = job.id % 1000;
        let _box_path = PathBuf::from(format!("/var/lib/isolate/{}", box_id));

        println!("Executing job: {}", job.id);

        // Cleanup previous box
        let _ = Command::new("isolate")
            .args(&["-b", &box_id.to_string(), "--cleanup"])
            .status()
            .await;

        // Initialize new box
        let init_output = Command::new("isolate")
            .args(&["-b", &box_id.to_string(), "--init"])
            .output()
            .await
            .unwrap();

        println!("Box initialized: {}", box_id);

        let box_path = String::from_utf8_lossy(&init_output.stdout)
            .trim()
            .to_string();

        println!("Box path: {}", box_path);

        let file_path = format!("{}/box", box_path);
        let stdin_file = format!("{}/stdin", file_path);
        let stdout_file = format!("{}/stdout", file_path);
        let stderr_file = format!("{}/stderr", file_path);

        // Write source code
        let source_path = format!("{}/{}", file_path, job.language.source_file);

        tokio::fs::write(&source_path, &job.source_code)
            .await
            .unwrap();
        tokio::fs::write(&stdin_file, &job.stdin).await.unwrap();

        let stdin_file = File::open(stdin_file).unwrap();
        let stdout_file = File::create(stdout_file).unwrap();
        let stderr_file = File::create(stderr_file).unwrap();

        println!("Source code written to: {}", source_path);

        // Run compilation if needed
        if let Some(compile_cmd) = &job.language.compile_cmd {
            let compile_status = Command::new("isolate")
                .args(&[
                    "-b",
                    &box_id.to_string(),
                    "--run",
                    "--",
                    "sh",
                    "-c",
                    compile_cmd,
                ])
                .status()
                .await
                .unwrap();

            if !compile_status.success() {
                job.status = JobStatus::Failed("Compilation failed".into());
                self.update_job_in_redis(job).await?;
                return Err(JobError::CompilationError("Compilation failed".into()));
            }
        }

        println!("Compilation successful");

        let lang = &job.language.run_cmd.split(" ").collect::<Vec<&str>>();
        // Execute the program
        let output = Command::new("isolate")
            .args(&[
                "-b",
                &box_id.to_string(),
                "-t",
                &job.settings.cpu_time_limit.to_string(),
                "-m",
                &(job.settings.memory_limit / 1024).to_string(),
                "--run",
                "--",
                lang[0],
                lang[1],
            ])
            .stdin(stdin_file)
            .stdout(stdout_file)
            .stderr(stderr_file)
            .output()
            .await
            .unwrap();

        println!("Execution completed");
        println!("Exit code: {:?} {:?}",box_id, output.status.code());

        // Process output
        job.output.stdout = Some(String::from_utf8_lossy(&output.stdout).into());
        job.output.stderr = Some(String::from_utf8_lossy(&output.stderr).into());
        job.output.exit_code = output.status.code();

        if output.status.success() {
            job.status = JobStatus::Completed;
        } else {
            job.status = JobStatus::Failed("Runtime error".into());
        }

        // Update job in Redis
        self.update_job_in_redis(job).await?;
        
        let _ = Command::new("isolate")
        .args(&["-b", &box_id.to_string(), "--cleanup"])
        .status()
        .await;


        Ok(())
    }

    async fn update_job_in_redis(&self, job: &Job) -> Result<(), JobError> {
        let job_id = job.id.to_string();
        self.redis
            .store_job(&job_id, job, None)
            .await
            .map_err(|e| JobError::RuntimeError(format!("Failed to update job in Redis: {}", e)))
    }
}
