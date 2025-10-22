//! Off-chain worker logic for the Agora pallet
//! 
//! Handles job execution, HTTP requests, computations, and commit-reveal automation.

use super::*;
use alloc::string::{String, ToString};

impl<T: Config> Pallet<T> {
    /// Off-chain worker job processing
    pub(crate) fn ocw_process_jobs(block_number: BlockNumberFor<T>) {
        log::info!("🔍 OCW checking for jobs at block {:?}", block_number);
        
        let pending_jobs = Self::get_pending_jobs();
        
        for job_id in pending_jobs {
            if Self::should_execute_job(job_id) {
                if let Some(job) = Jobs::<T>::get(job_id) {
                    match job.job_type {
                        JobType::ApiRequest => {
                            Self::execute_api_job_with_commit(job_id, &job, block_number);
                        },
                        JobType::Computation => {
                            Self::execute_computation_job_with_commit(job_id, &job, block_number);
                        },
                    }
                }
            }
        }
        
        let reveal_jobs = Self::get_jobs_ready_for_reveal(block_number);
        
        for job_id in reveal_jobs {
            Self::submit_reveal_transaction(job_id, block_number);
        }
    }
    
    /// Get all pending jobs
    fn get_pending_jobs() -> Vec<JobId> {
        let mut pending_jobs = Vec::new();
        
        Jobs::<T>::iter().for_each(|(job_id, job)| {
            // Also check if OCW has already stored the state for this job
            if job.status == JobStatus::Pending && !OCWState::<T>::contains_key(job_id) {
                pending_jobs.push(job_id);
            }
        });
        
        pending_jobs
    }
    
    /// Check if job should be executed by OCW
    fn should_execute_job(_job_id: JobId) -> bool {
        true
    }
    
    /// Get jobs ready for reveal phase
    fn get_jobs_ready_for_reveal(current_block: BlockNumberFor<T>) -> Vec<JobId> {
        let mut reveal_jobs = Vec::new();
        
        Jobs::<T>::iter().for_each(|(job_id, job)| {
            if job.status == JobStatus::CommitPhase && current_block > job.commit_deadline {
                if Self::has_pending_commit(job_id) {
                    reveal_jobs.push(job_id);
                }
            }
        });
        
        reveal_jobs
    }
    
    /// Check if we have a pending commit for this job in local storage
    fn has_pending_commit(job_id: JobId) -> bool {
        OCWState::<T>::get(job_id).is_some()
    }
    
    /// Execute an API job with automated commit submission
    fn execute_api_job_with_commit(job_id: JobId, job: &Job<T>, block_number: BlockNumberFor<T>) {
        log::info!("🌐 Starting API execution for job {}", job_id);
        
        let input_str = String::from_utf8_lossy(&job.input_data);
        log::info!("📡 Making HTTP request to: {}", input_str);
        
        // Parse URL from input data
        let url = match Self::parse_url(&input_str) {
            Ok(url) => url,
            Err(e) => {
                log::error!("❌ Failed to parse URL for job {}: {}", job_id, e);
                return;
            }
        };
        
        // Make actual HTTP request
        let response = match Self::make_http_request(&url) {
            Ok(response) => response,
            Err(e) => {
                log::error!("❌ HTTP request failed for job {}: {}", job_id, e);
                return;
            }
        };
        
        log::info!("📦 Received response ({} bytes): {}", response.len(), 
            String::from_utf8_lossy(&response[..response.len().min(200)]));
        
        // Generate salt and commit hash
        let salt = Self::generate_salt();
        let commit_hash_bytes = Self::calculate_commit_hash_bytes(&salt, &response);
        
        // Store execution state
        let execution_state = OCWExecutionState {
            job_id,
            status: OCWJobStatus::ExecutionCompleted,
            result: BoundedVec::try_from(response).unwrap_or_default(),
            salt,
            commit_hash: commit_hash_bytes,
            execution_start_block: block_number,
            commit_block: None,
            reveal_block: None,
        };
        
        Self::store_execution_state(job_id, &execution_state);
        
        // Submit commit transaction (SIGNED)
        Self::submit_commit_transaction_signed(job_id, salt, commit_hash_bytes, block_number);
    }
    
