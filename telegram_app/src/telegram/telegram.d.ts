import type { Telegram } from "@twa-dev/types";

declare global {
  interface Window {
    Telegram?: Telegram;
  }
}

export {};
