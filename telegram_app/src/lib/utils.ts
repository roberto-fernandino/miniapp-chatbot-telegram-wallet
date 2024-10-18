import { ec as EC } from "elliptic";
import { type ClassValue, clsx } from "clsx";
import axios from "axios";
import * as crypto from "crypto";
import { twMerge } from "tailwind-merge";
import { Turnkey } from "@turnkey/sdk-server";
import { TurnkeySigner as SolanaTurnkeySigner } from "@turnkey/solana";
import { SendTransactionError } from "@solana/web3.js";
import { Connection, PublicKey, VersionedTransaction } from "@solana/web3.js";
import { TelegramApi } from "../telegram/telegram-api";
import { log } from "console";
import WebApp from "@twa-dev/sdk";
import {
  DEFAULT_SOLANA_ACCOUNTS,
  DEFAULT_ETHEREUM_ACCOUNTS,
} from "@turnkey/sdk-browser";

const MAX_RETRIES = 3;
const RETRY_DELAY = 1000; // 1 second

// formatUnits is a utility function for converting a value from its smallest unit (e.g., wei) to a larger unit (e.g., ether)
import { formatUnits } from "@ethersproject/units";

// Api to fetch ETH data
import Moralis from "moralis";

/**
 * Get all Ethereum tokens balance.
 * @param {string} address - The Ethereum address.
 * @returns {Promise<any>} The balance of the Ethereum address.
 */
export async function getAllEthereumTokensBalance(address: string) {
  try {
    await Moralis.start({
      apiKey: import.meta.env.VITE_MORALIS_API_KEY,
    });

    const response = await Moralis.EvmApi.token.getWalletTokenBalances({
      address: address,
      chain: "0x1",
    });
    return response.toJSON();
  } catch (error) {
    throw error;
  }
}

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

const BASE_URL_API = "https://srv617785.hstgr.cloud/api";
const BOT_API_URL = "https://srv617785.hstgr.cloud/bot_api";

export interface CopyTradeWalletData {
  user_id: string;
  wallet_id: string;
  account_address: string;
  buy_amount: string;
  copy_trade_address: string;
  status: string;
}

/**
 * Get the balance of a Solana address.
 * @param {string} address - The Solana address.
 * @returns {Promise<string>} The balance in SOL.
 */
export async function getSolBalance(address: string): Promise<string> {
  const rpcUrl = import.meta.env.VITE_RPC_URL;

  if (!rpcUrl.startsWith("http://") && !rpcUrl.startsWith("https://")) {
    throw new Error("VITE_RPC_URL must start with http:// or https://");
  }

  const connection = new Connection(rpcUrl);
  const publicKey = new PublicKey(address);
  const balance = await connection.getBalance(publicKey);
  return (balance / 1e9).toFixed(4); // Convert lamports to SOL and format to 4 decimal places
}

/**
 * Get the balance of an Ethereum address.
 * @param {string} address - The Ethereum address.
 * @returns {Promise<string>} The balance in ETH.
 */
export async function getEthBalance(address: string): Promise<string> {
  try {
    await Moralis.start({
      apiKey: import.meta.env.VITE_MORALIS_API_KEY,
    });
  } catch (error) {
    throw error;
  }
  const response = await Moralis.EvmApi.balance.getNativeBalance({
    chain: "0x1",
    address: address,
  });
  return formatUnits(response.raw.balance, 18); // Convert wei to ETH
}

/**
 * Create a payload for the copy trade wallet.
 * @param {string} user_id - The user ID.
 * @param {string} wallet_id - The wallet ID.
 * @param {string} account_address - The account address.
 * @param {string} buy_amount - The buy amount.
 * @param {string} copy_trade_address - The copy trade address.
 * @param {string} status - The status.
 * @returns {string} The payload.
 */
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

/**
 * Creates an Ethereum Virtual Machine (EVM) account using the Turnkey API.
 *
 * This function retrieves user data from TelegramApi, initializes a Turnkey client,
 * and creates a new wallet account for the user.
 *
 * @async
 * @function createEvmAccount
 * @returns {Promise<string>} The address of the created EVM account.
 * @throws {Error} If the user data is invalid or cannot be parsed.
 */
