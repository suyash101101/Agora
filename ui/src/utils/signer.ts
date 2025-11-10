import type { KeyringPair } from '@polkadot/keyring/types';
import type { Signer } from '@polkadot/types/types';

/**
 * Helper function to determine if a signer is a keypair (test account) or extension signer
 * Based on Polkadot.js documentation patterns
 */
export function isKeypair(signer: any): signer is KeyringPair {
  return signer !== null && 
         typeof signer === 'object' &&
         'sign' in signer && 
         'address' in signer && 
         typeof signer.sign === 'function' &&
         !('signPayload' in signer) && 
         !('signRaw' in signer);
}

/**
 * Helper function to sign and send a transaction, handling both keypair and extension signers
 * 
 * Pattern based on official Polkadot.js documentation:
 * - Keyring pairs: tx.signAndSend(keypair, callback)
 * - Extension signers: tx.signAndSend(address, { signer }, callback)
 * 
 * @see https://polkadot.js.org/docs/extension/cookbook#sign-and-send-a-transaction
 * @see https://polkadot.js.org/docs/extension/usage
 * 
 * @param tx - The transaction to sign and send
 * @param signer - Either a KeyringPair (for test accounts) or Signer (for extension)
 * @param address - The address of the account (required for extension signers)
 * @param callback - Callback function that receives transaction status updates
 * @returns Promise that resolves with the unsubscribe function
 */
export async function signAndSend(
  tx: any,
  signer: any,
  address: string,
  callback: (result: any) => void | Promise<void>
): Promise<() => void> {
  if (!signer) {
    throw new Error('No signer available');
  }

  console.log('🔐 Signing transaction:', {
    method: tx.method.method,
    section: tx.method.section,
    args: tx.method.args.map((a: any) => a.toString()),
    signerType: isKeypair(signer) ? 'KeyringPair' : 'Extension',
    address,
  });

  // Log transaction details before signing
  const txHex = tx.toHex();
  console.log('📝 Transaction hex:', txHex);
  console.log('📝 Transaction length:', txHex.length);

  if (isKeypair(signer)) {
    // Keypair (test account) - pass directly as first parameter
    // Pattern: tx.signAndSend(keypair, callback)
    console.log('✅ Using KeyringPair signer for test account');
    const unsubscribePromise = tx.signAndSend(signer, (result: any) => {
      console.log('📡 Transaction callback received:', {
        status: result.status.type,
        txHash: result.txHash?.toHex(),
        blockHash: result.status.isInBlock ? result.status.asInBlock.toHex() : 
                   result.status.isFinalized ? result.status.asFinalized.toHex() : null,
      });
      callback(result);
    });
    
    // Handle both Promise and direct function returns
    if (unsubscribePromise && typeof unsubscribePromise.then === 'function') {
      return unsubscribePromise.then((unsub: any) => {
        console.log('✅ Transaction subscription established');
        return unsub;
      });
    } else if (typeof unsubscribePromise === 'function') {
      console.log('✅ Transaction subscription established (direct function)');
      return unsubscribePromise;
    } else {
      console.warn('⚠️ Unexpected unsubscribe return type:', unsubscribePromise);
      return () => {};
    }
  } else {
    // Extension signer - use options format with address
    // Pattern: tx.signAndSend(address, { signer }, callback)
    // @see https://polkadot.js.org/docs/extension/cookbook
    console.log('✅ Using Extension signer');
    const unsubscribePromise = tx.signAndSend(address, { signer }, (result: any) => {
      console.log('📡 Transaction callback received:', {
        status: result.status.type,
        txHash: result.txHash?.toHex(),
        blockHash: result.status.isInBlock ? result.status.asInBlock.toHex() : 
                   result.status.isFinalized ? result.status.asFinalized.toHex() : null,
      });
      callback(result);
    });
    
    // Handle both Promise and direct function returns
    if (unsubscribePromise && typeof unsubscribePromise.then === 'function') {
      return unsubscribePromise.then((unsub: any) => {
        console.log('✅ Transaction subscription established');
        return unsub;
      });
    } else if (typeof unsubscribePromise === 'function') {
      console.log('✅ Transaction subscription established (direct function)');
      return unsubscribePromise;
    } else {
      console.warn('⚠️ Unexpected unsubscribe return type:', unsubscribePromise);
      return () => {};
    }
  }
}

