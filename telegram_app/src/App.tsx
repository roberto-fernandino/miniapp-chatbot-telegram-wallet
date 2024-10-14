import React, { useState, useEffect } from "react";
import SolanaIcon from "./assets/sol.png";
import SwapSheet from "./components/ui/SwapSheet";
import {
  deleteCopyTradeWallet,
  generateKeyPair,
  getTokenData,
  setUserSession,
  signAndSendTransaction,
} from "./lib/utils";
import WebApp from "@twa-dev/sdk";
import axios from "axios";
import { Card, CardContent, CardHeader, CardTitle } from "./components/ui/card";
import { DEFAULT_ETHEREUM_ACCOUNTS, Turnkey } from "@turnkey/sdk-server";
import {
  setCopyTradeWallet,
  getCopyTrades,
  decryptPassword,
  encryptPassword,
  getSOLPrice,
  getBalance,
  transferSOL,
  copyTrade,
} from "./lib/utils";
import { Button } from "./components/ui/button";
import { Input } from "./components/ui/input";
import { Spinner } from "./components/ui/spinner";
import CopyIcon from "./assets/copy.svg";
import { ErrorHandler, LogFunction } from "./lib/cloudStorageUtil";
import { TelegramApi } from "./telegram/telegram-api";
import TokensBalances from "./components/ui/tokenBalances";
import TokensBalancesSwap from "./components/ui/tokensBalancesSwap";
import SwapInterface from "./components/ui/swap";