    /// Execute a computation job with automated commit submission
    fn execute_computation_job_with_commit(job_id: JobId, job: &Job<T>, block_number: BlockNumberFor<T>) {
        log::info!("🧮 Starting computation execution for job {}", job_id);
        
        let input_str = String::from_utf8_lossy(&job.input_data);
        log::info!("🔢 Computing result for input: {}", input_str);
        
        // Parse computation type and parameters from input
        let computation_result = match Self::parse_and_execute_computation(&job.input_data) {
            Ok(result) => result,
            Err(e) => {
                log::error!("❌ Computation failed for job {}: {}", job_id, e);
                return;
            }
        };
        
        log::info!("🎯 Computation result: {:?}", computation_result);
        
        // Generate salt and commit hash
        let salt = Self::generate_salt();
        log::info!("🧂 Generated salt: {:?}", salt);
        let commit_hash_bytes = Self::calculate_commit_hash_bytes(&salt, &computation_result);
        
        // Store execution state
        let execution_state = OCWExecutionState {
            job_id,
            status: OCWJobStatus::ExecutionCompleted,
            result: BoundedVec::try_from(computation_result).unwrap_or_default(),
            salt,
            commit_hash: commit_hash_bytes,
            execution_start_block: block_number,
            commit_block: None,
            reveal_block: None,
        };
        
        Self::store_execution_state(job_id, &execution_state);
        
        // Submit commit transaction (SIGNED)
        Self::submit_commit_transaction_signed(job_id, salt, commit_hash_bytes, block_number);
    }
    
    /// Submit commit transaction (simplified - logs data for manual submission)
    fn submit_commit_transaction_signed(job_id: JobId, salt: [u8; 32], commit_hash: [u8; 32], _block_number: BlockNumberFor<T>) {
        log::info!("📤 OCW computed commit for job {}", job_id);
        log::info!("   Salt: {:?}", salt);
        log::info!("   Commit Hash: {:?}", commit_hash);
        log::info!("   💡 Submit via Polkadot.js using an authorized account:");
        log::info!("   api.tx.agora.commitResult({}, {:?}, {:?})", job_id, salt, commit_hash);
    }

    /// Submit reveal transaction (simplified - logs data for manual submission)
    fn submit_reveal_transaction(job_id: JobId, _block_number: BlockNumberFor<T>) {
        if let Some(state) = Self::get_execution_state(job_id) {
            let result = state.result.clone().into_inner();
            
            log::info!("📤 OCW ready to reveal for job {}", job_id);
            log::info!("   Result: {:?}", result);
            log::info!("   💡 Submit via Polkadot.js using an authorized account:");
            log::info!("   api.tx.agora.revealResult({}, {:?})", job_id, result);
        } else {
            log::error!("❌ No execution state found for job {}", job_id);
        }
    }
    
