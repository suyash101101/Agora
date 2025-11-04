import { BN } from '@polkadot/util';
import { UNIT } from './constants';

export function formatBalance(balance: bigint | BN | string | number, decimals: number = 12): string {
  const balanceBN = typeof balance === 'bigint' ? new BN(balance.toString()) : new BN(balance.toString());
  const divisor = new BN(10).pow(new BN(decimals));
  const whole = balanceBN.div(divisor);
  const fraction = balanceBN.mod(divisor);
  
  if (fraction.isZero()) {
    return whole.toString();
  }
  
  const fractionStr = fraction.toString().padStart(decimals, '0');
  const trimmed = fractionStr.replace(/\.?0+$/, '');
  
  return `${whole.toString()}.${trimmed}`;
}

export function parseBalance(amount: string, decimals: number = 12): BN {
  const [whole, fraction = ''] = amount.split('.');
  const fractionPadded = fraction.padEnd(decimals, '0').slice(0, decimals);
  const wholeBN = new BN(whole || '0');
  const fractionBN = new BN(fractionPadded);
  const divisor = new BN(10).pow(new BN(decimals));
  
  return wholeBN.mul(divisor).add(fractionBN);
}

export function formatAddress(address: string, length: number = 8): string {
  if (address.length <= length * 2) return address;
  return `${address.slice(0, length)}...${address.slice(-length)}`;
}

export function formatBlockNumber(blockNumber: bigint | number | string): string {
  return new BN(blockNumber.toString()).toString();
}

export function formatTimestamp(blockNumber: bigint | number, blockTimeMs: number = 6000): string {
  const timestamp = Number(blockNumber) * blockTimeMs;
  return new Date(timestamp).toLocaleString();
}

export function formatDeadline(blockNumber: bigint | number, currentBlock: bigint | number, blockTimeMs: number = 6000): string {
  const blocksRemaining = Number(blockNumber) - Number(currentBlock);
  if (blocksRemaining <= 0) return 'Expired';
  
  const secondsRemaining = (blocksRemaining * blockTimeMs) / 1000;
  const minutes = Math.floor(secondsRemaining / 60);
  const seconds = Math.floor(secondsRemaining % 60);
  
  if (minutes > 0) {
    return `${minutes}m ${seconds}s`;
  }
  return `${seconds}s`;
}

export function hexToString(hex: string | Uint8Array): string {
  if (typeof hex === 'string') {
    return hex.startsWith('0x') ? hex.slice(2) : hex;
  }
  return Array.from(hex)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

export function stringToHex(str: string): string {
  return '0x' + Array.from(str)
    .map(char => char.charCodeAt(0).toString(16).padStart(2, '0'))
    .join('');
}

export function bytesToString(bytes: Uint8Array): string {
  return new TextDecoder().decode(bytes);
}

export function stringToBytes(str: string): Uint8Array {
  return new TextEncoder().encode(str);
}