export async function createEvmAccount(user_json: any): Promise<string> {
  // Initialize the Turnkey client with user-specific credentials
  const turnkeyClient = new Turnkey({
    apiBaseUrl: "https://api.turnkey.com",
    apiPrivateKey: user_json.privateKey,
    apiPublicKey: user_json.publicKey,
    defaultOrganizationId: user_json.subOrgId,
  });

  // Create a new wallet account using the Turnkey API
  let response = await turnkeyClient.apiClient().createWalletAccounts({
    walletId: user_json.walletId,
    accounts: DEFAULT_ETHEREUM_ACCOUNTS,
    organizationId: user_json.subOrgId,
  });

  // Return the API response
  return response.addresses[0];
}

/**
 * Checks if the user has any accounts on the platform.
 *
 * This function retrieves user data from TelegramApi, initializes a Turnkey client,
 * and checks if the user has any accounts on the platform.
 *
 * @async
 * @function checkUserAccounts
 * @returns {Promise<{
 *  has_solana: boolean;
 *  has_evm: boolean;
 *  has_sui: boolean;
 * }>} The response from the Turnkey API after creating the wallet account.
 **/
export async function checkUserAccounts(user_json: any): Promise<{
  has_solana: boolean;
  has_evm: boolean;
  has_sui: boolean;
}> {
  const turnkeyClient = new Turnkey({
    apiBaseUrl: "https://api.turnkey.com",
    apiPrivateKey: user_json.privateKey,
    apiPublicKey: user_json.publicKey,
    defaultOrganizationId: user_json.subOrgId,
  });
  let response = await turnkeyClient.apiClient().getWalletAccounts({
    walletId: user_json.walletId,
    organizationId: user_json.subOrgId,
  });
  let has_solana = false;
  let has_evm = false;
  let has_sui = false;
  for (let account of response.accounts) {
    if (account.addressFormat === "ADDRESS_FORMAT_SOLANA") {
      has_solana = true;
    }
    if (account.addressFormat === "ADDRESS_FORMAT_ETHEREUM") {
      has_evm = true;
    }
    if (account.addressFormat === "ADDRESS_FORMAT_SUI") {
      has_sui = true;
    }
  }
  return {
    has_solana,
    has_evm,
    has_sui,
  };
}

/**
 * Creates a Solana account using the Turnkey API.
 *
 * This function retrieves user data from TelegramApi, initializes a Turnkey client,
 * and creates a new wallet account for the user.
 *
 * @async
 * @function createSolanaAccount
 * @param {any} json_user - The user data from TelegramApi.
 * @returns {Promise<string>} The address of the created Solana account.
 * @throws {Error} If the user data is invalid or cannot be parsed.
 */
export async function createSolanaAccount(json_user: any): Promise<string> {
  const turnkeyClient = new Turnkey({
    apiBaseUrl: "https://api.turnkey.com",
    apiPrivateKey: json_user.privateKey,
    apiPublicKey: json_user.publicKey,
    defaultOrganizationId: json_user.subOrgId,
  });
  let response = await turnkeyClient.apiClient().createWalletAccounts({
    walletId: json_user.walletId,
    accounts: DEFAULT_SOLANA_ACCOUNTS,
    organizationId: json_user.subOrgId,
  });
  return response.addresses[0];
}

