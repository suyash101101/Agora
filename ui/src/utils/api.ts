import { ApiPromise, WsProvider } from '@polkadot/api';
import { DEFAULT_ENDPOINT } from './constants';

let apiInstance: ApiPromise | null = null;

export async function createApi(endpoint: string = DEFAULT_ENDPOINT): Promise<ApiPromise> {
  if (apiInstance && apiInstance.isConnected) {
    console.log('🔌 Using existing API connection');
    return apiInstance;
  }

  console.log('🔌 Connecting to blockchain:', endpoint);
  const provider = new WsProvider(endpoint);
  
  provider.on('connected', () => {
    console.log('✅ WebSocket connected to:', endpoint);
  });
  
  provider.on('disconnected', () => {
    console.warn('⚠️ WebSocket disconnected from:', endpoint);
  });
  
  provider.on('error', (error) => {
    console.error('❌ WebSocket error:', error);
  });
  
  const api = await ApiPromise.create({ provider });
  
  // Log connection info
  console.log('✅ API connected:', {
    endpoint,
    isConnected: api.isConnected,
    genesisHash: api.genesisHash.toHex(),
    runtimeVersion: api.runtimeVersion.toString(),
  });
  
  apiInstance = api;
  return api;
}

export async function disconnectApi(): Promise<void> {
  if (apiInstance) {
    await apiInstance.disconnect();
    apiInstance = null;
  }
}

export function getApi(): ApiPromise | null {
  return apiInstance;
}

export async function reconnectApi(endpoint: string): Promise<ApiPromise> {
  await disconnectApi();
  return createApi(endpoint);
}

