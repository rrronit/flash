use crate::{
    client::redis::RedisClient,
    core::{Job, JobStatus},
};
use redis::RedisError;
use std::{
    fs::File,
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
        let init_output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("isolate")
                .args(&["-b", &box_id.to_string(), "--init"])
                .output()
        })
        .await
        .map_err(|e| format!("Failed to initialize box: {:?}", e))
        .unwrap()?;


        let box_path = String::from_utf8_lossy(&init_output.stdout)
            .trim()
            .to_string();

        let file_path = format!("{}/box", box_path);
        let stdin_file = format!("{}/stdin", file_path);
        let stdout_file = format!("{}/stdout", file_path);
        let stderr_file = format!("{}/stderr", file_path);
        let metadata_file = format!("{}/metadata", file_path);

        // Write source code
        let source_path = format!("{}/{}", file_path, job.language.source_file);

        tokio::fs::write(&source_path, &job.source_code)    
            .await
            .expect(format!("Failed to write source code to {}", source_path).as_str());
        tokio::fs::write(&stdin_file, &job.stdin)
            .await
            .expect(format!("Failed to write stdin to {}", stdin_file).as_str());

        let stdin_file =
            File::open(stdin_file).expect(format!("Failed to open stdin file {}", box_id).as_str());
        // let mut stdout_file = File::create(stdout_file)
        //     .expect(format!("Failed to open stdout file {}", box_id).as_str());
        // let mut stderr_file = File::create(stderr_file)
        //     .expect(format!("Failed to open stderr file {}", box_id).as_str());
        File::create(&metadata_file)
            .expect(format!("Failed to open metadata file {}", box_id).as_str());

        // Run compilation if needed
        if let Some(compile_cmd) = &job.language.compile_cmd {
            let compile_parts: Vec<&str> = compile_cmd.split_whitespace().collect();
            let compile_executable = compile_parts[0];
            let compile_args = &compile_parts[1..];

            let compile_status = Command::new("isolate")
                .args(&[
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
                .await
                .expect(format!("Failed to compile job {}", job.id).as_str());

            println!("Compilation status: {:?}", compile_status);

            if !compile_status.status.success() {
                let compile_output =
                    tokio::fs::read_to_string(format!("{}/compile_output", file_path))
                        .await
                        .expect(
                            format!("Failed to read compile output for job {}", job.id).as_str(),
                        );

                println!("Compilation error: {}", compile_output);

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
            .await
            .unwrap();

        let stdout_content = tokio::fs::read_to_string(&stdout_file).await.unwrap();
        let stderr_content = tokio::fs::read_to_string(&stderr_file).await.unwrap();


        job.output.stdout = Some(stdout_content);
        job.output.stderr = Some(stderr_content);

        println!("Job finished");
        println!("output: {:?}", job.output.stdout);
        println!("stderr: {:?}", job.output.stderr);
        println!("status: {:?}", job.status);


        let metadata = self.get_metadata(box_id).await;
        job.finished_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .try_into()
                .unwrap(),
        );
        println!("metadata: {:?}", metadata);

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

        // job.output.result =
        //     Some(self.get_results(&job.output.stdout.clone().unwrap(), &job.expected_output));

        // Update job in Redis
        self.update_job_in_redis(job).await.unwrap();

        Ok(job.status.clone())
    }

    async fn update_job_in_redis(&self, job: &Job) -> Result<(), RedisError> {
        let job_id = job.id.to_string();
        self.redis
            .store_job(&job_id, job, None)
            .await
            .map_err(|_| RedisError::from((redis::ErrorKind::IoError, "Failed to store job")))
    }

    async fn get_metadata(&self, box_id: u64) -> Metadata {
        let metadata_file = format!("/var/local/lib/isolate/{}/box/metadata", box_id);
        let metadata = tokio::fs::read_to_string(metadata_file)
            .await
            .map_err(|_| JobStatus::RuntimeError)
            .map_err(|_| JobStatus::InternalError)
            .unwrap();

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
            "exitcode" => m.exit_code = f.1.parse().unwrap(),
            "message" => m.message = f.1.to_string(),
            "status" => m.status = f.1.to_string(),
            _ => {}
        });

        m
    }

    // fn get_results(&self, expected: &String, output: &String) -> {
    //     if expected == output {
    //         JobResult::Accepted
    //     } else {
    //         JobResult::WrongAnswer
    //     }
    // }
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
