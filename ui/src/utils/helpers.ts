import { randomAsU8a } from '@polkadot/util-crypto';
import { blake2AsHex, blake2AsU8a } from '@polkadot/util-crypto';
import { u8aConcat, u8aToHex, hexToU8a, compactToU8a } from '@polkadot/util';

/**
 * Generate a random 32-byte salt for commit-reveal scheme
 */
export function generateSalt(): string {
      const chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_=+[]{}|;:,.<>?';
      let saltString = '';
      const randomBytes = new Uint8Array(32);
      crypto.getRandomValues(randomBytes);
      
      for (let i = 0; i < 32; i++) {
        saltString += chars[randomBytes[i] % chars.length];
      }
      
      return saltString;
}

/**
 * Hash salt + result using Blake2-256 with SCALE encoding (matches Substrate runtime)
 * Returns hex string for display
 * 
 * This matches the Rust implementation:
 * 1. Concatenate salt + result
 * 2. SCALE encode the Vec<u8> (adds compact length prefix)
 * 3. Blake2b-256 hash the encoded data
 */
export function hashResult(salt: Uint8Array, result: Uint8Array): string {
  // Concatenate salt + result
  const combined = u8aConcat(salt, result);
  
  // SCALE encode: compact length prefix + data
  const lengthPrefix = compactToU8a(combined.length);
  const encoded = u8aConcat(lengthPrefix, combined);
  
  return blake2AsHex(encoded, 256);
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
 * This matches the runtime's blake2_256(SCALE_encode(salt + result)) calculation
 */
export function generateCommitHashBytes(salt: Uint8Array, result: Uint8Array): Uint8Array {
  // Concatenate salt + result
  const combined = u8aConcat(salt, result);
  
  // SCALE encode: compact length prefix + data
  const lengthPrefix = compactToU8a(combined.length);
  const encoded = u8aConcat(lengthPrefix, combined);
  
  return blake2AsU8a(encoded, 256);
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