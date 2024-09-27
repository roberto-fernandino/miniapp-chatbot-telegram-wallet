import React, { useState, useEffect } from "react";
import axios from "axios";
import WebApp from "@twa-dev/sdk";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import {
  addOrUpdateUser,
  addWalletToUser,
  parseUserData,
  getUserWallets,
  WalletData,
  setCopyTradeWallet,
  getCopyTrades,
} from "./lib/utils";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Spinner } from "./components/ui/spinner";
import type { TurnkeyApiClient } from "@turnkey/sdk-server";
import { Turnkey, TurnkeyActivityError } from "@turnkey/sdk-server";
import CopyIcon from "./assets/copy.svg";
import * as crypto from "crypto";
import { getWalletSolBalance } from "./lib/solana";
import { ErrorHandler, LogFunction } from "./lib/cloudStorageUtil";

const App: React.FC = () => {
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  // User Wallets array
  const [userWallets, setUserWallets] = useState<any[]>([]);

  // Logs
  const [logs, setLogs] = useState<
    Array<{ message: string; type: "info" | "error" | "success" }>
  >([]);
  const [showLogs, setShowLogs] = useState(false);

  // loading state
  const [isLoading, setIsLoading] = useState(false);

  // Currently selected wallet to buy with (Copy Trade)
  const [walletToBuyWithId, setWalletToBuyWithId] = useState<string>("");
  const [walletToBuyWithName, setWalletToBuyWithName] = useState<string>("");
  // Wallet to copy trade
  const [walletToCopyTrade, setWalletToCopyTrade] = useState<string>("");
  // Amount to buy copy trade
  const [amountToBuyCopyTrade, setAmountToBuyCopyTrade] = useState<string>("");
  // Copy trades set by user
  const [copyTrades, setCopyTrades] = useState<any[]>([]);

  const [walletName, setWalletName] = useState("");
  const [createWalletButtonActive, setCreateWalletButtonActive] =
    useState(true);

  useEffect(() => {
    initializeApp();
  }, []);

  useEffect(() => {
    if (userWallets.length > 0 && !walletToBuyWithId) {
      setWalletToBuyWithId(userWallets[0].wallet_id);
      setWalletToBuyWithName(userWallets[0].user_wallet_name);
    }
  }, [userWallets]);

  // Turnkey Setup
  const turnkey = new Turnkey({
    apiBaseUrl: "https://api.turnkey.com",
    apiPublicKey: import.meta.env.VITE_TURNKEY_PUBLIC!,
    apiPrivateKey: import.meta.env.VITE_TURNKEY_PRIVATE!,
    defaultOrganizationId: import.meta.env.VITE_TURNKEY_ORGNIZATION!,
  });
  const turnkeyClient = turnkey.apiClient();

  async function updateCopyTrades() {
    const getCopyTradesResponse = await getCopyTrades(
      WebApp.initDataUnsafe.user?.id.toString() ?? ""
    );
    const updatedCopyTrades = await Promise.all(
      getCopyTradesResponse.data.map(async (copyTrade: any) => {
        return { ...copyTrade };
      })
    );
    setCopyTrades(updatedCopyTrades);
  }
  async function updateUserWallets() {
    // get user wallets from redis
    let getUserWalletsResponse = await getUserWallets(
      WebApp.initDataUnsafe.user?.id.toString() ?? ""
    );
    const updatedWallets = await Promise.all(
      getUserWalletsResponse.data.map(async (wallet: WalletData) => {
        const { solBalance, usdtBalance } = await getWalletSolBalance(
          wallet.sol_address
        );
        return { ...wallet, solBalance, usdtBalance };
      })
    );
    setUserWallets(updatedWallets);
  }
  const initializeApp = async () => {
    setIsLoading(true);

    try {
      WebApp.ready();

      if (!WebApp.initDataUnsafe.user) {
        throw new Error("No User found. Please open App from Telegram");
      }
      log(
        `User authenticated: ${WebApp.initDataUnsafe.user.username}`,
        "success"
      );
      setIsAuthenticated(true);

      // add user to redis or update if already exists
      let addOrUpdateUserReponse = await addOrUpdateUser(
        parseUserData(WebApp.initDataUnsafe.user)
      );
      log(`${addOrUpdateUserReponse.data}`, "success");

      updateCopyTrades();
      updateUserWallets();
    } catch (error) {
      handleError(
        `Initialization error: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    } finally {
      setIsLoading(false);
    }
  };

  const log: LogFunction = (message, type) => {
    setLogs((prevLogs) => [...prevLogs, { message, type }]);
  };

  const handleError: ErrorHandler = (errorMessage) =>
    log(errorMessage, "error");

  const generateWallet = async (
    client: TurnkeyApiClient,
    walletUserName: string
  ): Promise<void> => {
    setIsLoading(true);
    setCreateWalletButtonActive(false);
    try {
      const username = WebApp.initDataUnsafe.user?.username;
      if (!username) throw new Error("Username not found");

      log(`Generating wallet for user ${username}`, "success");

      const walletName = `Solana Wallet ${crypto
        .randomBytes(2)
        .toString("hex")}`;
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

        await addWalletToUser(
          WebApp.initDataUnsafe.user?.id.toString() ?? "",
          walletId,
          walletName,
          walletUserName,
          address
        );

        let newWallet: WalletData = {
          user_id: WebApp.initDataUnsafe.user?.id.toString() ?? "",
          wallet_id: walletId,
          turnkey_wallet_name: walletName,
          user_wallet_name: walletUserName,
          sol_address: address,
        };

        // Add new wallet to existing wallets
        const updatedWallets = [...userWallets, newWallet];

        // Update balances for all wallets
        const walletsWithBalances = await Promise.all(
          updatedWallets.map(async (wallet) => {
            const { solBalance, usdtBalance } = await getWalletSolBalance(
              wallet.sol_address
            );
            return { ...wallet, solBalance, usdtBalance };
          })
        );

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
      handleError(
        `Error generating wallet: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    } finally {
      setIsLoading(false);
    }
  };

  const logout = () => {
    log("Logging out...", "info");
    WebApp.close();
  };

  const handleSetCopyTrade = async (
    user_id: string,
    wallet_id: string,
    buy_amount: string,
    copy_trade_address: string,
    status: string,
    user_wallet_name: string
  ) => {
    try {
      await setCopyTradeWallet(
        user_id,
        wallet_id,
        buy_amount,
        copy_trade_address,
        status,
        user_wallet_name
      );

      // Update the local state separately
      setCopyTrades((prevCopyTrades) =>
        prevCopyTrades.map((ct) =>
          ct.copy_trade_address === copy_trade_address
            ? { ...ct, wallet_id, buy_amount, status, user_wallet_name }
            : ct
        )
      );
      updateCopyTrades();
      updateUserWallets();

      log("Copy trade updated successfully", "success");
    } catch (error) {
      log("Failed to update copy trade", "error");
    }
  };

  return (
    <div className="container mx-auto p-4">
      <Card className="mb-4">
        <CardHeader>
          <CardTitle>
            {isAuthenticated ? "Wallet Manager" : "Capsule TG App Example"}
          </CardTitle>
        </CardHeader>
        <CardContent className="overflow-hidden">
          {!isAuthenticated ? (
            <p>Authenticating...</p>
          ) : (
            <div className="flex flex-col justify-between">
              {createWalletButtonActive ? (
                <Button
                  onClick={() => {
                    setCreateWalletButtonActive(false);
                  }}
                  disabled={isLoading}
                >
                  {isLoading ? <Spinner /> : "Create New Wallet"}
                </Button>
              ) : (
                <div>
                  {!isLoading && (
                    <Button
                      className="mb-2"
                      onClick={() => {
                        setCreateWalletButtonActive(true);
                      }}
                      disabled={isLoading}
                    >
                      {"Back"}
                    </Button>
                  )}
                  <Input
                    value={walletName}
                    onChange={(e) => setWalletName(e.target.value)}
                    placeholder="Enter Wallet Name"
                    className="mb-2 bg-card"
                  />
                  <Button
                    onClick={() => generateWallet(turnkeyClient, walletName)}
                    className="mb-2"
                    disabled={isLoading || !walletName}
                  >
                    {isLoading ? <Spinner /> : "Generate Wallet"}
                  </Button>
                </div>
              )}
              <h3 className="text-lg font-semibold text-primary mb-2">
                Wallets
              </h3>
              {userWallets.map((wallet) => (
                <div className="flex flex-col justify-between">
                  <div className="flex flex-row items-center">
                    <p className="text-sm mr-2">{wallet.user_wallet_name}</p>
                    <div className="flex flex-row items-center">
                      <p className="text-sm">
                        {`${wallet.sol_address.slice(
                          0,
                          3
                        )}...${wallet.sol_address.slice(-3)}`}
                      </p>
                      <Button
                        size="sm"
                        variant="ghost"
                        className="ml-2"
                        onClick={() =>
                          navigator.clipboard.writeText(wallet.sol_address)
                        }
                      >
                        <img src={CopyIcon} alt="Copy" className="h-4 w-4" />
                      </Button>
                      <div className="flex flex-col items-start ml-2">
                        <span className="text-sm font-semibold text-primary">
                          {wallet.solBalance !== undefined
                            ? `${wallet.solBalance.toFixed(4)} SOL`
                            : "Loading..."}
                        </span>
                        <span className="text-xs text-muted-foreground">
                          {wallet.usdtBalance !== undefined
                            ? `$${wallet.usdtBalance.toFixed(2)} USD`
                            : "Loading..."}
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
          <div className="flex flex-col items-center">
            <span>Wallet to buy with</span>
            <select
              className="mb-2  w-full  bg-white border-2 border-gray-300 rounded-md p-1"
              value={walletToBuyWithId ?? ""}
              onChange={(e) => {
                setWalletToBuyWithId(e.target.value);
                setWalletToBuyWithName(
                  e.target.options[e.target.selectedIndex].text
                );
              }}
            >
              {userWallets.map((wallet) => (
                <option
                  key={wallet.wallet_id}
                  value={wallet.wallet_id}
                  className="text-center"
                >
                  {wallet.user_wallet_name}
                </option>
              ))}
            </select>
          </div>
          <Input
            placeholder="Enter wallet address"
            className="mb-2 bg-card"
            value={walletToCopyTrade}
            onChange={(e) => {
              setWalletToCopyTrade(e.target.value);
            }}
          />
          <Input
            placeholder="Buy amount"
            className="mb-2 bg-card"
            value={amountToBuyCopyTrade}
            onChange={(e) => {
              setAmountToBuyCopyTrade(e.target.value);
            }}
          />
          <Button
            disabled={isLoading}
            onClick={() =>
              handleSetCopyTrade(
                WebApp.initDataUnsafe.user?.id.toString() ?? "",
                walletToBuyWithId,
                amountToBuyCopyTrade,
                walletToCopyTrade,
                "active",
                walletToBuyWithName
              )
            }
          >
            {isLoading ? <Spinner /> : "Copy Trade"}
          </Button>
          <div className="flex flex-col items-center w-full mt-3">
            <span>Copy trades</span>
            <div>
              {copyTrades.map((copyTrade) => (
                <div
                  key={copyTrade.wallet_id}
                  className="flex flex-row items-center justify-between p-4 bg-gray-100 rounded-lg mb-2 w-full"
                >
                  <div className="flex flex-col w-full">
                    <p className="font-semibold text-lg">
                      {copyTrade.user_wallet_name}
                    </p>
                    <p className="text-sm text-gray-600">
                      {copyTrade.copy_trade_address.slice(0, 3)}...
                      {copyTrade.copy_trade_address.slice(-3)}
                    </p>
                  </div>
                  <div className="flex items-center">
                    <p className="mr-4 font-medium">
                      {copyTrade.buy_amount} SOL
                    </p>
                    <Button
                      onClick={() =>
                        handleSetCopyTrade(
                          WebApp.initDataUnsafe.user?.id.toString() ?? "",
                          copyTrade.wallet_id,
                          copyTrade.buy_amount,
                          copyTrade.copy_trade_address,
                          copyTrade.status === "active" ? "inactive" : "active",
                          copyTrade.user_wallet_name
                        )
                      }
                      className={`px-4 py-2 rounded-full ${
                        copyTrade.status === "active"
                          ? "bg-red-500 hover:bg-red-600 text-white"
                          : "bg-green-500 hover:bg-green-600 text-white"
                      }`}
                    >
                      {copyTrade.status === "active" ? "Cancel" : "Activate"}
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </CardContent>
      </Card>

      <Card className="mb-4">
        <CardHeader className="flex justify-between flex-row">
          <CardTitle>App Logs</CardTitle>
          <Button
            size={"sm"}
            variant={"outline"}
            onClick={() => setShowLogs(!showLogs)}
          >
            {showLogs ? "Hide" : "Show"}
          </Button>
          <Button
            size={"sm"}
            disabled={logs.length === 0}
            variant={"outline"}
            onClick={() => setLogs([])}
          >
            Clear
          </Button>
        </CardHeader>
        <CardContent className="overflow-auto max-h-60">
          <div className="font-mono text-[12px]">
            {!!showLogs &&
              (logs.length === 0 ? (
                <p>No logs yet.</p>
              ) : (
                logs.map((log, index) => (
                  <p
                    key={index}
                    className={`${
                      log.type === "error"
                        ? "text-red-500"
                        : log.type === "success"
                        ? "text-green-500"
                        : ""
                    }`}
                  >
                    {log.message}
                  </p>
                ))
              ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
};

export default App;
