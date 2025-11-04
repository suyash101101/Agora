import { ApiPromise, WsProvider } from '@polkadot/api';
import { DEFAULT_ENDPOINT } from './constants';

let apiInstance: ApiPromise | null = null;

export async function createApi(endpoint: string = DEFAULT_ENDPOINT): Promise<ApiPromise> {
  if (apiInstance && apiInstance.isConnected) {
    return apiInstance;
  }

  const provider = new WsProvider(endpoint);
  const api = await ApiPromise.create({ provider });
  
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

