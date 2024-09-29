import { ec as EC } from "elliptic";
import { type ClassValue, clsx } from "clsx";
import axios from "axios";
import * as crypto from "crypto";
import { twMerge } from "tailwind-merge";
import { Connection, PublicKey } from "@solana/web3.js";
import { TelegramApi } from "../telegram/telegram-api";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
const BASE_URL_API = "https://woodcock-engaging-usually.ngrok-free.app/api";
export interface CopyTradeWalletData {
  user_id: string;
  wallet_id: string;
  account_address: string;
  buy_amount: string;
  copy_trade_address: string;
  status: string;
}

function createCopyTradeWalletPayload(
  user_id: string,
  wallet_id: string,
  account_address: string,
  buy_amount: string,
  copy_trade_address: string,
  status: string
): string {
  const payload: CopyTradeWalletData = {
    user_id,
    wallet_id,
    account_address,
    buy_amount,
    copy_trade_address,
    status,
  };
  return JSON.stringify(payload);
}

export async function setCopyTradeWallet(
  user_id: string,
  wallet_id: string,
  account_address: string,
  buy_amount: string,
  copy_trade_address: string,
  status: string
) {
  try {
    if (!wallet_id) {
      throw new Error("Wallet ID is required");
    }
    const payload = createCopyTradeWalletPayload(
      user_id,
      wallet_id,
      account_address,
      buy_amount,
      copy_trade_address,
      status
    );
    const response = await axios.post(
      `${BASE_URL_API}/set_copy_trade_wallet`,
      payload,
      {
        headers: {
          "Content-Type": "application/json",
          "User-Agent": "TelegramBot/1.0",
        },
        timeout: 10000, // 10 seconds timeout
      }
    );
    return response.data;
  } catch (error) {
    if (axios.isAxiosError(error)) {
      // Handle Axios errors
      const errorMessage = error.response
        ? `Server responded with status ${
            error.response.status
          }: ${JSON.stringify(error.response.data)}`
        : `Network error: ${error.message}`;
      console.error(`Axios error in setCopyTradeWallet: ${errorMessage}`);
      throw new Error(`Failed to set copy trade wallet: ${errorMessage}`);
    } else {
      // Handle other types of errors
      console.error(`Unexpected error in setCopyTradeWallet:`, error);
      throw new Error(
        `Unexpected error: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }
  }
}

export async function getCopyTrades(user_id: string) {
  try {
    const response = await axios.get(
      `${BASE_URL_API}/get_copy_trades/${user_id}`,
      {
        headers: {
          "User-Agent": "TelegramBot/1.0",
        },
      }
    );
    return response;
  } catch (error) {
    throw error;
  }
}

export const decryptPassword = (encryptedPassword: string): string => {
  const textParts = encryptedPassword.split(":");
  const iv = Buffer.from(textParts.shift()!, "hex");
  const encryptedText = Buffer.from(textParts.join(":"), "hex");
  const decipher = crypto.createDecipheriv(
    "aes-256-cbc",
    Buffer.from(import.meta.env.VITE_ENCRYPTION_KEY!),
    iv
  );
  let decrypted = decipher.update(encryptedText);
  decrypted = Buffer.concat([decrypted, decipher.final()]);
  return decrypted.toString();
};

export const encryptPassword = (password: string): string => {
  const iv = crypto.randomBytes(16);
  const cipher = crypto.createCipheriv(
    "aes-256-cbc",
    Buffer.from(import.meta.env.VITE_ENCRYPTION_KEY!),
    iv
  );
  let encrypted = cipher.update(password);
  encrypted = Buffer.concat([encrypted, cipher.final()]);
  return iv.toString("hex") + ":" + encrypted.toString("hex");
};

export function generateKeyPair() {
  const ec = new EC("p256"); // Use P-256 curve
  const key = ec.genKeyPair();
  const publicKey = key.getPublic(true, "hex"); // Get public key in hex format
  const privateKey = key.getPrivate("hex"); // Get private key in hex format
  return { publicKey, privateKey };
}

export async function getBalance(address: string): Promise<string> {
  const connection = new Connection(import.meta.env.VITE_RPC_URL);
  const publicKey = new PublicKey(address);
  const balance = await connection.getBalance(publicKey);
  return (balance / 1e9).toFixed(4); // Convert lamports to SOL and format to 4 decimal places
}

export async function getSOLPrice(): Promise<number> {
  const response = await fetch(
    "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd"
  );
  const data = await response.json();
  return data.solana.usd;
}

export async function deleteCopyTradeWallet(
  user_id: string,
  copy_trade_address: string
) {
  const response = await axios.delete(
    `${BASE_URL_API}/delete_copy_trade_wallet/${user_id}/${copy_trade_address}`
  );
  return response.data;
}

export async function setUserSession(user_id: string) {
  try {
    const user = await TelegramApi.getItem(`user_${user_id}`);
    if (!user) {
      throw new Error(`User data not found for user_id: ${user_id}`);
    }

    const json_user = JSON.parse(user);
    const payload = {
      user_id: json_user.tgUserId.toString(),
      session_end_time: json_user.sessionApiKeys.expirationDate ?? "null",
      public_key: json_user.sessionApiKeys.publicKey ?? "null",
      private_key: json_user.sessionApiKeys.privateKey ?? "null",
    };

    const response = await axios.post(
      `${BASE_URL_API}/set_user_session`,
      payload,
      {
        headers: {
          "Content-Type": "application/json",
          "User-Agent": "TelegramBot/1.0",
        },
        timeout: 10000, // 10 seconds timeout
      }
    );

    return response.data;
  } catch (error) {
    if (axios.isAxiosError(error)) {
      // Handle Axios errors
      const errorMessage = error.response
        ? `Server responded with status ${
            error.response.status
          }: ${JSON.stringify(error.response.data)}`
        : `Network error: ${error.message}`;
      console.error(`Axios error in setUserSession: ${errorMessage}`);
      throw new Error(`Failed to set user session: ${errorMessage}`);
    } else {
      // Handle other types of errors
      console.error(`Unexpected error in setUserSession:`, error);
      throw new Error(
        `Unexpected error: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }
  }
}
