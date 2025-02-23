
# Flash ⚡

> A High-Performance Code Execution and Isolation System

![flash logo](https://fal.media/files/kangaroo/PV2NZPSwvxfzkaINQ-h5w_image.png)

Flash provides a robust system for executing code in an isolated environment, ensuring security and resource management. It supports multiple programming languages, including Python, C++, JavaScript, and Java. **SQL support is coming soon!** The system leverages Linux namespaces, cgroups, and the `isolate` tool to create secure sandboxes for code execution.

---

## Benchmark Results (C++ Code Execution)

The system was benchmarked against [Judge0](https://github.com/judge0/judge0), a popular open-source code execution system. Below are the comparative results:

| Batch Size | Flash (s) | Judge0 (s) | Performance Gain |
|:-----------|:-------------|:-----------|:------------|
| 1          | 0.423       | 1.024      | 58.7%      |
| 5          | 0.686       | 1.067      | 35.7%      |
| 10         | 1.096       | 2.151      | 49.0%      |
| 20         | 1.972       | 3.478      | 43.3%      |
| 100        | 9.772       | 14.019     | 30.3%      |
| 200        | 11.261      | 28.140     | 60.0%      |

---

*Benchmark methodology: All tests run on equivalent hardware, using identical C++ programs, averaged over 100 runs.*

## Features  

- **Isolated Execution**: Each job runs in a secure sandbox with resource limits (CPU, memory, stack, etc.).  
- **Multi-Language Support**: Supports Python, C++, JavaScript, and Java. **SQL support coming soon!**  
- **Job Queuing**: Uses Redis for job queuing and status tracking.  
- **Resource Limits**: Enforces CPU time, memory, stack size, and process limits.  
- **Debugging Support**: Provides a debugging interface for step-by-step execution of code.**(coming soon)**
- **Scalability**: Handles multiple jobs concurrently with configurable concurrency levels.  

---

## Architecture  

The system is composed of the following components:  

1. **Worker**: Fetches jobs from Redis, executes them in isolated environments, and updates their status.  
2. **Server**: Provides an HTTP API for submitting jobs and checking their status.  
3. **Isolator**: Manages the isolation of code execution using Linux namespaces and cgroups.  
4. **Redis Client**: Handles communication with Redis for job queuing and status storage.  

---

## Usage  

### API Endpoints  

- **POST /create**: Submit a new job.  

  ```json  
  {  
    "code": "print('Hello, World!')",  
    "language": "python",  
    "input": "",  
    "expected": "Hello, World!",  
    "time_limit": 2.0,  
    "memory_limit": 128000,  
    "stack_limit": 64000  
  }  
  ```  

- **GET /check/{job_id}**: Check the status of a job.  

  ```json  
  {  
    "started_at": 1633024800,  
    "finished_at": 1633024805,  
    "stdout": "Hello, World!",  
    "time": 0.1,  
    "memory": 1024,  
    "stderr": "",  
    "token": 12345,  
    "compile_output": "",  
    "message": "",  
    "status": {  
      "id": 3,  
      "description": "Accepted"  
    }  
  }  
  ```  

- **POST /debug**: Debug a piece of code.  

  ```json  
  {  
    "code": "print('Hello, World!')",  
    "language": "python",  
    "input": ""  
  }  
  ```  

---
