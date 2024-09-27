import React, { useState, useEffect } from "react";
import WebApp from "@twa-dev/sdk";
import capsuleClient from "./lib/capsuleClient";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { addOrUpdateUser, addWalletToUser, parseUserData, getUserWallets, WalletData } from "./lib/utils";
import { Button } from "./components/ui/button";
import dotenv from "dotenv";
import { Input } from "./components/ui/input";
import { Spinner } from "./components/ui/spinner";
import type { TurnkeyApiClient } from "@turnkey/sdk-server";
import { Turnkey, TurnkeyActivityError } from "@turnkey/sdk-server";
import CopyIcon from "./assets/copy.svg";
import * as crypto from "crypto";
import { getWalletSolBalance } from "./lib/solana";
import path from 'path';
import {
  clearChunkedStorage,
  ErrorHandler,
  LogFunction,
} from "./lib/cloudStorageUtil";




const App: React.FC = () => {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [walletId, setWalletId] = useState<string | null>(null);
  const [userWallets, setUserWallets] = useState<any[]>([]);
  const [userShare, setUserShare] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [signature, setSignature] = useState("");
  const [logs, setLogs] = useState<Array<{ message: string; type: "info" | "error" | "success" }>>([]);
  const [showLogs, setShowLogs] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [loadingText, setLoadingText] = useState("");
  const [isStorageComplete, setIsStorageComplete] = useState(false);
  const [walletName, setWalletName] = useState("");
  const [isWalletInputOpen, setIsWalletInputOpen] = useState(false);
  const [createWalletButtonActive, setCreateWalletButtonActive] = useState(true);

  useEffect(() => {
    initializeApp();
  }, []);


  // Turnkey Setup
  const turnkey = new Turnkey({
    apiBaseUrl: "https://api.turnkey.com",
    apiPublicKey: import.meta.env.VITE_TURNKEY_PUBLIC!,
    apiPrivateKey: import.meta.env.VITE_TURNKEY_PRIVATE!,
    defaultOrganizationId: import.meta.env.VITE_TURNKEY_ORGNIZATION!,
  });
  const turnkeyClient = turnkey.apiClient();


  const initializeApp = async () => {
    setIsLoading(true);
    setLoadingText("Initializing...");

    try {
      WebApp.ready();
    
      
      if (!WebApp.initDataUnsafe.user) {
        throw new Error("No User found. Please open App from Telegram");
      }
      log(`User authenticated: ${WebApp.initDataUnsafe.user.username}`, "success");
      setIsAuthenticated(true);


      // add user to redis or update if already exists
      let addOrUpdateUserReponse = await addOrUpdateUser(parseUserData(WebApp.initDataUnsafe.user));
      log(`${addOrUpdateUserReponse.data}`, "success");

      // get user wallets from redis
      let getUserWalletsResponse = await getUserWallets(WebApp.initDataUnsafe.user?.id.toString() ?? ""); 

      // update user wallets with new wallets
      const updatedWallets = await Promise.all(getUserWalletsResponse.data.map(async (wallet: WalletData) => {
        const {solBalance, usdtBalance} = await getWalletSolBalance(wallet.sol_address);
        return {...wallet, solBalance, usdtBalance};
      }));
      setUserWallets(updatedWallets);

      
    } catch (error) {
      handleError(`Initialization error: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
      setLoadingText("");
    }
  };

  const log: LogFunction = (message, type) => {
    setLogs((prevLogs) => [...prevLogs, { message, type }]);
  };

  const handleError: ErrorHandler = (errorMessage) => log(errorMessage, "error");

  const generateWallet = async (client: TurnkeyApiClient, walletUserName: string): Promise<void> => {
    setIsLoading(true);
    setLoadingText("Generating a new wallet...");
    setCreateWalletButtonActive(false);
    try {
      const username = WebApp.initDataUnsafe.user?.username;
      if (!username) throw new Error("Username not found");

      log(`Generating wallet for user ${username}`, "success");
      
      const walletName = `Solana Wallet ${crypto.randomBytes(2).toString("hex")}`;
      log(`wallet name: ${walletName}`, "success");
      try {
        const response = await client.createWallet({
          walletName,
          accounts: [
            {
              pathFormat: "PATH_FORMAT_BIP32",
              // https://github.com/satoshilabs/slips/blob/master/slip-0044.md
              path: "m/44'/501'/0'/0'",
              curve: "CURVE_ED25519",
              addressFormat: "ADDRESS_FORMAT_SOLANA",
            },
          ],
        });

      log(`generated wallet: ${walletName}`, "success");
      const walletId = response.walletId;
      if (!walletId) {
        throw new Error("Response doesn't contain wallet ID");
      }

      const address = response.addresses[0];
      if (!address) {
        throw new Error("Response doesn't contain wallet address");
      }
      
      let walletAddResponse = await addWalletToUser(WebApp.initDataUnsafe.user?.id.toString() ?? "", walletId, walletName, walletUserName, address);

      let newWallet: WalletData = {
        user_id: WebApp.initDataUnsafe.user?.id.toString() ?? "",
        wallet_id: walletId,
        turnkey_wallet_name: walletName,
        user_wallet_name: walletUserName,
        sol_address: address
      };

      // Add new wallet to existing wallets
      const updatedWallets = [...userWallets, newWallet];


      // Update balances for all wallets
      const walletsWithBalances = await Promise.all(updatedWallets.map(async (wallet) => {
        const { solBalance, usdtBalance } = await getWalletSolBalance(wallet.sol_address);
        return { ...wallet, solBalance, usdtBalance };
      }));

      // Update state with new wallet and updated balances
      setUserWallets(walletsWithBalances);
      

      log(`wallet added to user: ${walletName}`, "success");

      } catch (error) {
        // If needed, you can read from `TurnkeyActivityError` to find out why the activity didn't succeed
        if (error instanceof TurnkeyActivityError) {
          throw error;
        }

        throw new TurnkeyActivityError({
          message: `Failed to create a new Solana wallet: ${
          (error as Error).message
          }`,
          cause: error as Error,
        });
      }
    } catch (error) {
      handleError(`Error generating wallet: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
      setLoadingText("");
    }
  };
  const signMessage = async () => {
    if (!walletId || !userShare) {
      handleError("Wallet ID or User Share not available to sign message");
      return;
    }

    setIsLoading(true);
    setLoadingText(`Signing message "${message}"...`);
    try {
      await capsuleClient.setUserShare(userShare);
      const messageBase64 = btoa(message);
      const sig = await capsuleClient.signMessage(walletId, messageBase64);

      if ("transactionReviewUrl" in sig) {
        throw new Error(`Error: Transaction review required: ${sig.transactionReviewUrl}`);
      }
      setSignature(sig.signature);
      log(`Message signed successfully`, "success");
    } catch (error) {
      handleError(`Error signing message: ${error}`);
    } finally {
      setIsLoading(false);
      setLoadingText("");
    }
  };

  const clearStorage = async () => {
    setIsLoading(true);
    setLoadingText("Clearing storage and resetting state...");
    try {
      await clearChunkedStorage(log, handleError);
      setUserShare(null);
      setWalletId(null);
      setIsStorageComplete(false);
      log("Finished clearing storage and resetting state", "success");
    } catch (error) {
      handleError(`Error clearing storage: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
      setLoadingText("");
    }
  };

  const logout = () => {
    log("Logging out...", "info");
    WebApp.close();
  };

  return (
    <div className="container mx-auto p-4">
     
      <Card className="mb-4">
        <CardHeader>
          <CardTitle>{isAuthenticated ? "Wallet Manager" : "Capsule TG App Example"}</CardTitle>
        </CardHeader>
        <CardContent className="overflow-hidden">
          {!isAuthenticated ? (
            <p>Authenticating...</p>
          ) : !walletId && (
            <div className="flex flex-col justify-between">
              {createWalletButtonActive ? (
                <Button
                  onClick={() => {setIsWalletInputOpen(true); setCreateWalletButtonActive(false)}}
                disabled={isLoading}>
                {isLoading ? <Spinner /> : "Create New Wallet"}
              </Button>
              ) : (
                <div>
                {!isLoading && 
                <Button className="mb-2"
                  onClick={() => {setIsWalletInputOpen(false); setCreateWalletButtonActive(true)}}
                  disabled={isLoading}>
                {"Back"}
                </Button>
                }
                <Input
                  value={walletName}
                  onChange={(e) => setWalletName(e.target.value)}
                  placeholder="Enter Wallet Name"
                  className="mb-2 bg-card"
                />
                <Button
                  onClick={() => generateWallet(turnkeyClient, walletName)}
                  className="mb-2"
                  disabled={isLoading || !walletName}>
                  {isLoading ? <Spinner /> : "Generate Wallet"}
                </Button>
              </div>
              )}
                <h3 className="text-lg font-semibold text-primary mb-2">Wallets</h3>
              {userWallets.map((wallet) => (
                <div className="flex flex-col justify-between">
                  <div className="flex flex-row items-center">
                  <p className="text-sm mr-2">{wallet.user_wallet_name}</p>
                  <div className="flex flex-row items-center">
                    <p className="text-sm">
                      {`${wallet.sol_address.slice(0, 3)}...${wallet.sol_address.slice(-3)}`}
                    </p>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="ml-2"
                      onClick={() => navigator.clipboard.writeText(wallet.sol_address)}
                    >
                      <img src={CopyIcon} alt="Copy" className="h-4 w-4" />
                    </Button>
                    <div className="flex flex-col items-start ml-2">
                      <span className="text-sm font-semibold text-primary">
                        {wallet.solBalance !== undefined ? `${wallet.solBalance.toFixed(4)} SOL` : 'Loading...'}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {wallet.usdtBalance !== undefined ? `$${wallet.usdtBalance.toFixed(2)} USD` : 'Loading...'}
                      </span>
                    </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
      <Card className="mb-4">
        <CardHeader className="flex justify-between flex-row">
          <CardTitle>Copy Trade</CardTitle>
        </CardHeader>
        <CardContent>
          <Input
            placeholder="Enter wallet address"
            className="mb-2 bg-card"
          />
          <Input
            placeholder="Buy amount"
            className="mb-2 bg-card"
          />
          <Button
            disabled={isLoading}
          >
            {isLoading ? <Spinner /> : "Copy Trade"}
          </Button>
        </CardContent>
      </Card>

      <Card className="mb-4">
        <CardHeader className="flex justify-between flex-row">
          <CardTitle>App Logs</CardTitle>
          <Button
            size={"sm"}
            variant={"outline"}
            onClick={() => setShowLogs(!showLogs)}>
            {showLogs ? 'Hide' : 'Show'}
          </Button>
          <Button
            size={"sm"}
            disabled={logs.length === 0}
            variant={"outline"}
            onClick={() => setLogs([])}>
            Clear
          </Button>
        </CardHeader>
        <CardContent className="overflow-auto max-h-60">
          <p>{userShare ? (isStorageComplete ? `Wallet Stored: ✅` : `Wallet Stored: In Progress`) : ``}</p>
          <p>{userShare ? (isLoading ? `Wallet Fetched: In Progress` : `Wallet Fetched: ✅`) : ``}</p>
          <div className="font-mono text-[12px]">
            {!!showLogs && (
              logs.length === 0 ? (
                <p>No logs yet.</p>
              ) : (
                logs.map((log, index) => (
                  <p
                    key={index}
                    className={`${log.type === "error" ? "text-red-500" : log.type === "success" ? "text-green-500" : ""}`}>
                    {log.message}
                  </p>
                ))
              ))
            }
          </div>
          </CardContent>
      </Card>
    </div>
  );
};

export default App;
