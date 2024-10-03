import { ec as EC } from "elliptic";
import { type ClassValue, clsx } from "clsx";
import axios from "axios";
import * as crypto from "crypto";
import { twMerge } from "tailwind-merge";
import { Turnkey } from "@turnkey/sdk-server";
import { TurnkeySigner } from "@turnkey/solana";
import {
  SendTransactionError,
  LAMPORTS_PER_SOL,
  sendAndConfirmTransaction,
} from "@solana/web3.js";

import {
  Connection,
  PublicKey,
  Transaction,
  SystemProgram,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import { TelegramApi } from "../telegram/telegram-api";
import { log } from "console";
import WebApp from "@twa-dev/sdk";
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

const MAX_RETRIES = 3;
const RETRY_DELAY = 2000; // 2 seconds
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

export async function transferSOL(
  from: string,
  to: string,
  amount: number,
  user_json_string: string
) {
  let user: any;
  try {
    user = JSON.parse(user_json_string);
  } catch (parseError) {
    throw new Error("Invalid user data provided.");
  }
  // Initialize connection to the Solana cluster
  const connection = new Connection(import.meta.env.VITE_RPC_URL, "confirmed");
  try {
    // turnkey client
    const turnkeyClient = new Turnkey({
      apiBaseUrl: "https://api.turnkey.com",
      apiPrivateKey: user.privateKey,
      apiPublicKey: user.publicKey,
      defaultOrganizationId: user.subOrgId,
    });
    const turnkeySigner = new TurnkeySigner({
      organizationId: user.subOrgId,
      client: turnkeyClient.apiClient(),
    });

    const fromPublicKey = new PublicKey(from);
    const toPublicKey = new PublicKey(to);

    // Fetch the sender's balance
    const balance = await connection.getBalance(fromPublicKey);

    // Get the recent blockhash
    const { blockhash, lastValidBlockHeight } =
      await connection.getLatestBlockhash();

    // Create a dummy transaction to calculate the fee
    const transaction = new Transaction({
      recentBlockhash: blockhash,
      feePayer: fromPublicKey,
    }).add(
      SystemProgram.transfer({
        fromPubkey: fromPublicKey,
        toPubkey: toPublicKey,
        lamports: 1,
      })
    );

    // Estimate the fee
    const feeCalculator = await connection.getFeeForMessage(
      transaction.compileMessage()
    );
    const fee = feeCalculator.value;

    if (fee === null) {
      throw new Error("Failed to calculate transaction fee.");
    }

    // Convert SOL to lamports
    let lamportsAmount = Math.round(amount * 1e9);

    // Check if the sender has enough balance
    if (lamportsAmount + fee > balance) {
      // Adjust the transfer amount
      const maxTransferableLamports = balance - fee;
      if (maxTransferableLamports <= 0) {
        return {
          success: false,
          error: "Insufficient balance to cover the transaction fee.",
        };
      }

      lamportsAmount = maxTransferableLamports;
    }

    // Create the actual transfer transaction
    const transferTransaction = new Transaction({
      recentBlockhash: blockhash,
      feePayer: fromPublicKey,
    }).add(
      SystemProgram.transfer({
        fromPubkey: fromPublicKey,
        toPubkey: toPublicKey,
        lamports: lamportsAmount,
      })
    );

    // Sign the transaction with Turnkey
    await turnkeySigner.addSignature(
      transferTransaction,
      fromPublicKey.toString()
    );

    // Send and confirm the transaction
    const signature = await connection.sendRawTransaction(
      transferTransaction.serialize()
    );

    // Confirm the transaction
    const confirmation = await connection.confirmTransaction({
      signature,
      blockhash,
      lastValidBlockHeight,
    });

    if (confirmation.value.err) {
      throw new Error(
        `Transaction failed: ${JSON.stringify(confirmation.value.err)}`
      );
    }

    return {
      success: true,
      signature,
      confirmation,
      transferredAmount: amount,
    };
  } catch (error) {
    if (error instanceof SendTransactionError) {
      // Handle SendTransactionError and log details
      const logs = (await error.getLogs(connection)) ?? ["No logs available"];
      throw new Error(`Failed to transfer SOL: ${logs}`);
    } else if (axios.isAxiosError(error)) {
      // Handle Axios errors
      const errorMessage = error.response
        ? `Server responded with status ${
            error.response.status
          }: ${JSON.stringify(error.response.data)}`
        : `Network error: ${error.message}`;
      throw new Error(`Failed to transfer SOL: ${errorMessage}`);
    } else {
      // Handle other types of errors
      throw new Error(
        `Unexpected error: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }
  }
}

export async function getAllTokensBalance(user_json_string: string) {
  let user = JSON.parse(user_json_string);
  const connection = new Connection(import.meta.env.VITE_RPC_URL);
  const publicKey = new PublicKey(user.account_address);
  const tokens = await connection.getParsedTokenAccountsByOwner(publicKey, {
    programId: new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
  });
  return tokens;
}

export async function copyTrade(data: any) {
  try {
    let user = await TelegramApi.getItem(
      `user_${WebApp.initDataUnsafe.user?.id}`
    );
    let json_user = JSON.parse(user);
    json_user.tgUserId = data.tgUserId;

    if (json_user.sessionApiKeys !== "") {
      log("Session active", "success");
      // Get turnkey client
      const turnkey = new Turnkey({
        apiBaseUrl: "https://api.turnkey.com",
        apiPublicKey: json_user.sessionApiKeys.publicKey,
        apiPrivateKey: json_user.sessionApiKeys.privateKey,
        defaultOrganizationId: json_user.subOrgId,
      });
      let turnkeyClient = turnkey.apiClient();

      // Get turnkey signer
      const turnkeySigner = new TurnkeySigner({
        organizationId: json_user.subOrgId,
        client: turnkeyClient,
      });

      let connection = new Connection(import.meta.env.VITE_RPC_URL);
      // Create a buffer from the transaction
      const transactionBuffer = Buffer.from(data.swapTransaction, "base64");

      // Create a transaction obj from the buffer
      let transaction = VersionedTransaction.deserialize(transactionBuffer);
      log(`Transaction deserialized`, "success");
      // Sign the transaction with the turnkey signer
      await turnkeySigner.addSignature(
        transaction,
        json_user.accounts[0].address
      );
      log(`Transaction signed by turnkey`, "success");

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
            } else {
              log(
                `Transaction failed after ${MAX_RETRIES} attempts: ${error}`,
                "error"
              );
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
