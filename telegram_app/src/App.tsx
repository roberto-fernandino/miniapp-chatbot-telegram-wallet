import React, { useState, useEffect } from "react";
import WebApp from "@twa-dev/sdk";
import capsuleClient from "./lib/capsuleClient";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { addOrUpdateUser, addWalletToUser, parseUserData, getUserWallets } from "./lib/utils";
import { Button } from "./components/ui/button";
import dotenv from "dotenv";
import { Input } from "./components/ui/input";
import { Spinner } from "./components/ui/spinner";
import type { TurnkeyApiClient } from "@turnkey/sdk-server";
import { Turnkey, TurnkeyActivityError } from "@turnkey/sdk-server";
import * as crypto from "crypto";
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

      let getUserWalletsResponse = await getUserWallets(WebApp.initDataUnsafe.user?.id.toString() ?? "");
      setUserWallets(getUserWalletsResponse.data);
    


      
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
      log(`wallet added to user: ${walletAddResponse.data}`, "success");

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
          ) : !walletId ? (
            <div className="flex justify-between">
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
            </div>
          ) : (
            <>
              <p className="text-[12px]">{`Wallet Address: ${address}`}</p>
              <Input
                value={message}
                onChange={(e) => setMessage(e.target.value)}
                placeholder="Message to sign"
                className="mb-2 bg-card"
              />
              <Button
                variant={"outline"}
                onClick={signMessage}
                className="mb-2"
                disabled={isLoading || !message}>
                {isLoading ? <Spinner /> : "Sign Message"}
              </Button>
              {signature && <p className="mb-2 break-all">Signature: {signature}</p>}
              <div>
                <Button
                  onClick={clearStorage}
                  className="ml-2"
                  disabled={isLoading}>
                  Clear Storage
                </Button>
              </div>
            </>
          )}
          {loadingText && <p className="mt-2">{loadingText}</p>}
        </CardContent>
      </Card>

      <Card>
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