export async function copyTrade(data: any) {
  try {
    let user = await TelegramApi.getItem(
      `user_${WebApp.initDataUnsafe.user?.id}`
    );
    let json_user = JSON.parse(user);
    if (json_user.sessionApiKeys !== "") {
      // Get turnkey client
      const turnkey = new Turnkey({
        apiBaseUrl: "https://api.turnkey.com",
        apiPublicKey: json_user.sessionApiKeys.publicKey,
        apiPrivateKey: json_user.sessionApiKeys.privateKey,
        defaultOrganizationId: json_user.subOrgId,
      });
      let turnkeyClient = turnkey.apiClient();

      // Get turnkey signer
      const turnkeySigner = new SolanaTurnkeySigner({
        organizationId: json_user.subOrgId,
        client: turnkeyClient,
      });

      let connection = new Connection(import.meta.env.VITE_RPC_URL);
      // Create a buffer from the transaction
      const transactionBuffer = Buffer.from(data.swapTransaction, "base64");

      // Create a transaction obj from the buffer
      let transaction = VersionedTransaction.deserialize(transactionBuffer);
      // Sign the transaction with the turnkey signer
      await turnkeySigner.addSignature(
        transaction,
        json_user.accounts[0].address
      );

      let retries = 0;
      let success = false;

      while (retries < MAX_RETRIES && !success) {
        try {
          log(`Sending transaction (attempt ${retries + 1})`, "info");
          const signature = await connection.sendRawTransaction(
            transaction.serialize()
          );
          log(`Transaction sent to jupiter`, "success");
          log(`Waiting for blockchain validation`, "info");
          const latestBlockHash = await connection.getLatestBlockhash();
          const confirmation = await connection.confirmTransaction({
            signature,
            ...latestBlockHash,
          });
          log(`RPC Response: ${JSON.stringify(confirmation)}`, "success");
          log(
            `Confirmed tx, check:\n https://solscan.io/tx/${signature}`,
            "success"
          );
          success = true;
          return {
            success: success,
            signature: signature,
            blockHash: latestBlockHash.blockhash,
          };
        } catch (error) {
          retries++;
          if (retries < MAX_RETRIES) {
            log(
              `Transaction failed. Retrying in ${
                RETRY_DELAY / 1000
              } seconds...`,
              "info"
            );
            await new Promise((resolve) => setTimeout(resolve, RETRY_DELAY));
          } else {
            if (error instanceof SendTransactionError) {
              log(
                `Transaction failed after ${MAX_RETRIES} attempts: ${error.message}`,
                "error"
              );
              let logs = log(`Error: ${error.getLogs(connection)}`, "error");
              log(`Logs: ${JSON.stringify(logs)}`, "error");
              return {
                success: false,
                signature: "",
                blockHash: "",
              };
            } else {
              log(
                `Transaction failed after ${MAX_RETRIES} attempts: ${error}`,
                "error"
              );
              return {
                success: false,
                signature: "",
                blockHash: "",
              };
            }
          }
        }
      }

      if (!success) {
        log("Failed to send transaction after multiple attempts", "error");
      }
    } else {
      log("Session not active", "error");
    }
  } catch (error) {
    log(`Error in checkUserSessionAndCopyTrade: ${error}`, "error");
  }
}

export async function fetchTokenPrice(tokenMint: string) {
  try {
    const response = await axios.get(
      `https://api-v3.raydium.io/mint/price?mints=${tokenMint}`
    );
    return response.data;
  } catch (error) {
    throw error;
  }
}

export async function getTokenInfo(tokenMint: string) {
  try {
    const response = await axios.get(
      `https://api-v3.raydium.io/mint/ids?mints=${tokenMint}`
    );
    return response.data;
  } catch (error) {
    throw error;
  }
}

export async function getTokenData(tokenMint: string) {
  try {
    const info_response = await axios.get(
      `https://api-v3.raydium.io/mint/ids?mints=${tokenMint}`
    );
    const price_response = await axios.get(
      `https://api-v3.raydium.io/mint/price?mints=${tokenMint}`
    );
    return {
      info: info_response.data,
      price: price_response.data,
    };
  } catch (error) {
    throw error;
  }
}

export async function getUserFirstCalls(userId: string) {
  const response = await axios.get(`${BOT_API_URL}/user_calls/${userId}`);
  return response.data;
}

export function formatNumber(value: number): string {
  if (value >= 1_000_000) {
    return (value / 1_000_000).toFixed(2) + "M";
  } else if (value >= 1_000) {
    return (value / 1_000).toFixed(2) + "K";
  } else {
    return value.toString();
  }
}

export function formatTime(isoString: string): string {
  const date = new Date(isoString);
  return date.toLocaleString("en-GB", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    day: "2-digit",
    month: "2-digit",
  });
}
