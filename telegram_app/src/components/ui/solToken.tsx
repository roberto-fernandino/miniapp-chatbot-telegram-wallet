import React, { useEffect, useState } from "react";
import { PublicKey, AccountInfo, ParsedAccountData } from "@solana/web3.js";
import { getTokenInfo } from "../../lib/utils";
import TokenPrice from "./tokenPrice";
interface TokenInfo {
  pubkey: PublicKey;
  account: AccountInfo<ParsedAccountData>;
}

const solToken: React.FC<{ token: TokenInfo }> = ({ token }) => {
  const [tokenLogoUrl, setTokenLogoUrl] = useState<string | null>(null);
  useEffect(() => {
    getTokenInfo(token.account.data.parsed.info.mint).then((response) => {
      setTokenLogoUrl(response.data[0].logoURI);
    });
  }, [token]);

  return (
    <div key={token.pubkey.toBase58()}>
      <div className="flex flex-row items-center justify-between">
        {token.account.data.parsed.info.tokenAmount.uiAmount > 0 && (
          <>
            {tokenLogoUrl && (
              <img
                src={tokenLogoUrl}
                alt={token.account.data.parsed.info.mint}
                className="w-8 h-8 rounded-full mr-8"
              />
            )}
            <div className="text-xs truncate w-24">
              {token.account.data.parsed.info.mint.slice(0, 4)}
              {"..."}
              {token.account.data.parsed.info.mint.slice(-4)}
            </div>
            <div className="text-xs flex flex-col items-center justify-center">
              <span className="text-xs text-gray-500">
                {token.account.data.parsed.info.tokenAmount.uiAmount.toFixed(2)}
              </span>
              <TokenPrice ca={token.account.data.parsed.info.mint} />
            </div>
          </>
        )}
      </div>
    </div>
  );
};

export default solToken;
