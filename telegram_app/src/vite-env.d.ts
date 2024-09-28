/// <reference types="vite/client" />

import { Environment } from "@usecapsule/web-sdk";

interface ImportMetaEnv {
  readonly VITE_CAPSULE_ENV: Environment;
  readonly VITE_CAPSULE_API_KEY: string;
  // Add Turnkey-related environment variables
  readonly VITE_TURNKEY_PUBLIC: string;
  readonly VITE_TURNKEY_PRIVATE: string;
  readonly VITE_TURNKEY_ORGNIZATION: string;
  readonly VITE_ENCRYPTION_KEY: string;
  readonly VITE_RPC_URL: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
