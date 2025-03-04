use crate::{
    client::redis::RedisClient,
    core::{Job, JobStatus},
};
use futures::TryFutureExt;
use redis::RedisError;
use std::{
    fs::{self, File},
    io::Error,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::process::Command;
use tracing;

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

    #[tracing::instrument(skip(self, job), fields(job_id = job.id), level = "info")]
    pub async fn execute(&self, job: &mut Job) -> Result<JobStatus, Error> {
        let total_start_time = SystemTime::now();
        
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

        tracing::info!("Starting job execution in box {}", box_id);

        // Initialize new box
        let init_start_time = SystemTime::now();
        let init_output = Command::new("isolate")
            .args(&["-b", &box_id.to_string(), "--cg", "--init"])
            .output()
            .await
            .map_err(|e| {
                tracing::error!("Failed to initialize isolate box {}: {:?}", box_id, e);
                Error::new(std::io::ErrorKind::Other, format!("Failed to initialize box: {:?}", e))
            })?;
        let init_duration = init_start_time.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::info!("Box initialization took {:?}", init_duration);

        if !init_output.status.success() {
            let stderr = String::from_utf8_lossy(&init_output.stderr);
            tracing::error!("Isolate initialization failed: {}", stderr);
            job.status = JobStatus::InternalError;
            self.update_job_in_redis(job).await?;
            return Ok(JobStatus::InternalError);
        }

        tracing::debug!("Box {} initialized", box_id);

        let box_path = String::from_utf8_lossy(&init_output.stdout).trim().to_string();
        if box_path.is_empty() {
            tracing::error!("Empty box path returned from isolate init");
            job.status = JobStatus::InternalError;
            self.update_job_in_redis(job).await?;
            return Ok(JobStatus::InternalError);
        }

        let file_setup_start = SystemTime::now();
        let (file_path, metadata_file, stdin_file, stdout_file, stderr_file) =
            self.setup_files(job, &box_path).map_err(|e| {
                tracing::error!("Error setting up files: {:?}", e);
                e
            })?;
        let file_setup_duration = file_setup_start.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::info!("File setup took {:?}", file_setup_duration);

        tracing::debug!("Files set up for job {}", job.id);

        // Run compilation if needed
        if let Some(compile_cmd) = &job.language.compile_cmd {
            let compile_start = SystemTime::now();
            tracing::info!("Compiling {} code for job {}", job.language.name, job.id);
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
                .await
                .map_err(|e| {
                    tracing::error!("Error running compilation: {:?}", e);
                    Error::new(std::io::ErrorKind::Other, format!("Failed to run compilation: {:?}", e))
                })?;
            let compile_duration = compile_start.elapsed().unwrap_or(Duration::from_secs(0));
            tracing::info!("Compilation took {:?}", compile_duration);

            let output_reading_start = SystemTime::now();
            let compile_output_path = format!("{}/compile_output", file_path);
            if Path::new(&compile_output_path).exists() {
                let compile_output = fs::read_to_string(&compile_output_path)
                    .map_err(|e| {
                        tracing::error!("Error reading compile output from {}: {:?}", compile_output_path, e);
                        Error::new(std::io::ErrorKind::Other, format!("Failed to read compile output for job {}", job.id))
                    })?;

                job.output.compile_output = Some(compile_output.clone());
                
                if !compile_status.status.success() {
                    tracing::info!("Compilation failed for job {}: {}", job.id, compile_output);
                    job.status = JobStatus::CompilationError;
                    
                    let redis_update_start = SystemTime::now();
                    self.update_job_in_redis(job).await.map_err(|e| {
                        tracing::error!("Error updating job in Redis: {:?}", e);
                        Error::new(std::io::ErrorKind::Other, "Failed to update job in Redis")
                    })?;
                    let redis_update_duration = redis_update_start.elapsed().unwrap_or(Duration::from_secs(0));
                    tracing::info!("Redis update took {:?}", redis_update_duration);

                    let total_duration = total_start_time.elapsed().unwrap_or(Duration::from_secs(0));
                    tracing::info!("Total execution time for failed compilation: {:?}", total_duration);
                    return Ok(JobStatus::CompilationError);
                }
            } else if !compile_status.status.success() {
                tracing::error!("Compilation failed but compile_output file not found");
                job.status = JobStatus::CompilationError;
                job.output.compile_output = Some(String::from_utf8_lossy(&compile_status.stderr).to_string());
                
                let redis_update_start = SystemTime::now();
                self.update_job_in_redis(job).await?;
                let redis_update_duration = redis_update_start.elapsed().unwrap_or(Duration::from_secs(0));
                tracing::info!("Redis update took {:?}", redis_update_duration);

                let total_duration = total_start_time.elapsed().unwrap_or(Duration::from_secs(0));
                tracing::info!("Total execution time for failed compilation: {:?}", total_duration);
                return Ok(JobStatus::CompilationError);
            }
            let output_reading_duration = output_reading_start.elapsed().unwrap_or(Duration::from_secs(0));
            tracing::info!("Reading compilation output took {:?}", output_reading_duration);
        }

        tracing::info!("Executing job {}", job.id);
        let run_parts: Vec<&str> = job.language.run_cmd.split_whitespace().collect();
        let run_executable = run_parts[0];
        let run_args = &run_parts[1..];

        let execution_start = SystemTime::now();
        let run_output = Command::new("isolate")
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
            .await
            .map_err(|e| {
                tracing::error!("Error executing job {}: {:?}", job.id, e);
                Error::new(std::io::ErrorKind::Other, format!("Failed to run job: {:?}", e))
            })?;
        let execution_duration = execution_start.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::info!("Execution took {:?}", execution_duration);

        tracing::debug!("Job {} execution completed with status: {}", job.id, run_output.status);

        let output_reading_start = SystemTime::now();
        if Path::new(&stdout_file).exists() {
            let stdout_content = fs::read_to_string(&stdout_file).unwrap_or_else(|e| {
                tracing::error!("Error reading stdout from {}: {:?}", stdout_file, e);
                String::new()
            });
            job.output.stdout = Some(stdout_content);
        }

        if Path::new(&stderr_file).exists() {
            let stderr_content = fs::read_to_string(&stderr_file).unwrap_or_else(|e| {
                tracing::error!("Error reading stderr from {}: {:?}", stderr_file, e);
                String::new()
            });
            job.output.stderr = Some(stderr_content);
        } else {
            job.output.stderr = Some(String::new());
        }
        let output_reading_duration = output_reading_start.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::info!("Reading output files took {:?}", output_reading_duration);

        let metadata_start = SystemTime::now();
        let metadata = match self.get_metadata(box_id) {
            Ok(meta) => {
                tracing::debug!("Retrieved metadata for job {}: {:?}", job.id, meta);
                meta
            },
            Err(e) => {
                tracing::error!("Error getting metadata for job {}: {:?}", job.id, e);
                job.status = JobStatus::InternalError;
                self.update_job_in_redis(job).await?;
                return Ok(JobStatus::InternalError);
            }
        };
        let metadata_duration = metadata_start.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::info!("Reading metadata took {:?}", metadata_duration);

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
        job.output.message = Some(metadata.message.clone());
        
        let status_determination_start = SystemTime::now();
        let stdout = job.output.stdout.clone().unwrap_or_default();
        job.status = determine_status(
            metadata.status,
            metadata.exit_code,
            &stdout,
            &job.expected_output,
        );
        let status_determination_duration = status_determination_start.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::info!("Status determination took {:?}", status_determination_duration);

        tracing::info!("Job {} completed with status: {:?}", job.id, job.status);

        let redis_update_start = SystemTime::now();
        self.update_job_in_redis(job).await.map_err(|e| {
            tracing::error!("Error updating job in Redis: {:?}", e);
            Error::new(std::io::ErrorKind::Other, "Failed to update job in Redis")
        })?;
        let redis_update_duration = redis_update_start.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::info!("Final Redis update took {:?}", redis_update_duration);

        let total_duration = total_start_time.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::info!("Total job execution took {:?}", total_duration);

        Ok(job.status.clone())
    }

    async fn update_job_in_redis(&self, job: &Job) -> Result<(), Error> {
        let redis_start = SystemTime::now();
        let result = self.redis.store_job(&job.id.to_string(), job, None).await.map_err(|e| {
            tracing::error!("Error storing job {} in Redis: {:?}", job.id, e);
            Error::new(std::io::ErrorKind::Other, format!("Failed to store job in Redis: {}", e))
        });
        let redis_duration = redis_start.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::debug!("Redis store operation took {:?}", redis_duration);
        result
    }

    fn get_metadata(&self, box_id: u64) -> Result<Metadata, Error> {
        let start_time = SystemTime::now();
        
        let metadata_file = format!("/var/local/lib/isolate/{}/box/metadata", box_id);
        if !Path::new(&metadata_file).exists() {
            return Err(Error::new(
                std::io::ErrorKind::NotFound, 
                format!("Metadata file not found at {}", metadata_file)
            ));
        }
        
        let metadata = fs::read_to_string(&metadata_file).map_err(|e| {
            tracing::error!("Error reading metadata file {}: {:?}", metadata_file, e);
            Error::new(std::io::ErrorKind::Other, format!("Failed to read metadata: {}", e))
        })?;

        let lines: Vec<&str> = metadata.lines().collect();

        let meta = lines.iter().filter_map(|&line| {
            let mut parts = line.splitn(2, ':');
            match (parts.next(), parts.next()) {
                (Some(key), Some(value)) => Some((key, value)),
                _ => None,
            }
        });

        let mut m = Metadata {
            time: 0.0,
            memory: 0,
            exit_code: 0,
            message: "".to_string(),
            status: "".to_string(),
        };

        for (key, value) in meta {
            match key {
                "time" => {
                    m.time = value.parse().unwrap_or_else(|_| {
                        tracing::warn!("Failed to parse time value: {}", value);
                        0.0
                    })
                },
                "max-rss" => {
                    m.memory = value.parse().unwrap_or_else(|_| {
                        tracing::warn!("Failed to parse max-rss value: {}", value);
                        0
                    })
                },
                "cg-mem" => {
                    m.memory = value.parse().unwrap_or_else(|_| {
                        tracing::warn!("Failed to parse cg-mem value: {}", value);
                        0
                    })
                },
                "exitcode" => {
                    m.exit_code = value.parse().unwrap_or_else(|_| {
                        tracing::warn!("Failed to parse exitcode value: {}", value);
                        0
                    })
                },
                "message" => m.message = value.to_string(),
                "status" => m.status = value.to_string(),
                _ => {}
            }
        }

        let duration = start_time.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::debug!("Getting metadata took {:?}", duration);

        Ok(m)
    }

    fn setup_files(
        &self,
        job: &Job,
        box_path: &str,
    ) -> Result<(String, String, File, String, String), Error> {
        let start_time = SystemTime::now();
        
        let file_path = format!("{}/box", box_path);
        let stdin_file = format!("{}/stdin", file_path);
        let stdout_file = format!("{}/stdout", file_path);
        let stderr_file = format!("{}/stderr", file_path);
        let metadata_file = format!("{}/metadata", file_path);

        // Write source code
        let source_path = format!("{}/{}", file_path, job.language.source_file);
        
        let source_write_start = SystemTime::now();
        fs::write(&source_path, &job.source_code).map_err(|e| {
            tracing::error!("Error writing source code to {}: {:?}", source_path, e);
            Error::new(std::io::ErrorKind::Other, format!("Failed to write source code: {}", e))
        })?;
        let source_write_duration = source_write_start.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::debug!("Writing source code took {:?}", source_write_duration);

        let stdin_write_start = SystemTime::now();
        fs::write(&stdin_file, &job.stdin).map_err(|e| {
            tracing::error!("Error writing stdin to {}: {:?}", stdin_file, e);
            Error::new(std::io::ErrorKind::Other, format!("Failed to write stdin: {}", e))
        })?;
        let stdin_write_duration = stdin_write_start.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::debug!("Writing stdin took {:?}", stdin_write_duration);

        let stdin_open_start = SystemTime::now();
        let stdin_file = File::open(&stdin_file).map_err(|e| {
            tracing::error!("Error opening stdin file {}: {:?}", stdin_file, e);
            Error::new(std::io::ErrorKind::Other, format!("Failed to open stdin file: {}", e))
        })?;
        let stdin_open_duration = stdin_open_start.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::debug!("Opening stdin took {:?}", stdin_open_duration);

        let duration = start_time.elapsed().unwrap_or(Duration::from_secs(0));
        tracing::debug!("Total setup_files took {:?}", duration);

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