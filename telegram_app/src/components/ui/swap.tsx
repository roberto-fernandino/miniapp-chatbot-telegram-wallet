import React, { useState, useEffect } from "react";
import { Button } from "./button";
import { Input } from "./input";
import solLogo from "../../assets/sol.png";
import { getAllSolanaTokensBalance, getSOLPrice } from "../../lib/solana";
import { AccountInfo, ParsedAccountData, PublicKey } from "@solana/web3.js";
import { isValidSolanaAddress } from "../../lib/solana";
import { isValidEthereumAddress } from "../../lib/eth";

interface Token {
  symbol: string;
  name: string;
  logoURI: string;
  balance: string;
  address: string;
}
type Network = "solana" | "ethereum";

interface SolTokenInfo {
  pubkey: PublicKey;
  account: AccountInfo<ParsedAccountData>;
}

interface SwapInterfaceProps {
  tokenData: any;
  solBalance: string;
  ethBalance: string;
  address: string;
  swapSolanaTokens: (
    userPublicKey: string,
    toToken: Token,
    fromToken: Token,
    fromAmount: number,
    slippage: number
  ) => Promise<any>;
}

const SwapInterface: React.FC<SwapInterfaceProps> = ({
  tokenData,
  solBalance,
  address,
  swapSolanaTokens: swapSolanaTkoens,
}) => {
  const [fromToken, setFromToken] = useState<Token>({
    symbol: "SOL",
    name: "Solana",
    logoURI: solLogo,
    balance: solBalance,
    address: "So11111111111111111111111111111111111111112",
  });
  const [toToken, setToToken] = useState<Token | null>(null);
  const [fromAmount, setFromAmount] = useState("");
  const [toAmount, setToAmount] = useState("");
  const [tokenAmount, setTokenAmount] = useState("");
  const [toTokenPrice, setToTokenPrice] = useState("");
  const [slippage, setSlippage] = useState(0.5);
  const [debounceTimeout, setDebounceTimeout] = useState<NodeJS.Timeout | null>(
    null
  );
  const [network, setNetwork] = useState<Network | null>(null);

  useEffect(() => {
    if (tokenData) {
      setToToken({
        symbol: tokenData?.pair.token1Symbol,
        name: tokenData?.pair.token1Name,
        logoURI: tokenData?.pair.token1LogoURI ?? "",
        balance: tokenAmount,
        address: tokenData?.pair.token1Address,
      });
    }
  }, [tokenData, tokenAmount]);

  useEffect(() => {
    getAllSolanaTokensBalance(address).then((response) => {
      response.value.map((token: SolTokenInfo) => {
        setTokenAmount(token.account.data.parsed.info.tokenAmount.uiAmount);
      });
    });
  }, [address]);

  useEffect(() => {
    const prices = Object.values(tokenData.price.data as string);
    setToTokenPrice(prices[0]);
  }, [tokenData]);

  const handleSwapPositions = () => {
    if (toToken) {
      const temp = fromToken;
      setFromToken(toToken);
      setToToken(temp);
      setFromAmount(toAmount);
      setToAmount(fromAmount);
    }
  };

  const handleChangeAmount = (fromAmount: string) => {
    setFromAmount(fromAmount);

    if (debounceTimeout) {
      clearTimeout(debounceTimeout);
    }

    const timeoutId = setTimeout(async () => {
      if (fromToken.symbol === "SOL") {
        const solPrice = await getSOLPrice();
        const swapUsdAmount = parseFloat(fromAmount) * solPrice;
        setToAmount((swapUsdAmount / parseFloat(toTokenPrice)).toString());
      } else {
        const solPrice = await getSOLPrice();
        const swapUsdAmount = parseFloat(toAmount) * parseFloat(toTokenPrice);
        setToAmount((swapUsdAmount / solPrice).toString());
      }
    }, 1500);

    setDebounceTimeout(timeoutId);
  };

  const handleMaxAmount = () => {
    handleChangeAmount(fromToken.balance);
  };

  const checkAddressNetwork = (address: string): Network | null => {
    if (isValidSolanaAddress(address)) {
      return "solana";
    } else if (isValidEthereumAddress(address)) {
      return "ethereum";
    }
    return null;
  };
  const handleSwap = async (
    userPublicKey: string,
    toToken: Token,
    fromToken: Token,
    fromAmount: number,
    slippage: number
  ) => {
    const network = checkAddressNetwork(address);
    if (!network) {
      throw new Error("Invalid address for solana and ethereum.");
    }
    if (network === "solana") {
      await swapSolanaTkoens(
        userPublicKey,
        toToken,
        fromToken,
        fromAmount,
        slippage
      );
    }
  };
  return (
    <div className="p-4 bg-white rounded-lg shadow-md">
      <div className="mb-4">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center">
            <img
              src={fromToken.logoURI}
              alt={fromToken.name}
              className="w-8 h-8 rounded-full mr-2"
            />
            <span className="font-bold">{fromToken.symbol}</span>
          </div>
          <Button onClick={handleMaxAmount} size="sm">
            Max
          </Button>
        </div>
        <Input
          type="number"
          value={fromAmount}
          onChange={(e) => handleChangeAmount(e.target.value)}
          placeholder="0.00"
        />
        <div className="text-sm text-gray-500 mt-1">
          Balance: {fromToken.balance}
        </div>
      </div>

      <Button onClick={handleSwapPositions} className="w-full mb-4">
        ↑↓
      </Button>

      <div className="mb-4">
        {toToken ? (
          <>
            <div className="flex items-center mb-2">
              <img
                src={toToken.logoURI}
                alt={toToken.name}
                className="w-8 h-8 rounded-full mr-2"
              />
              <span className="font-bold">{toToken.symbol}</span>
            </div>
            <Input
              type="number"
              value={toAmount}
              onChange={(e) => setToAmount(e.target.value)}
              placeholder="0.00"
            />
            <div className="text-sm text-gray-500 mt-1">
              Balance: {toToken.balance}
            </div>
          </>
        ) : (
          <div className="text-center text-gray-500">
            Select a token to swap to
          </div>
        )}
      </div>
      <div className="mb-4">
        <label
          htmlFor="slippage"
          className="block text-sm font-medium text-gray-700 mb-1"
        >
          Slippage Tolerance
        </label>
        <div className="flex items-center space-x-2">
          <Input
            id="slippage"
            type="number"
            value={slippage}
            onChange={(e) => setSlippage(parseFloat(e.target.value))}
            placeholder="0.5"
            className="w-24"
          />
          <span className="text-gray-500">%</span>
          <Button onClick={() => setSlippage(0.1)} size="sm">
            0.1%
          </Button>
          <Button onClick={() => setSlippage(0.5)} size="sm">
            0.5%
          </Button>
          <Button onClick={() => setSlippage(1)} size="sm">
            1%
          </Button>
        </div>
      </div>
      <Button
        onClick={() => {
          if (toToken?.address && fromAmount) {
            handleSwap(
              address,
              toToken,
              fromToken,
              parseFloat(fromAmount),
              slippage
            );
          }
        }}
        className="w-full"
        disabled={!toToken || !fromAmount}
      >
        Swap
      </Button>
    </div>
  );
};

export default SwapInterface;