const App: React.FC = () => {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [isRegistered, setIsRegistered] = useState(false);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [userAccounts, setUserAccounts] = useState<any[]>([]);
  const [walletId, setWalletId] = useState<string>("");
  const [solAddress, setSolAddress] = useState<string>("");

  // Session information
  const [sessionActive, setSessionActive] = useState<boolean>(false);
  const [sessionDuration, setSessionDuration] = useState<string>("");

  // Logs
  const [logs, setLogs] = useState<
    Array<{ message: string; type: "info" | "error" | "success" }>
  >([]);
  const [showLogs, setShowLogs] = useState(false);

  // loading state
  const [isLoading, setIsLoading] = useState(false);

  // Account/Address to copy trade
  const [walletToCopyTrade, setWalletToCopyTrade] = useState<string>("");

  // SOL Amount to buy copy trade
  const [amountToBuyCopyTrade, setAmountToBuyCopyTrade] = useState<string>("");

  // Copy trades set by user
  const [copyTrades, setCopyTrades] = useState<any[]>([]);
  const [createSessionButtonActive, setCreateSessionButtonActive] =
    useState(true);

  // Balance information
  const [solBalance, setSolBalance] = useState<string>("0");
  const [usdSolBalance, setUsdSolBalance] = useState<string>("0.00");

  // Session
  const [remainingTime, setRemainingTime] = useState<string>("");

  // Tranfer
  const [transferSolActive, setTransferSolActive] = useState(false);
  const [amount, setAmount] = useState("");
  const [recipient, setRecipient] = useState("");

  // WebSocket
  const [socket, setSocket] = useState<WebSocket | null>(null);

  // Swap
  const [swapSheetOpen, setSwapSheetOpen] = useState(false);
  const [tokenCa, setTokenCa] = useState("");
  const [tokenData, setTokenData] = useState<any>(null);

  useEffect(() => {
    initializeApp();
  }, []);

  const turnkey = new Turnkey({
    apiBaseUrl: "https://api.turnkey.com",
    apiPublicKey: import.meta.env.VITE_TURNKEY_PUBLIC!,
    apiPrivateKey: import.meta.env.VITE_TURNKEY_PRIVATE!,
    defaultOrganizationId: import.meta.env.VITE_TURNKEY_ORGNIZATION!,
  });
  const rootTurnkeyClient = turnkey.apiClient();

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

  // USER REGISTRATION
  const handleRegister = async () => {
    try {
      const { publicKey, privateKey } = generateKeyPair();

      const encryptedPassword = encryptPassword(password);

      log("Creating user in Turnkey...", "info");
      const createSubOrgWithUserRespse =
        await rootTurnkeyClient.createSubOrganization({
          subOrganizationName: WebApp.initDataUnsafe.user?.username ?? "",
          organizationId: import.meta.env.VITE_TURNKEY_ORGNIZATION!,
          rootQuorumThreshold: 1,
          disableEmailAuth: true,
          disableEmailRecovery: true,
          wallet: {
            walletName: WebApp.initDataUnsafe.user?.username ?? "",
            accounts: [
              {
                curve: "CURVE_ED25519",
                pathFormat: "PATH_FORMAT_BIP32",
                path: "m/44'/501'/0'/0'",
                addressFormat: "ADDRESS_FORMAT_SOLANA",
              },
              ...DEFAULT_ETHEREUM_ACCOUNTS,
            ],
          },
          rootUsers: [
            {
              userName: WebApp.initDataUnsafe.user?.username ?? "",
              userEmail: email,
              apiKeys: [
                {
                  publicKey: publicKey,
                  apiKeyName: `telegram_app_${WebApp.initDataUnsafe.user?.id}`,
                  curveType: "API_KEY_CURVE_P256",
                },
              ],
              authenticators: [],
              oauthProviders: [],
            },
          ],
        });
      log(
        `Create sub org with user response: ${JSON.stringify(
          createSubOrgWithUserRespse
        )}`,
        "success"
      );
      const walletId =
        (await createSubOrgWithUserRespse)?.wallet?.walletId ?? "";
      const subOrgId =
        (await createSubOrgWithUserRespse)?.subOrganizationId ?? "";
      const userId = (await createSubOrgWithUserRespse)?.rootUserIds?.[0] ?? "";
      const user = {
        email,
        password: encryptedPassword,
        tgUserId: WebApp.initDataUnsafe.user?.id,
        publicKey: publicKey,
        privateKey: privateKey,
        subOrgId: subOrgId,
        walletId: walletId,
        userId: userId,
      };
      log(
        `User object created: ${JSON.stringify({
          user,
        })}`,
        "success"
      );

      await TelegramApi.setItem(
        `user_${WebApp.initDataUnsafe.user?.id}`,
        JSON.stringify(user)
      );
      log("User stored in TelegramApi", "success");

      setIsRegistered(true);
      log("User registered successfully", "success");
    } catch (error) {
      console.error("Full error object:", error);
      if (error instanceof Error) {
        log(`Error during registration: ${error.message}`, "error");
      } else {
        log(`Unknown error during registration: ${String(error)}`, "error");
      }
    }
    handleLogin(password);
    window.location.reload();
  };

  // USER LOGIN
  const handleLogin = async (password: string) => {
    try {
      log("Logging in...", "success");
      const user = await TelegramApi.getItem(
        `user_${WebApp.initDataUnsafe.user?.id}`
      );

      let json_user;
      try {
        json_user = JSON.parse(user);
      } catch (error) {
        return;
      }

      if (!json_user.password) {
        log("Password not found in user data", "error");
        return;
      }

      let decryptedPassword;
      try {
        decryptedPassword = decryptPassword(json_user.password);
      } catch (error) {
        log(`Error decrypting password: ${error}`, "error");
        return;
      }

      if (password === decryptedPassword) {
        setIsAuthenticated(true);
        setIsRegistered(true);
      } else {
        log("Invalid password", "error");
      }
    } catch (error) {
      log(`Unexpected error during login: ${error}`, "error");
    }
  };

  const log: LogFunction = (message, type) => {
    setLogs((prevLogs) => [...prevLogs, { message, type }]);
  };

  const handleError: ErrorHandler = (errorMessage) =>
    log(errorMessage, "error");

  const handleSetCopyTrade = async (
    user_id: string,
    wallet_id: string,
    account_address: string,
    buy_amount: string,
    copy_trade_address: string,
    status: string
  ) => {
    try {
      await setCopyTradeWallet(
        user_id,
        wallet_id,
        account_address,
        buy_amount,
        copy_trade_address,
        status
      );

      // Update the local state separately
      setCopyTrades((prevCopyTrades) =>
        prevCopyTrades.map((ct) =>
          ct.copy_trade_address === copy_trade_address
            ? { ...ct, wallet_id, buy_amount, status, account_address }
            : ct
        )
      );
      updateCopyTrades();

      log("Copy trade updated successfully", "success");
    } catch (error) {
      log(`Failed to update copy trade: ${error}`, "error");
    }
  };

  const handleCreateSession = async (sessionDuration: string) => {
    setIsLoading(true);
    const user = await TelegramApi.getItem(
      `user_${WebApp.initDataUnsafe.user?.id}`
    );
    let json_user;
    try {
      json_user = JSON.parse(user);
    } catch (error) {
      log("User not found in TelegramApi", "error");
      return;
    }
    const turnkey = new Turnkey({
      apiBaseUrl: "https://api.turnkey.com",
      apiPublicKey: json_user.publicKey,
      apiPrivateKey: json_user.privateKey,
      defaultOrganizationId: json_user.subOrgId,
    });
    const turnkeyClient = turnkey.apiClient();
    const { publicKey, privateKey } = generateKeyPair();
    try {
      const response = await turnkeyClient.createApiKeys({
        apiKeys: [
          {
            apiKeyName: `telegram_app_session_${WebApp.initDataUnsafe.user?.id}`,
            publicKey: publicKey,
            curveType: "API_KEY_CURVE_P256",
            expirationSeconds: (parseInt(sessionDuration) * 60).toString(),
          },
        ],
        userId: json_user.userId,
        organizationId: json_user.subOrgId,
      });

      json_user.sessionApiKeys = {
        expirationDate: new Date(
          Date.now() + parseInt(sessionDuration) * 60 * 1000
        ).toISOString(),
        publicKey: publicKey,
        privateKey: privateKey,
      };

      await TelegramApi.setItem(
        `user_${WebApp.initDataUnsafe.user?.id}`,
        JSON.stringify(json_user)
      );

      setCreateSessionButtonActive(false);
      setSessionActive(true);
      await setUserSession(json_user.tgUserId);
    } catch (error) {
      log(`Failed to create session: ${error}`, "error");
    } finally {
      setIsLoading(false);
      setCreateSessionButtonActive(true); // Go back to main menu
    }
  };

  const checkSessionApiKeys = async () => {
    const user = await TelegramApi.getItem(
      `user_${WebApp.initDataUnsafe.user?.id}`
    );
    let json_user;
    try {
      json_user = JSON.parse(user);
      if (json_user.sessionApiKeys && json_user.sessionApiKeys !== "") {
        const expirationDate = new Date(
          json_user.sessionApiKeys.expirationDate
        );
        const now = new Date();
        const timeLeft = expirationDate.getTime() - now.getTime();

        if (timeLeft > 0) {
          setSessionActive(true);
          const minutes = Math.floor(timeLeft / 60000);
          const seconds = Math.floor((timeLeft % 60000) / 1000);
          setRemainingTime(`${minutes}m ${seconds}s`);
        } else {
          // Session has expired
          await handleSessionExpiration(json_user);
        }
      } else {
        setSessionActive(false);
        setRemainingTime("");
      }
    } catch (error) {
      log("User not found in TelegramApi", "error");
      return;
    }
  };

  const handleSessionExpiration = async (json_user: any) => {
    setSessionActive(false);
    setRemainingTime("");
    setCreateSessionButtonActive(true);

    json_user.sessionApiKeys = "";
    await TelegramApi.setItem(
      `user_${WebApp.initDataUnsafe.user?.id}`,
      JSON.stringify(json_user)
    );
    await setUserSession(json_user.tgUserId);
  };

  useEffect(() => {
    const intervalId = setInterval(checkSessionApiKeys, 1000); // Run every second

    return () => clearInterval(intervalId); // Cleanup interval on component unmount
  }, []);

  const handleGetTokenData = async (tokenCa: string) => {
    const data = await getTokenData(tokenCa);
    setTokenData(data);
  };

  const handleDeleteCopyTrade = async (copy_trade_address: string) => {
    await deleteCopyTradeWallet(
      WebApp.initDataUnsafe.user?.id.toString() ?? "",
      copy_trade_address
    );
    updateCopyTrades();
  };

  const initializeApp = async () => {
    // Initialize Telegram API
    TelegramApi.removeItems([`user_${WebApp.initDataUnsafe.user?.id}`]);
    TelegramApi.init();
    WebApp.ready();
    setIsLoading(true);
    checkSessionApiKeys();
    const user = await TelegramApi.getItem(
      `user_${WebApp.initDataUnsafe.user?.id}`
    );
    let json_user;
    try {
      json_user = JSON.parse(user);
    } catch (error) {
      log("User not signed up", "success");
      return;
    }
    try {
      const tgUser = WebApp.initDataUnsafe.user;
      if (!tgUser) {
        throw new Error("No User found. Please open App from Telegram");
      }
      const turnkey = new Turnkey({
        apiBaseUrl: "https://api.turnkey.com",
        apiPublicKey: json_user?.publicKey ?? "",
        apiPrivateKey: json_user?.privateKey ?? "",
        defaultOrganizationId: json_user?.subOrgId ?? "",
      });

      const turnkeyClient = turnkey.apiClient();

      const accounts = await turnkeyClient.getWalletAccounts({
        walletId: json_user.walletId,
      });
      json_user.accounts = accounts.accounts;
      await TelegramApi.setItem(
        `user_${WebApp.initDataUnsafe.user?.id}`,
        JSON.stringify(json_user)
      );
      setUserAccounts(accounts.accounts);
      setWalletId(json_user.walletId);

      if (user && user !== "") {
        setIsRegistered(true);
        setIsAuthenticated(true);
      } else {
        setIsAuthenticated(false);
        setIsRegistered(false);
      }

      updateCopyTrades();
    } catch (error) {
      handleError(
        `Initialization error: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    } finally {
      setIsLoading(false);
      // Access the query string
      const urlParams = new URLSearchParams(window.location.search);

      // Get the tgWebAppStartParam
      const startParam = urlParams.get("tgWebAppStartParam");

      if (startParam) {
        // Decode the startParam
        const decodedParams = decodeURIComponent(startParam);

        // Manually parse the parameters
        const paramsArray = decodedParams.split("&");
        let params: { [key: string]: string } = {};
        paramsArray.forEach((param) => {
          const [key, value] = param.split("=");
          params[key] = value;
        });
        const tokenCA = params["tokenCA"];
        // Extract the openSwap and tokenCA parameters

        if (tokenCA) {
          setSwapSheetOpen(true);
          setTokenCa(tokenCA);
          handleGetTokenData(tokenCA);
        }
      }
    }
  };

  const updateBalance = async () => {
    if (userAccounts.length > 0) {
      try {
        const solanaAddress = userAccounts.find(
          (account) => account.addressFormat === "ADDRESS_FORMAT_SOLANA"
        )?.address;
        if (solanaAddress) {
          const balance = await getBalance(solanaAddress);
          setSolBalance(balance);

          const solPrice = await getSOLPrice();
          const usdValue = (parseFloat(balance) * solPrice).toFixed(2);

          setUsdSolBalance(usdValue);
        }
      } catch (error) {
        log(`Failed to update balance: ${error}`, "error");
      }
    }
  };

  interface Token {
    symbol: string;
    name: string;
    logoURI: string;
    balance: string;
    address: string;
  }

  const swapTokens = async (
    userPublicKey: string,
    toToken: Token,
    fromToken: Token,
    fromAmount: number,
    slippage: number
  ) => {
    const slippageBps = slippage * 100;
    log(`Slippage: ${slippageBps}`, "info");
    log(`From Token: ${fromToken.address}`, "info");
    log(`To Token: ${JSON.stringify(toToken.address)}`, "info");
    log(`From Amount: ${fromAmount}`, "info");

    // convert fromAmount to integer
    let integerAmount;
    if (fromToken.symbol === "SOL") {
      integerAmount = fromAmount * 1e9;
    } else {
      integerAmount = fromAmount * 10 ** 6;
    }

    // get quote
    let urlQuote = `https://public.jupiterapi.com/quote?inputMint=${fromToken.address}&outputMint=${toToken.address}&amount=${integerAmount}&slippageBps=${slippageBps}`;

    const quoteResponse = await axios.get(urlQuote);

    // get swap
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
    log("Swap built", "success");
    let tx = swapResponse.data.swapTransaction;
    if (!tx || typeof tx !== "string") {
      return;
    }
    // Verify that tx is a valid base64 string
    if (!/^[A-Za-z0-9+/]*={0,2}$/.test(tx)) {
      throw new Error("Swap transaction is not a valid base64 string");
    }
    log("Swap transaction is valid", "success");
    log("Signing and sending transaction", "info");
    let result = await signAndSendTransaction(tx);
    log(`Transaction result: ${JSON.stringify(result)}`, "success");
    return result;
  };

  const handleEndSession = async () => {
    setSessionActive(false);
    setRemainingTime("");
    let user = await TelegramApi.getItem(
      `user_${WebApp.initDataUnsafe.user?.id}`
    );
    let json_user = JSON.parse(user);
    json_user.sessionApiKeys = "";
    await TelegramApi.setItem(
      `user_${WebApp.initDataUnsafe.user?.id}`,
      JSON.stringify(json_user)
    );
    await setUserSession(json_user.tgUserId);
  };

  const handleTransferSol = async (amount: string, recipient: string) => {
    setIsLoading(true);
    try {
      let user = await TelegramApi.getItem(
        `user_${WebApp.initDataUnsafe.user?.id}`
      );
      let json_user = JSON.parse(user);
      const { success, signature, confirmation, transferredAmount } =
        await transferSOL(
          json_user.accounts[0].address,
          recipient,
          parseFloat(amount),
          user
        );
      if (success) {
        log(`Transferred ${amount} SOL to ${recipient}`, "success");
        log(`check tx: https://solscan.io/tx/${signature}`, "success");
      }
      if (!success) {
        log(`Failed to transfer SOL: ${confirmation}`, "error");
      }
    } catch (error) {
      log(`Failed to transfer SOL: ${error}`, "error");
    } finally {
      setIsLoading(false);
      setTransferSolActive(false);
      updateBalance();
      setAmount("");
      setRecipient("");
    }
  };

  useEffect(() => {
    let newSocket = new WebSocket("wss://srv617785.hstgr.cloud/solana_ws");

    // Establish connection handshake
    newSocket.onopen = function () {
      log("WebSocket connection established", "success");
    };

    newSocket.onmessage = function (event) {
      log(`Event received: ${event.data}`, "success");
      const data = JSON.parse(event.data);
      if (data.event_type === "copy_trade") {
        const handleCopyTrade = async () => {
          log("copy_trade event received", "success");
          try {
            let result:
              | {
                  success: boolean;
                  signature: string;
                  blockHash: string;
                }
              | undefined = await copyTrade(data.data);
            if (result && result.success) {
              log(
                `Confirmed tx, check:\n https://solscan.io/tx/${result.signature}`,
                "success"
              );
            } else {
              log(`Failed to send transaction`, "error");
            }
          } catch (error) {
            log(`copyTrade: ${error}`, "error");
          }
        };
        handleCopyTrade();
      }

      setSocket(newSocket);

      // Clean up the socket on component unmount
      return () => {
        newSocket.close();
        log("WebSocket connection closed", "info");
      };
    };
  }, []);

  // update the balance when userAccounts changes
  useEffect(() => {
    updateBalance();
  }, [userAccounts]);

  return (
    <div className="container mx-auto p-4">
      {isAuthenticated ? (
        <>
          <Card className="mb-4">
            <CardHeader>
              <CardTitle>
                Hi, {WebApp.initDataUnsafe.user?.username}
                {sessionActive && (
                  <button
                    className="text-red-500 text-xs font-thin ml-2 bg-slate-200 p-2 rounded-md hover:bg-slate-300 transition-colors duration-200 shadow-sm"
                    onClick={() => {
                      handleEndSession();
                    }}
                  >
                    End Session
                  </button>
                )}
              </CardTitle>
            </CardHeader>
            <CardContent className="overflow-hidden">
              {!isAuthenticated ? (
                <Spinner />
              ) : (
                <div className="flex flex-col justify-between w-full">
                  {createSessionButtonActive ? (
                    <Button
                      onClick={() => {
                        setCreateSessionButtonActive(false);
                      }}
                      disabled={isLoading || sessionActive}
                    >
                      {sessionActive ? (
                        `Session Active (${remainingTime})`
                      ) : isLoading ? (
                        <Spinner />
                      ) : (
                        "Create New Session"
                      )}
                    </Button>
                  ) : (
                    <div className="flex flex-col items-center w-full">
                      <span className="text-lg font-semibold mb-2 text-[#e7422cc7]">
                        Create session
                      </span>
                      <div className="flex flex-col justify-between w-full">
                        {!isLoading && (
                          <Button
                            className="mb-2"
                            onClick={() => {
                              setCreateSessionButtonActive(true);
                            }}
                            disabled={isLoading}
                          >
                            {"Back"}
                          </Button>
                        )}
                        <Input
                          placeholder="Duration of the session in minutes"
                          className="mb-2"
                          value={sessionDuration || ""}
                          onChange={(e) => {
                            const value = parseInt(e.target.value);
                            setSessionDuration(
                              isNaN(value) ? "" : value.toString()
                            );
                          }}
                          type="number"
                        />

                        <Button
                          className="mb-2"
                          disabled={isLoading}
                          onClick={() => {
                            handleCreateSession(sessionDuration);
                          }}
                        >
                          {isLoading ? <Spinner /> : "Confirm"}
                        </Button>
                      </div>
                    </div>
                  )}
                  <Button
                    className="mb-2 mt-2"
                    onClick={() => {
                      setTransferSolActive(true);
                    }}
                  >
                    Transfer SOL
                  </Button>
                  {transferSolActive && (
                    <>
                      <Button
                        variant={"ghost"}
                        className="mb-2"
                        onClick={() => {
                          setTransferSolActive(false);
                        }}
                      >
                        Back
                      </Button>
                      <Input
                        placeholder="Enter amount to transfer"
                        value={amount}
                        onChange={(e) => setAmount(e.target.value)}
                      />
                      <Button
                        className="mb-2 mt-2"
                        size="sm"
                        onClick={() => setAmount(solBalance)}
                      >
                        {isLoading ? <Spinner /> : "Max"}
                      </Button>
                      <Input
                        placeholder="Enter recipient address"
                        value={recipient}
                        onChange={(e) => setRecipient(e.target.value)}
                        className="mb-2 text-[9px]"
                      />
                      <Button
                        className="mb-2 mt-2"
                        onClick={() => {
                          navigator.clipboard
                            .readText()
                            .then((text) => {
                              setRecipient(text);
                            })
                            .catch((err) => {
                              console.error(
                                "Failed to read clipboard contents: ",
                                err
                              );
                            });
                        }}
                      >
                        {isLoading ? <Spinner /> : "Paste"}
                      </Button>

                      <Button
                        className="mb-2 mt-2"
                        onClick={() => {
                          handleTransferSol(amount, recipient);
                        }}
                      >
                        {isLoading ? <Spinner /> : "Send"}
                      </Button>
                    </>
                  )}
                  <Button onClick={() => setSwapSheetOpen(true)}>Swap</Button>
                  <SwapSheet
                    isOpen={swapSheetOpen}
                    onClose={() => setSwapSheetOpen(false)}
                  >
                    <h2 className="text-2xl font-bold text-center mb-4 bg-gradient-to-r from-purple-400 via-purple-600 to-purple-800 text-transparent bg-clip-text">
                      Buy/Sell Tokens
                    </h2>

                    <div className="flex flex-row items-center justify-between">
                      <Input
                        type="text"
                        placeholder="token ca"
                        onChange={(e) => {
                          setTokenCa(e.target.value);
                          setTokenData(null);
                        }}
                      />
                      <Button onClick={() => handleGetTokenData(tokenCa)}>
                        Get Token Data
                      </Button>
                    </div>
                    {tokenData && (
                      <>
                        <SwapInterface
                          tokenData={tokenData}
                          solBalance={solBalance}
                          address={userAccounts[0].address}
                          swapTokens={swapTokens}
                        />
                      </>
                    )}

                    <div className="flex flex-col items-center justify-center w-full mt-3">
                      <TokensBalancesSwap
                        address={userAccounts[0].address}
                        setTokenData={setTokenData}
                        setTokenCa={setTokenCa}
                      />
                    </div>
                  </SwapSheet>

                  <h3 className="text-2xl font-bold mb-4 bg-gradient-to-r from-purple-400 via-purple-600 to-purple-800 text-transparent bg-clip-text">
                    Wallets
                  </h3>
                  {userAccounts.map((account) => (
                    <div
                      key={account.walletId}
                      className="flex items-center justify-between mb-2 p-2 bg-gray-50 rounded"
                    >
                      <div className="flex flex-col items-center justify-between w-full">
                        <div className="flex items-center">
                          <span
                            className="mr-2 text-sm font-medium"
                            style={{
                              color:
                                account.addressFormat ===
                                "ADDRESS_FORMAT_SOLANA"
                                  ? "purple"
                                  : "inherit",
                            }}
                          >
                            {account.addressFormat ===
                            "ADDRESS_FORMAT_SOLANA" ? (
                              <img src={SolanaIcon} className="w-8 h-4" />
                            ) : (
                              "Unknown"
                            )}
                          </span>
                          <span className="text-sm text-[#ff4d35] mr-5">
                            {`${account.address.slice(
                              0,
                              3
                            )}...${account.address.slice(-3)}`}
                          </span>
                          <div className="flex flex-col items-center justify-center">
                            <span className="text-sm text-[#ff4d35]">
                              SOL {solBalance}
                            </span>
                            <span className="text-sm text-[#ff4d35]">
                              ${usdSolBalance}
                            </span>
                          </div>{" "}
                          <button
                            className="p-2 hover:bg-gray-100 rounded"
                            onClick={() => {
                              navigator.clipboard.writeText(account.address);
                              // Optionally, you can add a toast or alert here to confirm the copy action
                              alert("Address copied to clipboard!");
                            }}
                          >
                            <img src={CopyIcon} className="w-4 h-4" />
                          </button>
                        </div>
                        <div className="flex flex-col items-center justify-center">
                          <TokensBalances address={account.address} />
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
                onClick={() => {
                  handleSetCopyTrade(
                    WebApp.initDataUnsafe.user?.id.toString() ?? "",
                    walletId,
                    userAccounts[0].address,
                    amountToBuyCopyTrade,
                    walletToCopyTrade,
                    "active"
                  );
                }}
              >
                {isLoading ? <Spinner /> : "Create Copy Trade"}
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
                              walletId,
                              userAccounts[0].address,
                              copyTrade.buy_amount,
                              copyTrade.copy_trade_address,
                              copyTrade.status === "active"
                                ? "inactive"
                                : "active"
                            )
                          }
                          className={`px-4 py-2 rounded-full ${
                            copyTrade.status === "active"
                              ? "bg-[#ffbaba]/ hover:bg-red-600 text-purple-950 hover:text-white"
                              : "bg-green-500 hover:bg-green-600 text-white"
                          }`}
                        >
                          {copyTrade.status === "active"
                            ? "Cancel"
                            : "Activate"}
                        </Button>
                        <Button
                          className="bg-red-700 hover:bg-red-600 text-white rounded-full"
                          onClick={() => {
                            handleDeleteCopyTrade(copyTrade.copy_trade_address);
                          }}
                        >
                          Delete
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </CardContent>
          </Card>
        </>
      ) : (
        <Card>
          <CardHeader>
            <CardTitle>
              <p>Hi {WebApp.initDataUnsafe.user?.username}</p>
            </CardTitle>
          </CardHeader>
          <CardContent>
            {!isRegistered ? (
              <p>Please create a account to continue</p>
            ) : (
              <p>Please authenticate to continue</p>
            )}
            <Input
              placeholder="Email"
              className="mb-2"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
            />
            <Input
              placeholder="Create a password"
              className="mb-2"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
            {isRegistered ? (
              <Button onClick={() => handleLogin(password)}> Login </Button>
            ) : (
              <Button onClick={() => handleRegister()}> Register </Button>
            )}
          </CardContent>
        </Card>
      )}

      <Card className="mt-4">
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
