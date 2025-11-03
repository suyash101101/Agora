//! Helper functions for the Agora pallet
//! 
//! Contains consensus determination, reward distribution, job lifecycle management,
//! and other internal utility functions.

use super::*;
use alloc::collections::BTreeMap;

impl<T: Config> Pallet<T> {
    /// Determine consensus result from reveals (simple majority)
    pub(crate) fn determine_consensus(
        reveals: &BoundedVec<Reveal<T>, ConstU32<100>>,
    ) -> Result<BoundedVec<u8, ConstU32<2048>>, DispatchError> {
        // Count occurrences of each result
        let mut result_counts: BTreeMap<Vec<u8>, usize> = BTreeMap::new();

        for reveal in reveals.iter() {
            let result_vec = reveal.result.to_vec();
            *result_counts.entry(result_vec).or_insert(0) += 1;
        }

        // Find result with most votes
        let consensus = result_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(result, _)| result)
            .ok_or(Error::<T>::InsufficientReveals)?;

        // Convert to bounded vec
        consensus.try_into().map_err(|_| Error::<T>::InputDataTooLarge.into())
    }

    /// Distribute rewards to honest workers and slash dishonest ones
    pub(crate) fn distribute_rewards_and_slash(
        job_id: JobId,
        job: &Job<T>,
        reveals: &BoundedVec<Reveal<T>, ConstU32<100>>,
        consensus_result: &BoundedVec<u8, ConstU32<2048>>,
    ) -> DispatchResult {
        let honest_workers: Vec<_> = reveals
            .iter()
            .filter(|r| &r.result == consensus_result)
            .map(|r| r.worker.clone())
            .collect();

        let dishonest_workers: Vec<_> = reveals
            .iter()
            .filter(|r| &r.result != consensus_result)
            .map(|r| r.worker.clone())
            .collect();

        // Calculate reward per honest worker
        let total_honest = honest_workers.len() as u128;

        if total_honest > 0 {
            let reward_per_worker = job.bounty / total_honest;

            // Release bounty and distribute to honest workers
            T::Currency::release(
                &HoldReason::JobBounty.into(),
                &job.creator,
                job.bounty,
                frame::traits::tokens::Precision::BestEffort,
            )?;

            for worker in honest_workers {
                // Transfer reward
                T::Currency::transfer(
                    &job.creator,
                    &worker,
                    reward_per_worker,
                    frame::traits::tokens::Preservation::Preserve,
                )?;

                // Update reputation
                if let Some(mut worker_info) = Workers::<T>::get(&worker) {
                    worker_info.reputation = worker_info.reputation.saturating_add(10).min(1000);
                    Workers::<T>::insert(&worker, worker_info);
                }

                let worker_account_id: <T as frame_system::Config>::AccountId = worker;

                Self::deposit_event(Event::WorkerRewarded {
                    job_id,
                    worker: worker_account_id,
                    amount: reward_per_worker,
                });
            }
        }

        // Slash dishonest workers
        for worker in dishonest_workers {
            if let Some(mut worker_info) = Workers::<T>::get(&worker) {
                // Slash 10% of stake
                let slash_amount = worker_info.stake / 10;
                
                // Reduce stake
                worker_info.stake = worker_info.stake.saturating_sub(slash_amount);
                
                // Reduce reputation
                worker_info.reputation = worker_info.reputation.saturating_sub(50);
                
                Workers::<T>::insert(&worker, worker_info);
                
                let worker_account_id: <T as frame_system::Config>::AccountId = worker;

                Self::deposit_event(Event::WorkerSlashed {
                    job_id,
                    worker: worker_account_id,
                    amount: slash_amount,
                });
            }
        }

        Ok(())
    }

    /// Process job lifecycle transitions (called in on_initialize)
    pub(crate) fn process_job_transitions(block_number: BlockNumberFor<T>) -> Weight {
        let mut weight = Weight::from_parts(0, 0);
        let mut processed_jobs = 0;

        // Iterate through all jobs to find those needing transitions
        Jobs::<T>::iter().for_each(|(job_id, mut job)| {
            if processed_jobs >= 10 { // Limit processing per block
                return;
            }

            match job.status {
                JobStatus::Pending => {
                    // Auto-transition to CommitPhase when commit deadline approaches
                    if block_number >= job.commit_deadline.saturating_sub(T::CommitPhaseDuration::get()) {
                        job.status = JobStatus::CommitPhase;
                        Jobs::<T>::insert(job_id, job);
                        processed_jobs += 1;
                        weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(1));
                    }
                }
                JobStatus::CommitPhase => {
                    // Auto-transition to RevealPhase when commit deadline passes
                    if block_number > job.commit_deadline {
                        job.status = JobStatus::RevealPhase;
                        Jobs::<T>::insert(job_id, job);
                        processed_jobs += 1;
                        weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(1));
                    }
                }
                JobStatus::RevealPhase => {
                    // Auto-finalize when reveal deadline passes
                    if block_number > job.reveal_deadline {
                        if let Ok(_) = Self::finalize_job_internal(job_id, &job) {
                            processed_jobs += 1;
                            weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads_writes(5, 3));
                        }
                    }
                }
                _ => {} // No action needed for Completed/Failed jobs
            }
        });

        weight
    }

    /// Process unbonding workers (called in on_initialize)
    pub(crate) fn process_unbonding_workers(block_number: BlockNumberFor<T>) -> Weight {
        let mut weight = Weight::from_parts(0, 0);
        let mut processed_workers = 0;

        // Find workers ready to complete unbonding
        UnbondingWorkers::<T>::iter().for_each(|(worker, unbonding_block)| {
            if processed_workers >= 5 { // Limit processing per block
                return;
            }

            if block_number >= unbonding_block {
                // Complete unbonding
                if let Some(worker_info) = Workers::<T>::get(&worker) {
                    // Release stake
                    let _ = T::Currency::release(
                        &HoldReason::WorkerStake.into(),
                        &worker,
                        worker_info.stake,
                        frame::traits::tokens::Precision::BestEffort,
                    );

                    // Remove from unbonding and workers
                    UnbondingWorkers::<T>::remove(&worker);
                    Workers::<T>::remove(&worker);

                    processed_workers += 1;
                    weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(2));

                    Self::deposit_event(Event::WorkerUnregistered { worker });
                }
            }
        });

        weight
    }

    /// Internal finalize job function (used by both manual and auto-finalization)
    pub(crate) fn finalize_job_internal(job_id: JobId, job: &Job<T>) -> DispatchResult {
        // Get reveals
        let reveals = Reveals::<T>::get(job_id).ok_or(Error::<T>::InsufficientReveals)?;
        ensure!(reveals.len() > 0, Error::<T>::InsufficientReveals);

        // Determine consensus result
        let consensus_result = Self::determine_consensus(&reveals)?;

        // Distribute rewards and slash dishonest workers
        Self::distribute_rewards_and_slash(job_id, job, &reveals, &consensus_result)?;

        // Store final result
        Results::<T>::insert(job_id, consensus_result.clone());

        // If the job originated via XCM, send the result hash back to the origin parachain
        let result_hash = <T as frame_system::Config>::Hashing::hash_of(&consensus_result.to_vec());
        let _ = Self::maybe_send_remote_result(job_id, result_hash);

        // Update job status
        let mut updated_job = job.clone();
        updated_job.status = JobStatus::Completed;
        Jobs::<T>::insert(job_id, updated_job);

        // Emit event
        Self::deposit_event(Event::JobFinalized {
            job_id,
            result: consensus_result.to_vec(),
        });

        log::info!("🔍 Job {} finalized, checking if remote job", job_id);
    
        if RemoteJobInfo::<T>::contains_key(job_id) {
            log::info!("📤 Job {} is remote, sending result back via XCM", job_id);
            
            match Self::maybe_send_remote_result(job_id, consensus_result.clone()) {
                Ok(_) => {
                    log::info!("✅ Successfully sent remote result for job {}", job_id);
                },
                Err(e) => {
                    log::error!("❌ Failed to send remote result for job {}: {:?}", job_id, e);
                }
            }
        } else {
            log::info!("📍 Job {} is local, no XCM message needed", job_id);
        }

        Ok(())
    }

    /// Generate a random salt for commit-reveal
    pub(crate) fn generate_salt() -> [u8; 32] {
        // Use block hash for pseudo-random salt (simplified approach)
        let block_hash = frame_system::Pallet::<T>::block_hash(frame_system::Pallet::<T>::block_number());
        let mut salt = [0u8; 32];
        salt.copy_from_slice(&block_hash.as_ref()[..32]);
        salt
    }
    
    /// Calculate commit hash from salt and result
    pub(crate) fn calculate_commit_hash_bytes(salt: &[u8; 32], result: &[u8]) -> [u8; 32] {
        let mut salted_input = Vec::new();
        salted_input.extend_from_slice(salt);
        salted_input.extend_from_slice(result);
        frame::hashing::blake2_256(&salted_input)
    }
    
    /// Store execution state in local storage (OCWState)
    pub(crate) fn store_execution_state(job_id: JobId, state: &OCWExecutionState<T>) {
        OCWState::<T>::insert(job_id, state);
        log::info!("💾 Stored execution state for job {}", job_id);
    }
    
    /// Get execution state from local storage (OCWState)
    pub(crate) fn get_execution_state(job_id: JobId) -> Option<OCWExecutionState<T>> {
        OCWState::<T>::get(job_id)
    }
}