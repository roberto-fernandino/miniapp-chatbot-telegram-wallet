import React from "react";
import TokenPrice from "./tokenPrice";

interface TokenInfo {
  token_address: string;
  name: string;
  symbol: string;
  logo?: string;
  thumbnail?: string;
  decimals: number;
  balance: string;
  possible_spam: boolean;
  verified_contract?: boolean;
}

const EthToken: React.FC<{ token: TokenInfo }> = ({ token }) => {
  return (
    <div key={token.token_address}>
      <div className="flex flex-row items-center justify-between">
        {parseFloat(token.balance) > 0 && (
          <>
            {token.logo && (
              <img
                src={token.logo}
                alt={token.name}
                className="w-8 h-8 rounded-full mr-8"
              />
            )}
            <div className="text-xs truncate w-24">{token.symbol}</div>
            <div className="text-xs flex flex-col items-center justify-center">
              <span className="text-xs text-gray-500">
                {parseFloat(token.balance).toFixed(2)}
              </span>
              <TokenPrice ca={token.token_address} />
            </div>
          </>
        )}
      </div>
    </div>
  );
};

export default EthToken;
