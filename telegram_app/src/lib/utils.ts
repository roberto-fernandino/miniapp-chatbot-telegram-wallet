import { type ClassValue, clsx } from "clsx";
import axios from "axios";
import * as crypto from "crypto";
import { twMerge } from "tailwind-merge";
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

type UserData = {
  id: number;
  first_name: string;
  last_name: string;
  username: string;
  language_code: string;
  allows_write_to_pm: boolean;
};

export function parseUserData(userData: any) {
  return {
    id: userData.id.toString(),
    first_name: userData.first_name,
    last_name: userData.last_name,
    username: userData.username,
    language_code: userData.language_code,
    allows_write_to_pm: userData.allows_write_to_pm,
  };
}

// Add or update user in redis
export async function addOrUpdateUser(userData: UserData) {
  try {
    const response = await axios.post(
      "https://selected-namely-panda.ngrok-free.app/api/add_or_update_user",
      JSON.stringify(userData),
      {
        headers: {
          "Content-Type": "application/json",
          "User-Agent": "TelegramBot/1.0",
        },
        timeout: 5000, // 5 seconds timeout
      }
    );

    return response;
  } catch (error) {
    console.error("Error in addOrUpdateUser:", error);
    if (axios.isAxiosError(error) && error.response) {
      console.error("Response status:", error.response.status);
      console.error("Response data:", error.response.data);
    }
    throw error;
  }
}

export interface WalletData {
  user_id: string;
  wallet_id: string;
  turnkey_wallet_name: string;
  user_wallet_name: string;
  sol_address: string;
}
export function createWalletPayload(
  user_id: string,
  wallet_id: string,
  turnkey_wallet_name: string,
  user_wallet_name: string,
  sol_address: string
): string {
  const payload: WalletData = {
    user_id,
    wallet_id,
    turnkey_wallet_name,
    user_wallet_name,
    sol_address,
  };
  return JSON.stringify(payload);
}

export async function addWalletToUser(
  user_id: string,
  wallet_id: string,
  turnkey_wallet_name: string,
  user_wallet_name: string,
  sol_address: string
) {
  try {
    const response = await axios.post(
      "https://selected-namely-panda.ngrok-free.app/api/add_wallet_to_user",
      createWalletPayload(
        user_id,
        wallet_id,
        turnkey_wallet_name,
        user_wallet_name,
        sol_address
      ),
      {
        headers: {
          "Content-Type": "application/json",
          "User-Agent": "TelegramBot/1.0",
        },
        timeout: 5000, // 5 seconds timeout
      }
    );

    return response;
  } catch (error) {
    console.error("Error in addWalletToUser:", error);
    if (axios.isAxiosError(error) && error.response) {
      console.error("Response status:", error.response.status);
      console.error("Response data:", error.response.data);
    }
    throw error;
  }
}

export async function getUserWallets(user_id: string) {
  try {
    const response = await axios.get(
      `https://selected-namely-panda.ngrok-free.app/api/user_wallets/${user_id}`,
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

export interface CopyTradeWalletData {
  user_id: string;
  wallet_id: string;
  buy_amount: string;
  copy_trade_address: string;
  status: string;
  user_wallet_name: string;
}

function createCopyTradeWalletPayload(
  user_id: string,
  wallet_id: string,
  buy_amount: string,
  copy_trade_address: string,
  status: string,
  user_wallet_name: string
): string {
  const payload: CopyTradeWalletData = {
    user_id,
    wallet_id,
    buy_amount,
    copy_trade_address,
    status,
    user_wallet_name,
  };
  return JSON.stringify(payload);
}

export async function setCopyTradeWallet(
  user_id: string,
  wallet_id: string,
  buy_amount: string,
  copy_trade_address: string,
  status: string,
  user_wallet_name: string
) {
  try {
    if (!wallet_id) {
      throw new Error("Wallet ID is required");
    }
    const payload = createCopyTradeWalletPayload(
      user_id,
      wallet_id,
      buy_amount,
      copy_trade_address,
      status,
      user_wallet_name
    );
    const response = await axios.post(
      "https://selected-namely-panda.ngrok-free.app/api/set_copy_trade_wallet",
      payload,
      {
        headers: {
          "Content-Type": "application/json",
          "User-Agent": "TelegramBot/1.0",
        },
        timeout: 5000, // 5 seconds timeout
      }
    );
    return response.data;
  } catch (error) {
    throw error;
  }
}

export async function getCopyTrades(user_id: string) {
  try {
    const response = await axios.get(
      `https://selected-namely-panda.ngrok-free.app/api/get_copy_trades/${user_id}`,
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
