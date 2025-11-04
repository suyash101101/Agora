// Chain constants from runtime/src/lib.rs
export const UNIT = 1_000_000_000_000n; // 12 decimals
export const MILLI_UNIT = 1_000_000_000n;
export const MICRO_UNIT = 1_000_000n;

// Agora pallet constants
export const MIN_WORKER_STAKE = 100n * UNIT;
export const MIN_JOB_BOUNTY = 50n * UNIT;
export const COMMIT_PHASE_DURATION = 30; // blocks
export const REVEAL_PHASE_DURATION = 30; // blocks
export const BLOCK_TIME_MS = 6000; // 6 seconds

// Default WebSocket endpoints
export const DEFAULT_ENDPOINTS = {
  para1000: 'ws://localhost:9990',
  para2000: 'ws://localhost:9991',
  relay: 'ws://localhost:9988',
} as const;

export const DEFAULT_ENDPOINT = DEFAULT_ENDPOINTS.para1000;

// Job types
export enum JobType {
  ApiRequest = 0,
  Computation = 1,
}

export const JOB_TYPE_LABELS: Record<JobType, string> = {
  [JobType.ApiRequest]: 'API Request',
  [JobType.Computation]: 'Computation',
};

// Job status
export enum JobStatus {
  Pending = 'Pending',
  CommitPhase = 'CommitPhase',
  RevealPhase = 'RevealPhase',
  Completed = 'Completed',
  Failed = 'Failed',
}

export const JOB_STATUS_COLORS: Record<JobStatus, string> = {
  [JobStatus.Pending]: 'bg-yellow-100 text-yellow-800',
  [JobStatus.CommitPhase]: 'bg-blue-100 text-blue-800',
  [JobStatus.RevealPhase]: 'bg-purple-100 text-purple-800',
  [JobStatus.Completed]: 'bg-green-100 text-green-800',
  [JobStatus.Failed]: 'bg-red-100 text-red-800',
};

