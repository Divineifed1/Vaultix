import { useState } from 'react';
import { WalletHookReturn } from '@/types/escrow';

export const useWallet = (): WalletHookReturn => {
  const [connected, setConnected] = useState(false);
  const [publicKey, setPublicKey] = useState<string | null>(null);

  const connect = () => {
    // Mock implementation - in real app this would connect to a wallet provider
    setConnected(true);
    setPublicKey('GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H'); // Mock public key
  };

  return {
    connected,
    publicKey,
    connect
  };
};