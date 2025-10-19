import { cryptoWaitReady, mnemonicGenerate, mnemonicToMiniSecret, sr25519PairFromSeed } from '@polkadot/util-crypto';
import { u8aToHex } from '@polkadot/util';

const main = async () => {
  await cryptoWaitReady();

  const mnemonic = mnemonicGenerate();
  const seed = mnemonicToMiniSecret(mnemonic);
  const pair = sr25519PairFromSeed(seed);

  console.log('Mnemonic:', mnemonic);
  console.log('Public Key:', u8aToHex(pair.publicKey));
  console.log('Secret Key:', u8aToHex(pair.secretKey));
};

main();