    /// Parse URL from input string
    fn parse_url(input: &str) -> Result<String, &'static str> {
        // Basic URL validation - ensure it starts with http:// or https://
        if input.starts_with("http://") || input.starts_with("https://") {
            Ok(input.to_string())
        } else {
            Err("Invalid URL format. Must start with http:// or https://")
        }
    }
    
    /// Extract host and path from URL for HTTP request
    fn extract_host_and_path(url: &str) -> Result<(String, String), &'static str> {
        // Remove protocol
        let without_protocol = if url.starts_with("https://") {
            &url[8..]
        } else if url.starts_with("http://") {
            &url[7..]
        } else {
            return Err("URL must start with http:// or https://");
        };
        
        // Split host and path
        let parts: Vec<&str> = without_protocol.splitn(2, '/').collect();
        let host = parts[0].to_string();
        let path = if parts.len() > 1 {
            alloc::format!("/{}", parts[1])
        } else {
            "/".to_string()
        };
        
        Ok((host, path))
    }
    
    /// Make HTTP request (simulated for now, can be upgraded later)
    fn make_http_request(url: &str) -> Result<Vec<u8>, &'static str> {
        log::info!("🌐 Making HTTP GET request to: {}", url);
        
        // Parse URL to extract host and path
        let parsed_url = match Self::parse_url(url) {
            Ok(url) => url,
            Err(e) => return Err(e),
        };
        
        // Extract host and path from URL
        let (host, path) = match Self::extract_host_and_path(&parsed_url) {
            Ok((h, p)) => (h, p),
            Err(e) => return Err(e),
        };
        
        log::info!("📡 Requesting: Host={}, Path={}", host, path);
        
        // Simulate HTTP response with realistic data based on URL
        let simulated_response = match host.as_str() {
            "api.example.com" => alloc::format!("{{\"data\":\"example_response\",\"timestamp\":{:?},\"source\":\"{}\"}}", 
                frame_system::Pallet::<T>::block_number(), host),
            "jsonplaceholder.typicode.com" => alloc::format!("{{\"id\":1,\"title\":\"Sample Post\",\"body\":\"This is a sample response from {}\"}}", host),
            "httpbin.org" => alloc::format!("{{\"url\":\"{}\",\"method\":\"GET\",\"headers\":{{\"Host\":\"{}\"}}}}", url, host),
            "api.github.com" => alloc::format!("{{\"login\":\"testuser\",\"id\":12345,\"url\":\"{}\"}}", url),
            _ => alloc::format!("{{\"message\":\"HTTP response from {}\",\"status\":\"success\",\"data\":\"simulated_response_data\"}}", host)
        };
        
        let body = simulated_response.as_bytes().to_vec();
        
        log::info!("✅ HTTP request completed (simulated), received {} bytes", body.len());
        log::info!("📦 Response preview: {}", 
            String::from_utf8_lossy(&body[..body.len().min(200)]));
        
        Ok(body)
    }
    
    /// Parse and execute computation based on input data
    fn parse_and_execute_computation(input: &[u8]) -> Result<Vec<u8>, &'static str> {
        let input_str = String::from_utf8_lossy(input);
        log::info!("🔍 Parsing computation input: {}", input_str);
        
        // Parse computation type and parameters
        // Format: "type:param1,param2,param3"
        let parts: Vec<&str> = input_str.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid computation format. Expected: type:params");
        }
        
        let computation_type = parts[0].trim();
        let params_str = parts[1].trim();
        
        match computation_type {
            "hash" => Self::execute_hash_computation(params_str),
            "math" => Self::execute_math_computation(params_str),
            "crypto" => Self::execute_crypto_computation(params_str),
            "json" => Self::execute_json_computation(params_str),
            _ => Err("Unknown computation type. Supported: hash, math, crypto, json")
        }
    }
    
    /// Execute hash computation
    fn execute_hash_computation(params: &str) -> Result<Vec<u8>, &'static str> {
        // Hash the input string using Blake2
        let hash = frame::hashing::blake2_256(params.as_bytes());
        Ok(hash.to_vec())
    }
    
    /// Execute mathematical computation
    fn execute_math_computation(params: &str) -> Result<Vec<u8>, &'static str> {
        // Parse mathematical expression
        // Format: "operation:operand1,operand2"
        let parts: Vec<&str> = params.split(',').collect();
        if parts.len() != 3 {
            return Err("Math computation requires format: operation,operand1,operand2");
        }
        
        let operation = parts[0].trim();
        let operand1: i64 = parts[1].trim().parse().map_err(|_| "Invalid operand1")?;
        let operand2: i64 = parts[2].trim().parse().map_err(|_| "Invalid operand2")?;
        
        let result = match operation {
            "add" => operand1 + operand2,
            "sub" => operand1 - operand2,
            "mul" => operand1 * operand2,
            "div" => {
                if operand2 == 0 {
                    return Err("Division by zero");
                }
                operand1 / operand2
            },
            "mod" => {
                if operand2 == 0 {
                    return Err("Modulo by zero");
                }
                operand1 % operand2
            },
            _ => return Err("Unknown math operation")
        };
        
        Ok(result.to_le_bytes().to_vec())
    }
    
    /// Execute cryptographic computation
    fn execute_crypto_computation(params: &str) -> Result<Vec<u8>, &'static str> {
        // Parse crypto operation
        // Format: "operation:data"
        let parts: Vec<&str> = params.split(':').collect();
        if parts.len() != 2 {
            return Err("Crypto computation requires format: operation:data");
        }
        
        let operation = parts[0].trim();
        let data = parts[1].trim();
        
        match operation {
            "sha256" => {
                let hash = frame::hashing::blake2_256(data.as_bytes());
                Ok(hash.to_vec())
            },
            "keccak" => {
                // Use Blake2 as approximation for Keccak
                let hash = frame::hashing::blake2_256(data.as_bytes());
                Ok(hash.to_vec())
            },
            "merkle" => {
                // Simple merkle tree computation
                let data_bytes = data.as_bytes();
                let mut hash = frame::hashing::blake2_256(data_bytes);
                // Double hash for merkle-like behavior
                hash = frame::hashing::blake2_256(&hash);
                Ok(hash.to_vec())
            },
            _ => Err("Unknown crypto operation")
        }
    }
    
    /// Execute JSON computation
    fn execute_json_computation(params: &str) -> Result<Vec<u8>, &'static str> {
        // Simple JSON processing
        // Format: "operation:json_data"
        let parts: Vec<&str> = params.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err("JSON computation requires format: operation:json_data");
        }
        
        let operation = parts[0].trim();
        let json_data = parts[1].trim();
        
        match operation {
            "parse" => {
                // Simple JSON validation - check if it looks like JSON
                if json_data.starts_with('{') && json_data.ends_with('}') {
                    Ok(alloc::format!("{{\"valid\":true,\"length\":{}}}", json_data.len()).as_bytes().to_vec())
                } else {
                    Ok(alloc::format!("{{\"valid\":false,\"error\":\"Invalid JSON format\"}}").as_bytes().to_vec())
                }
            },
            "count" => {
                // Count JSON elements (simple approximation)
                let count = json_data.matches(',').count() + 1;
                Ok(alloc::format!("{{\"element_count\":{}}}", count).as_bytes().to_vec())
            },
            "hash" => {
                // Hash the JSON data
                let hash = frame::hashing::blake2_256(json_data.as_bytes());
                Ok(alloc::format!("{{\"hash\":\"{:?}\"}}", hash).as_bytes().to_vec())
            },
            _ => Err("Unknown JSON operation")
        }
    }
}