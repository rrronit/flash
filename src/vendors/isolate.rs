use crate::{
    client::redis::RedisClient,
    core::{Job, JobStatus},
};
use futures::TryFutureExt;
use redis::RedisError;
use std::{
    fs::{self, File},
    io::Error,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::process::Command;

#[derive(Debug)]
pub struct Metadata {
    pub time: f64,
    pub memory: u64,
    pub exit_code: i32,
    pub message: String,
    pub status: String,
}

#[derive(Clone)]
pub struct IsolateExecutor {
    redis: RedisClient,
}

impl IsolateExecutor {
    pub fn new(redis: RedisClient) -> Self {
        Self { redis }
    }
    pub async fn execute(&self, job: &mut Job) -> Result<JobStatus, Error> {
        let box_id = job.id % 2147483647;
        job.status = JobStatus::Processing;
        job.started_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .try_into()
                .unwrap(),
        );

        // Initialize new box
        let init_output = Command::new("isolate")
            .args(&["-b", &box_id.to_string(), "--cg", "--init"])
            .output()
            .map_err(|e| format!("Failed to initialize box: {:?}", e))
            .await
            .unwrap();

        let (file_path, metadata_file, stdin_file, stdout_file, stderr_file) =
                self.setup_files(job, &init_output.stdout)?;

        // Run compilation if needed
        if let Some(compile_cmd) = &job.language.compile_cmd {
            let compile_parts: Vec<&str> = compile_cmd.split_whitespace().collect();
            let compile_executable = compile_parts[0];
            let compile_args = &compile_parts[1..];

            let compile_status = Command::new("isolate")
                .args(&[
                    "--cg",
                    "-b",
                    &box_id.to_string(),
                    "-M",
                    metadata_file.as_str(),
                    "--process=60",
                    "-t",
                    "5",
                    "-x",
                    "0",
                    "-w",
                    "10",
                    "-k",
                    "12800",
                    "-f",
                    "1024",
                    format!("--cg-mem={}", job.settings.memory_limit.to_string()).as_str(),
                    "-E",
                    "PATH=\"/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"",
                    "-E",
                    "HOME=/tmp",
                    "-d",
                    "/etc:noexec",
                    "--run",
                    "--",
                    "/usr/bin/sh",
                    "-c",
                    format!(
                        "{} {} 2> /box/compile_output",
                        compile_executable,
                        compile_args.join(" ")
                    )
                    .as_str(),
                ])
                .output()
                .map_err(|e| format!("Failed to run compilation: {:?}", e))
                .await
                .unwrap();

            if !compile_status.status.success() {
                let compile_output = fs::read_to_string(format!("{}/compile_output", file_path))
                    .expect(format!("Failed to read compile output for job {}", job.id).as_str());

                job.output.compile_output = Some(compile_output);
                job.status = JobStatus::CompilationError;
                self.update_job_in_redis(job).await.unwrap();
                return Ok(JobStatus::CompilationError);
            }
        }

        let run_parts: Vec<&str> = job.language.run_cmd.split_whitespace().collect();
        let run_executable = run_parts[0];
        let run_args = &run_parts[1..];

        let _ = Command::new("isolate")
            .args(&[
                "--cg",
                "-b",
                &box_id.to_string(),
                "-M",
                &metadata_file,
                "--process=60",
                "-t",
                &job.settings.cpu_time_limit.to_string(),
                "-x",
                "0",
                "-w",
                "10",
                "-k",
                "128000",
                format!("--cg-mem={}", job.settings.memory_limit.to_string()).as_str(),
                "-E",
                "PATH=\"/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"",
                "-E",
                "HOME=/tmp",
                "-d",
                "/etc:noexec",
                "--run",
                "--",
                "/usr/bin/sh",
                "-c",
                format!(
                    "{} {} > /box/stdout 2> /box/stderr",
                    run_executable,
                    run_args.join(" ")
                )
                .as_str(),
            ])
            .stdin(stdin_file)
            .output()
            .map_err(|e| format!("Failed to run job: {:?}", e))
            .await
            .unwrap();

        let stdout_content = fs::read_to_string(&stdout_file).unwrap();
        let stderr_content = fs::read_to_string(&stderr_file).unwrap();

        job.output.stdout = Some(stdout_content);
        job.output.stderr = Some(stderr_content);

        let metadata = match self.get_metadata(box_id) {
            Ok(meta) => meta,
            Err(e) => {
                job.status = JobStatus::InternalError;
                self.update_job_in_redis(job).await.unwrap();
                return Ok(JobStatus::InternalError);
            }
        };

        job.finished_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .try_into()
                .unwrap(),
        );

        job.output.memory = Some(metadata.memory);
        job.output.time = Some(metadata.time);
        job.output.exit_code = Some(metadata.exit_code);
        job.output.message = Some(metadata.message);
        job.status = determine_status(
            metadata.status,
            metadata.exit_code,
            &job.output.stdout.clone().unwrap(),
            &job.expected_output,
        );

        self.update_job_in_redis(job).await.unwrap();

        Ok(job.status.clone())
    }

    async fn update_job_in_redis(&self, job: &Job) -> Result<(), RedisError> {
        let job_id = job.id.to_string();
        self.redis.store_job(&job_id, job, None).await.map_err(|_| {
            RedisError::from((redis::ErrorKind::IoError, "Failed to store job in Redis"))
        })
    }

    fn get_metadata(&self, box_id: u64) -> Result<Metadata,Error> {
        let metadata_file = format!("/var/local/lib/isolate/{}/box/metadata", box_id);
        let metadata = fs::read_to_string(metadata_file)
            .map_err(|_| JobStatus::RuntimeError)
            .map_err(|_| JobStatus::InternalError)
            .map_err(|_| Error::new(std::io::ErrorKind::Other, "Failed to read metadata"))?;

        let lines: Vec<&str> = metadata.lines().collect();

        let meta = lines.iter().map(|&pairs| {
            let mut parts = pairs.splitn(2, ':');
            let key = parts.next().unwrap();
            let value = parts.next().unwrap();
            (key, value)
        });

        let mut m: Metadata = Metadata {
            time: 0.0,
            memory: 0,
            exit_code: 0,
            message: "".to_string(),
            status: "".to_string(),
        };

        meta.for_each(|f| match f.0 {
            "time" => m.time = f.1.parse().unwrap(),
            "max-rss" => m.memory = f.1.parse().unwrap(),
            "cg-mem" => m.memory = f.1.parse().unwrap(),
            "exitcode" => m.exit_code = f.1.parse().unwrap(),
            "message" => m.message = f.1.to_string(),
            "status" => m.status = f.1.to_string(),
            _ => {}
        });

        Ok(m)
    }

    fn setup_files(
        self: &Self,
        job: &Job,
        stdout: &[u8],
    ) -> Result<(String, String, File, String, String), Error> {
        let box_path = String::from_utf8_lossy(&stdout).trim().to_string();
        let file_path = format!("{}/box", box_path);
        let stdin_file = format!("{}/stdin", file_path);
        let stdout_file = format!("{}/stdout", file_path);
        let stderr_file = format!("{}/stderr", file_path);
        let metadata_file = format!("{}/metadata", file_path);

        // Write source code
        let source_path = format!("{}/{}", file_path, job.language.source_file);

        fs::write(&source_path, &job.source_code)?;
        fs::write(&stdin_file, &job.stdin)?;

        let stdin_file = File::open(stdin_file)?;

        Ok((
            file_path,
            metadata_file,
            stdin_file,
            stdout_file,
            stderr_file,
        ))
    }
}

fn determine_status(
    status: String,
    exitcode: i32,
    stdout: &String,
    expected: &String,
) -> JobStatus {
    match status.as_str() {
        "TO" => JobStatus::TimeLimitExceeded,
        "SG" => find_typeof_runtime(exitcode),
        "RE" => JobStatus::RuntimeError("NZEC".to_string()),
        "XX" => JobStatus::InternalError,
        _ if (expected.is_empty() || stdout.trim() == expected.trim()) => JobStatus::Accepted,
        _ => JobStatus::WrongAnswer,
    }
}

fn find_typeof_runtime(exitcode: i32) -> JobStatus {
    match exitcode {
        11 => JobStatus::RuntimeError("SIGSEGV".to_string()),
        25 => JobStatus::RuntimeError("SIGXFSZ".to_string()),
        8 => JobStatus::RuntimeError("SIGFPE".to_string()),
        6 => JobStatus::RuntimeError("SIGABRT".to_string()),
        _ => JobStatus::RuntimeError("Other".to_string()),
    }
}
