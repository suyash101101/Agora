import type { AccountId, Balance, BlockNumber } from '@polkadot/types/interfaces';
import type { Hash } from '@polkadot/types/interfaces/runtime';

export type JobId = number;

export interface Job {
  creator: AccountId;
  bounty: Balance;
  jobType: number; // 0 = ApiRequest, 1 = Computation
  inputData: Uint8Array;
  status: string;
  createdAt: BlockNumber;
  commitDeadline: BlockNumber;
  revealDeadline: BlockNumber;
  originParaId: number;
  result: Uint8Array;
}

export interface WorkerInfo {
  stake: Balance;
  reputation: number;
  isActive: boolean;
  registeredAt: BlockNumber;
}

export interface Commit {
  worker: AccountId;
  salt: Uint8Array;
  resultHash: Hash;
  committedAt: BlockNumber;
}

export interface Reveal {
  worker: AccountId;
  salt: Uint8Array;
  result: Uint8Array;
  revealedAt: BlockNumber;
}

export interface ChainInfo {
  name: string;
  token: string;
  decimals: number;
  blockNumber: BlockNumber;
  endpoint: string;
}

export interface AccountInfo {
  address: string;
  name?: string;
  balance?: Balance;
  source: 'extension' | 'manual';
}

