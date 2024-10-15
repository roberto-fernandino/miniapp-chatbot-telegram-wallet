import {
  Connection,
  PublicKey,
  SendTransactionError,
  SystemProgram,
  Transaction,
  VersionedTransaction,
} from "@solana/web3.js";
import { TurnkeySigner as SolanaTurnkeySigner } from "@turnkey/solana";
import axios from "axios";
import { TelegramApi } from "../telegram/telegram-api";
import WebApp from "@twa-dev/sdk";
import { Turnkey } from "@turnkey/sdk-server";
import { log } from "console";
const MAX_RETRIES = 3;
const RETRY_DELAY = 2000; // 2 seconds

/**
 * Get the SOL balance of a wallet.
 * @param walletAddress - The address of the wallet.
 * @returns - The SOL balance of the wallet.
 */
export async function getWalletSolBalance(walletAddress: string) {
  const connection = new Connection(import.meta.env.VITE_RPC_URL!, "confirmed");

  try {
    const publicKey = new PublicKey(walletAddress);

    const balance = await connection.getBalance(publicKey);

    const solBalance = balance / 10 ** 9;
    const usdtBalance = solBalance * (await getSOLPrice());

    return { solBalance, usdtBalance };
  } catch (error) {
    console.error("Error fetching SOL balance:", error);
    throw error;
  }
}

/**
 * Check if the given address is a valid Solana address.
 * @param address - The address to check.
 * @returns - True if the address is valid, false otherwise.
 */
export function isValidSolanaAddress(address: string): boolean {
  try {
    new PublicKey(address);
    return true;
  } catch (error) {
    return false;
  }
}

interface SolToken {
  symbol: string;
  name: string;
  logoURI: string;
  balance: string;
  address: string;
}

/**
 * Swap SOL tokens.
 * @param userPublicKey - The public key of the user.
 * @param toToken - The token to swap to.
 * @param fromToken - The token to swap from.
 * @param fromAmount - The amount of tokens to swap.
 * @param slippage - The slippage percentage.
 * @returns - The transaction result.
 */
export async function swapSolanaTokens(
  userPublicKey: string,
  toToken: SolToken,
  fromToken: SolToken,
  fromAmount: number,
  slippage: number
) {
  const slippageBps = slippage * 100;
  let integerAmount;
  if (fromToken.symbol === "SOL") {
    integerAmount = fromAmount * 1e9;
  } else {
    integerAmount = fromAmount * 10 ** 6;
  }

  let urlQuote = `https://public.jupiterapi.com/quote?inputMint=${fromToken.address}&outputMint=${toToken.address}&amount=${integerAmount}&slippageBps=${slippageBps}`;

  const quoteResponse = await axios.get(urlQuote);

  const urlSwap = `https://public.jupiterapi.com/swap`;
  const payload = {
    userPublicKey: userPublicKey,
    quoteResponse: quoteResponse.data,
  };
  const swapResponse = await axios.post(urlSwap, payload, {
    headers: {
      "Content-Type": "application/json",
    },
  });
  let tx = swapResponse.data.swapTransaction;
  if (!tx || typeof tx !== "string") {
    return;
  }
  // Verify that tx is a valid base64 string
  if (!/^[A-Za-z0-9+/]*={0,2}$/.test(tx)) {
    throw new Error("Swap transaction is not a valid base64 string");
  }
  let result = await signAndSendSolTransaction(tx);
  return result;
}

/**
 * Get the price of SOL.
 * @returns {Promise<number>} The price of SOL.
 */
export async function getSOLPrice(): Promise<number> {
  const response = await fetch(
    "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd"
  );
  const data = await response.json();
  return data.solana.usd;
}

export async function getAllSolanaTokensBalance(address: string) {
  const connection = new Connection(import.meta.env.VITE_RPC_URL);
  const publicKey = new PublicKey(address);
  const tokens = await connection.getParsedTokenAccountsByOwner(publicKey, {
    programId: new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
  });
  return tokens;
}

export async function signAndSendSolTransaction(swapTransaction: string) {
  try {
    const user = await TelegramApi.getItem(
      `user_${WebApp.initDataUnsafe.user?.id}`
    );
    const json_user = JSON.parse(user);
    const turnkey = new Turnkey({
      apiBaseUrl: "https://api.turnkey.com",
      apiPrivateKey: json_user.privateKey,
      apiPublicKey: json_user.publicKey,
      defaultOrganizationId: json_user.subOrgId,
    });
    const turnkeyClient = turnkey.apiClient();
    const turnkeySigner = new SolanaTurnkeySigner({
      organizationId: json_user.subOrgId,
      client: turnkeyClient,
    });

    let connection = new Connection(import.meta.env.VITE_RPC_URL);

    const transactionBuffer = Buffer.from(swapTransaction, "base64");

    let transaction = VersionedTransaction.deserialize(transactionBuffer);
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
        return {
          success: true,
          signature: signature,
          blockHash: latestBlockHash.blockhash,
          error: "",
        };
      } catch (error) {
        retries++;
        if (retries < MAX_RETRIES) {
          log(
            `Transaction failed. Retrying in ${RETRY_DELAY / 1000} seconds...`,
            "info"
          );
          await new Promise((resolve) => setTimeout(resolve, RETRY_DELAY));
        } else {
          log(`Transaction failed after ${MAX_RETRIES} attempts`, "error");
          return {
            success: false,
            signature: "",
            blockHash: "",
            error: `Transaction failed after ${MAX_RETRIES} attempts`,
          };
        }
      }
    }
  } catch (error) {
    log(`Error in signAndSendTransaction: ${error}`, "error");
    return {
      success: false,
      signature: "",
      blockHash: "",
      error: `Error in signAndSendTransaction: ${error}`,
    };
  }
}

/**
 * Transfers SOL from one account to another.
 *
 * This function retrieves user data from TelegramApi, initializes a Turnkey client,
 * and creates a new wallet account for the user.
 *
 * @async
 * @function transferSOL
 * @param {string} from - The sender's public key.
 * @param {string} to - The receiver's public key.
 * @param {number} amount - The amount of SOL to transfer.
 * @param {string} user_json_string - The user data from TelegramApi.
 * @returns {Promise<any>} The response from the Turnkey API after creating the wallet account.
 * @throws {Error} If the user data is invalid or cannot be parsed.
 */
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
    const turnkeySigner = new SolanaTurnkeySigner({
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
