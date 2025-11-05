import { randomAsU8a } from '@polkadot/util-crypto';
import { blake2AsHex, blake2AsU8a } from '@polkadot/util-crypto';
import { u8aConcat, u8aToHex, hexToU8a } from '@polkadot/util';

/**
 * Generate a random 32-byte salt for commit-reveal scheme
 */
export function generateSalt(): Uint8Array {
  return randomAsU8a(32);
}

/**
 * Hash salt + result using Blake2-256 (same as runtime)
 * Returns hex string for display
 */
export function hashResult(salt: Uint8Array, result: Uint8Array): string {
  const combined = u8aConcat(salt, result);
  return blake2AsHex(combined, 256);
}

/**
 * Generate commit hash from salt and result
 * Returns hex string for display
 */
export function generateCommitHash(salt: Uint8Array, result: Uint8Array): string {
  return hashResult(salt, result);
}

/**
 * Generate commit hash bytes (Uint8Array) for API submission
 * This matches the runtime's blake2_256(salt + result) calculation
 */
export function generateCommitHashBytes(salt: Uint8Array, result: Uint8Array): Uint8Array {
  const combined = u8aConcat(salt, result);
  return blake2AsU8a(combined, 256);
}

/**
 * Convert hex string hash to Hash type for API
 * Polkadot.js API accepts hex strings and converts them automatically
 */
export function hashHexToHash(hex: string): string {
  // Ensure it starts with 0x and is properly formatted
  if (!hex.startsWith('0x')) {
    return `0x${hex}`;
  }
  return hex;
}

/**
 * Ensure salt is exactly 32 bytes
 */
export function ensureSalt32Bytes(salt: Uint8Array): Uint8Array {
  if (salt.length !== 32) {
    throw new Error(`Salt must be exactly 32 bytes, got ${salt.length}`);
  }
  return salt;
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

