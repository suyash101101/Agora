import { randomAsU8a } from '@polkadot/util-crypto';
import { blake2AsHex } from '@polkadot/util-crypto';
import { u8aConcat, u8aToHex } from '@polkadot/util';

export function generateSalt(): Uint8Array {
  return randomAsU8a(32);
}

export function hashResult(salt: Uint8Array, result: Uint8Array): string {
  const combined = u8aConcat(salt, result);
  return blake2AsHex(combined, 256);
}

export function generateCommitHash(salt: Uint8Array, result: Uint8Array): string {
  return hashResult(salt, result);
}

export function validateJobInput(input: string, maxBytes: number = 1024): { valid: boolean; error?: string } {
  const bytes = new TextEncoder().encode(input);
  if (bytes.length > maxBytes) {
    return {
      valid: false,
      error: `Input data exceeds maximum size of ${maxBytes} bytes (current: ${bytes.length} bytes)`,
    };
  }
  return { valid: true };
}

export function validateBounty(bounty: bigint, minBounty: bigint): { valid: boolean; error?: string } {
  if (bounty < minBounty) {
    return {
      valid: false,
      error: `Bounty must be at least ${minBounty.toString()}`,
    };
  }
  return { valid: true };
}

export function validateStake(stake: bigint, minStake: bigint): { valid: boolean; error?: string } {
  if (stake < minStake) {
    return {
      valid: false,
      error: `Stake must be at least ${minStake.toString()}`,
    };
  }
  return { valid: true };
}

export async function waitForBlock(api: any, targetBlock: number): Promise<void> {
  return new Promise((resolve) => {
    const unsubscribe = api.rpc.chain.subscribeNewHeads((header: any) => {
      if (header.number.toNumber() >= targetBlock) {
        unsubscribe();
        resolve();
      }
    });
  });
}

export function isJobInCommitPhase(currentBlock: number, commitDeadline: number): boolean {
  return currentBlock <= commitDeadline;
}

export function isJobInRevealPhase(currentBlock: number, commitDeadline: number, revealDeadline: number): boolean {
  return currentBlock > commitDeadline && currentBlock <= revealDeadline;
}

export function canFinalizeJob(currentBlock: number, revealDeadline: number): boolean {
  return currentBlock > revealDeadline;
}

