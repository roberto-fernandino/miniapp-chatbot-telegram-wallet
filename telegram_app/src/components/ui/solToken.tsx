import React, { useEffect, useState } from "react";
import { PublicKey, AccountInfo, ParsedAccountData } from "@solana/web3.js";
import {
  getTokenData,
  getTokenInfo,
  fetchDexcelerateScanner,
} from "../../lib/utils";
import TokenPrice from "./tokenPrice";
interface TokenInfo {
  pubkey: PublicKey;
  account: AccountInfo<ParsedAccountData>;
}

const SolToken: React.FC<{
  token: TokenInfo;
}> = ({ token }) => {
  const [tokenLogoUrl, setTokenLogoUrl] = useState<string | null>(null);
  const [scannerResponse, setScannerResponse] = useState<any>(null);

  useEffect(() => {
    getTokenInfo(token.account.data.parsed.info.mint).then((response) => {
      setTokenLogoUrl(response.data[0].logoURI);
    });
  }, [token]);
  useEffect(() => {
    fetchDexcelerateScanner(token.account.data.parsed.info.mint).then(
      (response) => {
        setScannerResponse(response);
      }
    );
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
              {scannerResponse?.pair.token1Name}
            </div>
            <div className="text-xs flex flex-col items-center justify-center">
              <span className="text-xs text-gray-500">
                {token.account.data.parsed.info.tokenAmount.uiAmount.toFixed(2)}
              </span>
              <TokenPrice
                ca={token.account.data.parsed.info.mint}
                amount={parseFloat(
                  token.account.data.parsed.info.tokenAmount.uiAmount
                )}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
};

export default SolToken;
